import {
  forceCenter,
  forceLink,
  forceManyBody,
  forceSimulation,
  type Simulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
} from 'd3-force';
import { useEffect, useMemo, useRef, useState } from 'react';

import type { VizGraph } from '@/lib/graph/adapter';

export interface LayoutNode extends SimulationNodeDatum {
  id: string;
  val: number;
  /** Z coordinate — d3-force is 2D, we add z ourselves for the 3D mode. */
  z?: number;
}

type LayoutLink = SimulationLinkDatum<LayoutNode>;

export interface LayoutConfig {
  mode: '2d' | '3d';
  /** Strength of node-node repulsion (d3 default is -30). Higher = more spread. */
  charge?: number;
  /** Scale factor for the z-axis in 3D mode. 0 = flat. */
  zSpread?: number;
  /** How many ticks before we freeze the simulation. */
  cooldownTicks?: number;
}

/**
 * Runs a d3-force simulation over the graph and returns a frozen set of
 * positions + a bumped version counter. The consumer re-renders when
 * `version` changes.
 *
 * 2D mode uses d3's planar layout as-is. 3D mode projects the planar layout
 * onto a thick shell by scattering z off each node's final position — good
 * enough for ≤2k nodes without building a custom 3D solver.
 */
export function useForceLayout(graph: VizGraph, config: LayoutConfig) {
  const { mode, charge = -240, zSpread = 60, cooldownTicks = 180 } = config;

  const simRef = useRef<Simulation<LayoutNode, LayoutLink> | null>(null);
  const positionsRef = useRef<Map<string, { x: number; y: number; z: number }>>(
    new Map()
  );
  const [version, setVersion] = useState(0);

  // Build the nodes/links arrays once per input identity.
  const { nodes, links } = useMemo(() => {
    const ns: LayoutNode[] = graph.nodes.map((n) => ({ id: n.id, val: n.val }));
    const ls: LayoutLink[] = graph.links.map((l) => ({
      source: l.source,
      target: l.target,
    }));
    return { nodes: ns, links: ls };
  }, [graph]);

  useEffect(() => {
    // Kill any previous simulation.
    simRef.current?.stop();

    if (nodes.length === 0) {
      positionsRef.current = new Map();
      setVersion((v) => v + 1);
      return;
    }

    const sim = forceSimulation<LayoutNode>(nodes)
      .force(
        'link',
        forceLink<LayoutNode, LayoutLink>(links)
          .id((d) => d.id)
          .distance(30)
          .strength(0.25)
      )
      .force('charge', forceManyBody().strength(charge))
      .force('center', forceCenter(0, 0))
      .alphaDecay(1 - Math.pow(0.001, 1 / cooldownTicks))
      .stop();

    // Run the sim synchronously to convergence — cheap for ≤2k nodes and
    // avoids the user watching it animate.
    const total = Math.ceil(
      Math.log(sim.alphaMin()) / Math.log(1 - sim.alphaDecay())
    );
    for (let i = 0; i < total; i++) sim.tick();

    // Assign z based on node id hash so the 3D mode has depth without a
    // full 3D solver. 2D mode leaves z = 0.
    const positions = new Map<string, { x: number; y: number; z: number }>();
    for (const n of nodes) {
      const z = mode === '3d' ? stableZ(n.id, zSpread) : 0;
      positions.set(n.id, { x: n.x ?? 0, y: n.y ?? 0, z });
    }
    positionsRef.current = positions;
    simRef.current = sim;
    setVersion((v) => v + 1);

    return () => {
      sim.stop();
    };
  }, [nodes, links, mode, charge, zSpread, cooldownTicks]);

  return { positions: positionsRef.current, version };
}

/** Deterministic pseudo-random z in [-spread, +spread] from a node id.
 *  Keeps 3D layout stable across re-renders. */
function stableZ(id: string, spread: number): number {
  let h = 2166136261;
  for (let i = 0; i < id.length; i++) {
    h ^= id.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  // Map the 32-bit hash to [-1, +1].
  const norm = ((h >>> 0) / 0xffffffff) * 2 - 1;
  return norm * spread;
}
