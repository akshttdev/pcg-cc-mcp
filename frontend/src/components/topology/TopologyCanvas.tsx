import { OrbitControls } from '@react-three/drei';
import { Canvas, useThree } from '@react-three/fiber';
import { useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';

import type { VizGraph, VizNode } from '@/lib/graph/adapter';

import { useForceLayout } from './useForceLayout';

export interface TopologyCanvasProps {
  graph: VizGraph;
  mode: '2d' | '3d';
  onNodeClick?: (node: VizNode) => void;
  onNodeHover?: (node: VizNode | null) => void;
  highlightId?: string | null;
  className?: string;
}

export function TopologyCanvas({
  graph,
  mode,
  onNodeClick,
  onNodeHover,
  highlightId = null,
  className,
}: TopologyCanvasProps) {
  const { positions, version } = useForceLayout(graph, { mode });

  return (
    <div
      className={className ?? 'relative h-full w-full'}
      data-testid="topology-canvas"
    >
      <Canvas
        camera={{ position: [0, 0, 400], fov: 50, near: 1, far: 4000 }}
        gl={{ antialias: true, alpha: true }}
      >
        <color attach="background" args={['#0b1020']} />
        <ambientLight intensity={0.8} />
        <pointLight position={[200, 200, 200]} intensity={0.5} />

        <CameraRig positions={positions} version={version} mode={mode} />

        <Scene
          graph={graph}
          positions={positions}
          version={version}
          highlightId={highlightId}
          onNodeClick={onNodeClick}
          onNodeHover={onNodeHover}
        />
      </Canvas>

      {graph.nodes.length === 0 && (
        <div className="absolute inset-0 flex items-center justify-center text-sm text-slate-400">
          Empty graph — run sync or broaden filters.
        </div>
      )}
    </div>
  );
}

/**
 * Auto-fit the camera to the graph bounds whenever the layout changes,
 * and configure OrbitControls for the current mode (left-click pan in 2D;
 * full orbit in 3D). Also exposes wheel-zoom + drag-pan universally.
 */
function CameraRig({
  positions,
  version,
  mode,
}: {
  positions: Map<string, { x: number; y: number; z: number }>;
  version: number;
  mode: '2d' | '3d';
}) {
  const { camera, size } = useThree();
  const controlsRef = useRef<React.ComponentRef<typeof OrbitControls> | null>(
    null
  );

  // Compute a good camera distance from the graph's bounding sphere.
  useEffect(() => {
    if (positions.size === 0) return;

    let minX = Infinity,
      maxX = -Infinity,
      minY = Infinity,
      maxY = -Infinity;
    for (const p of positions.values()) {
      if (p.x < minX) minX = p.x;
      if (p.x > maxX) maxX = p.x;
      if (p.y < minY) minY = p.y;
      if (p.y > maxY) maxY = p.y;
    }
    const cx = (minX + maxX) / 2;
    const cy = (minY + maxY) / 2;
    const radius = Math.max(maxX - minX, maxY - minY) / 2 || 60;

    // Derive distance from the perspective camera's FOV so the graph fills
    // ~75% of the shorter viewport dimension.
    const perspective = camera as THREE.PerspectiveCamera;
    const fovRad = (perspective.fov * Math.PI) / 180;
    const aspect = size.width / size.height;
    const halfH = radius * 1.35;
    const halfW = (radius * 1.35) / Math.min(1, aspect);
    const distForHeight = halfH / Math.tan(fovRad / 2);
    const distForWidth = halfW / (Math.tan(fovRad / 2) * aspect);
    const dist = Math.max(distForHeight, distForWidth, 80);

    perspective.position.set(cx, cy, dist);
    perspective.lookAt(cx, cy, 0);
    perspective.updateProjectionMatrix();

    // Point OrbitControls' pivot at the graph centre.
    const c = controlsRef.current as unknown as {
      target: THREE.Vector3;
      update: () => void;
    } | null;
    if (c) {
      c.target.set(cx, cy, 0);
      c.update();
    }
  }, [positions, version, camera, size.width, size.height]);

  // Switch mouse behaviour per mode: 2D feels right with left=pan, wheel=zoom,
  // rotate off. 3D keeps the classic orbit.
  const mouseButtons = useMemo(() => {
    return mode === '2d'
      ? {
          LEFT: THREE.MOUSE.PAN,
          MIDDLE: THREE.MOUSE.DOLLY,
          RIGHT: THREE.MOUSE.ROTATE,
        }
      : {
          LEFT: THREE.MOUSE.ROTATE,
          MIDDLE: THREE.MOUSE.DOLLY,
          RIGHT: THREE.MOUSE.PAN,
        };
  }, [mode]);

  return (
    <OrbitControls
      ref={controlsRef}
      enablePan
      enableZoom
      enableRotate={mode === '3d'}
      makeDefault
      screenSpacePanning
      zoomSpeed={1.1}
      panSpeed={1.2}
      mouseButtons={mouseButtons}
    />
  );
}

interface SceneProps {
  graph: VizGraph;
  positions: Map<string, { x: number; y: number; z: number }>;
  version: number;
  highlightId: string | null;
  onNodeClick?: (node: VizNode) => void;
  onNodeHover?: (node: VizNode | null) => void;
}

function Scene({
  graph,
  positions,
  version,
  highlightId,
  onNodeClick,
  onNodeHover,
}: SceneProps) {
  const { gl } = useThree();
  const [hoveredId, setHoveredId] = useState<string | null>(null);

  useMemo(() => {
    gl.domElement.style.cursor = hoveredId ? 'pointer' : 'grab';
  }, [hoveredId, gl]);

  const neighborhood = useMemo<Set<string> | null>(() => {
    const focus = hoveredId ?? highlightId;
    if (!focus) return null;
    const set = new Set<string>([focus]);
    for (const l of graph.links) {
      const s =
        typeof l.source === 'string'
          ? l.source
          : (l.source as { id: string }).id;
      const t =
        typeof l.target === 'string'
          ? l.target
          : (l.target as { id: string }).id;
      if (s === focus) set.add(t);
      if (t === focus) set.add(s);
    }
    return set;
  }, [hoveredId, highlightId, graph.links]);

  const lineGeometry = useMemo(() => {
    const vertices: number[] = [];
    const colors: number[] = [];
    for (const l of graph.links) {
      const s =
        typeof l.source === 'string'
          ? l.source
          : (l.source as { id: string }).id;
      const t =
        typeof l.target === 'string'
          ? l.target
          : (l.target as { id: string }).id;
      const p1 = positions.get(s);
      const p2 = positions.get(t);
      if (!p1 || !p2) continue;
      vertices.push(p1.x, p1.y, p1.z, p2.x, p2.y, p2.z);

      const dim = neighborhood && !(neighborhood.has(s) && neighborhood.has(t));
      const alpha = dim ? 0.08 : 0.4;
      colors.push(1, 1, 1, alpha, 1, 1, 1, alpha);
    }
    const geom = new THREE.BufferGeometry();
    geom.setAttribute(
      'position',
      new THREE.Float32BufferAttribute(vertices, 3)
    );
    geom.setAttribute('color', new THREE.Float32BufferAttribute(colors, 4));
    return geom;
    // `positions` is a ref-stable Map mutated in place by `useForceLayout`;
    // `version` is the change-signal that fires after each sim pass.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graph.links, neighborhood, version]);

  return (
    <>
      <lineSegments>
        <primitive object={lineGeometry} attach="geometry" />
        <lineBasicMaterial vertexColors transparent />
      </lineSegments>

      {graph.nodes.map((n) => {
        const p = positions.get(n.id);
        if (!p) return null;
        const dim = neighborhood && !neighborhood.has(n.id);
        return (
          <mesh
            key={n.id}
            position={[p.x, p.y, p.z]}
            onClick={(e) => {
              e.stopPropagation();
              onNodeClick?.(n);
            }}
            onPointerOver={(e) => {
              e.stopPropagation();
              setHoveredId(n.id);
              onNodeHover?.(n);
            }}
            onPointerOut={() => {
              setHoveredId((prev) => (prev === n.id ? null : prev));
              onNodeHover?.(null);
            }}
          >
            <sphereGeometry args={[n.val, 16, 16]} />
            <meshStandardMaterial
              color={n.color}
              emissive={n.color}
              emissiveIntensity={dim ? 0.05 : 0.25}
              transparent
              opacity={dim ? 0.25 : 1}
            />
          </mesh>
        );
      })}
    </>
  );
}
