import type {
  EntityGraphEdge,
  EntityGraphNode,
  EntitySubgraph,
} from 'shared/types';

/// The shape the canvas consumes. Keeps the original backend fields and
/// adds `source`/`target` (d3-force convention) plus a rendering `val`.
export interface VizNode extends EntityGraphNode {
  /** Rendering radius derived from node_type. */
  val: number;
  /** Hex color derived from node_type. */
  color: string;
}

export interface VizLink {
  id: string;
  source: string;
  target: string;
  edge_type: string;
  weight: number | null;
}

export interface VizGraph {
  nodes: VizNode[];
  links: VizLink[];
}

/** Per-node-type color palette. Matches the original KnowledgeGraphViz
 *  palette where it overlapped; extends with new types from Phase 1. */
export const NODE_COLOR: Record<string, string> = {
  organization: '#4499ff', // electric blue
  company: '#ffd700', // gold
  client: '#ffaa33', // warm amber
  person: '#00ffee', // cyan
  deal: '#ff6b6b', // coral
  proposal: '#ff8844', // amber
  project: '#a78bfa', // violet
  deliverable: '#d8b4fe', // pale violet
  task: '#fbbf24', // yellow
  task_attempt: '#f59e0b', // orange
  pipeline: '#14b8a6', // teal
  pipeline_stage: '#5eead4', // pale teal
  knowledge_source: '#00ff88', // matrix green
  research_pass: '#84cc16', // lime
  brand_profile: '#f472b6', // pink
  deck: '#c084fc', // lavender
  activity: '#94a3b8', // slate
  agent: '#38bdf8', // sky
};
const DEFAULT_COLOR = '#aaaaff';

/** Per-node-type radius used by the force sim + sphere geometry. */
export const NODE_RADIUS: Record<string, number> = {
  organization: 3.5,
  company: 2.8,
  client: 2.4,
  person: 1.6,
  deal: 2.0,
  proposal: 1.8,
  project: 2.2,
  deliverable: 1.4,
  task: 1.2,
  task_attempt: 1.0,
  pipeline: 2.0,
  pipeline_stage: 1.2,
  knowledge_source: 1.0,
  research_pass: 1.0,
  brand_profile: 1.4,
  deck: 1.6,
  activity: 0.8,
  agent: 2.0,
};
const DEFAULT_RADIUS = 1.2;

export function colorFor(nodeType: string): string {
  return NODE_COLOR[nodeType] ?? DEFAULT_COLOR;
}

export function radiusFor(nodeType: string): number {
  return NODE_RADIUS[nodeType] ?? DEFAULT_RADIUS;
}

/** Convert the backend `EntitySubgraph` to the canvas's `VizGraph`. */
export function toGraphViz(sub: EntitySubgraph): VizGraph {
  const nodes: VizNode[] = sub.nodes.map((n) => ({
    ...n,
    val: radiusFor(n.node_type),
    color: colorFor(n.node_type),
  }));
  const links: VizLink[] = sub.edges.map((e) => ({
    id: e.id,
    source: e.from_node_id,
    target: e.to_node_id,
    edge_type: e.edge_type,
    weight: e.weight,
  }));
  return { nodes, links };
}

/** Build an adjacency map: node_id → set of neighbor node_ids.
 *  Used by the canvas for hover-highlight neighborhoods. */
export function adjacencyMap(
  edges: EntityGraphEdge[]
): Map<string, Set<string>> {
  const map = new Map<string, Set<string>>();
  const add = (from: string, to: string) => {
    const set = map.get(from) ?? new Set<string>();
    set.add(to);
    map.set(from, set);
  };
  for (const e of edges) {
    add(e.from_node_id, e.to_node_id);
    add(e.to_node_id, e.from_node_id);
  }
  return map;
}
