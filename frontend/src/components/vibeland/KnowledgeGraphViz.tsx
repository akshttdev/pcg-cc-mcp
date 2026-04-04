import { useFrame, useThree } from '@react-three/fiber';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import * as THREE from 'three';

// ── Types ──────────────────────────────────────────────────────────────────────

interface GraphNode {
  id: string;
  node_type: string;
  label: string;
  metadata?: string | null;
}

interface GraphEdge {
  id: string;
  from_node_id: string;
  to_node_id: string;
  edge_type: string;
  weight?: number | null;
}

interface GraphData {
  nodes: GraphNode[];
  edges: GraphEdge[];
}

// ── Visual config per node type ────────────────────────────────────────────────

const NODE_COLOR: Record<string, THREE.Color> = {
  company: new THREE.Color('#ffd700'), // gold
  organization: new THREE.Color('#4499ff'), // electric blue
  person: new THREE.Color('#00ffee'), // cyan
  knowledge_source: new THREE.Color('#00ff88'), // matrix green
  proposal: new THREE.Color('#ff8844'), // amber
};
const DEFAULT_COLOR = new THREE.Color('#aaaaff');

const NODE_RADIUS: Record<string, number> = {
  company: 2.8,
  organization: 3.5,
  person: 1.6,
  knowledge_source: 0.9,
  proposal: 1.2,
};
const DEFAULT_RADIUS = 1.2;

// ── Force simulation constants ─────────────────────────────────────────────────

const SPREAD = 110; // initial random placement radius
const REPULSION = 900; // node-node repulsion strength
const ATTRACTION = 0.035; // edge spring strength
const DAMPING = 0.86; // velocity damping per tick
const CENTER_PULL = 0.004; // pull toward graph center
const PHYSICS_TICKS = 260; // frames before sim freezes
const TICK_EVERY = 2; // run physics every N frames

// ── Label sprite ──────────────────────────────────────────────────────────────

function makeLabel(text: string): THREE.Sprite {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d')!;
  const fontSize = 22;
  ctx.font = `bold ${fontSize}px monospace`;
  const w = Math.ceil(ctx.measureText(text).width) + 20;
  canvas.width = w;
  canvas.height = fontSize + 12;
  ctx.font = `bold ${fontSize}px monospace`;
  ctx.fillStyle = 'rgba(0,0,0,0.55)';
  ctx.fillRect(0, 0, canvas.width, canvas.height);
  ctx.fillStyle = '#ffffff';
  ctx.fillText(text, 10, fontSize);
  const tex = new THREE.CanvasTexture(canvas);
  const mat = new THREE.SpriteMaterial({
    map: tex,
    transparent: true,
    depthTest: false,
  });
  const sprite = new THREE.Sprite(mat);
  sprite.scale.set(w / 18, 2.2, 1);
  return sprite;
}

// ── Main component ─────────────────────────────────────────────────────────────

export function KnowledgeGraphViz({
  position = [-260, 55, -60] as [number, number, number],
}) {
  const [graph, setGraph] = useState<GraphData | null>(null);
  const [, setStatus] = useState<'idle' | 'loading' | 'ready' | 'empty'>(
    'idle'
  );
  const [hovered, setHovered] = useState<number | null>(null);

  const groupRef = useRef<THREE.Group>(null);
  const meshRef = useRef<THREE.InstancedMesh>(null);
  const linesRef = useRef<THREE.LineSegments>(null);
  const labelsRef = useRef<THREE.Group>(null);

  const posRef = useRef<Float32Array | null>(null); // x,y,z per node
  const velRef = useRef<Float32Array | null>(null); // vx,vy,vz per node
  const frameRef = useRef(0);
  const frozenRef = useRef(false);

  const dummy = useMemo(() => new THREE.Object3D(), []);
  const col = useMemo(() => new THREE.Color(), []);
  const raycaster = useMemo(() => new THREE.Raycaster(), []);
  const { camera, gl } = useThree();

  // ── Fetch & auto-sync ────────────────────────────────────────────────────────

  useEffect(() => {
    setStatus('loading');

    const load = async () => {
      // First try reading existing graph
      const res = await fetch('/api/graph/global').then((r) => r.json());
      const data: GraphData = res.data;

      if (!data || data.nodes.length < 10) {
        // Graph is empty — trigger a sync first
        await fetch('/api/graph/sync/global', { method: 'POST' });
        const res2 = await fetch('/api/graph/global').then((r) => r.json());
        const data2: GraphData = res2.data;
        if (!data2 || data2.nodes.length === 0) {
          setStatus('empty');
          return;
        }
        setGraph(data2);
      } else {
        setGraph(data);
      }
      setStatus('ready');
    };

    load().catch((e) => {
      console.error('[KG]', e);
      setStatus('empty');
    });
  }, []);

  // ── Initialize simulation positions ─────────────────────────────────────────

  useEffect(() => {
    if (!graph) return;
    const n = graph.nodes.length;
    const pos = new Float32Array(n * 3);
    const vel = new Float32Array(n * 3);

    // Spread nodes by type to help clustering converge faster
    const typeOffsets: Record<string, [number, number, number]> = {
      company: [0, 20, 0],
      organization: [30, 0, 30],
      person: [-20, -15, 0],
      knowledge_source: [0, -30, 20],
    };

    for (let i = 0; i < n; i++) {
      const type = graph.nodes[i].node_type;
      const [ox, oy, oz] = typeOffsets[type] ?? [0, 0, 0];
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(2 * Math.random() - 1);
      const r = 30 + Math.random() * SPREAD;
      pos[i * 3] = ox + r * Math.sin(phi) * Math.cos(theta);
      pos[i * 3 + 1] = oy + r * Math.cos(phi);
      pos[i * 3 + 2] = oz + r * Math.sin(phi) * Math.sin(theta);
    }

    posRef.current = pos;
    velRef.current = vel;
    frameRef.current = 0;
    frozenRef.current = false;
  }, [graph]);

  // ── Build lookup structures ──────────────────────────────────────────────────

  const nodeIndex = useMemo<Map<string, number>>(() => {
    if (!graph) return new Map();
    return new Map(graph.nodes.map((n, i) => [n.id, i]));
  }, [graph]);

  const edgePairs = useMemo<Int32Array>(() => {
    if (!graph || !nodeIndex.size) return new Int32Array(0);
    const valid = graph.edges.filter(
      (e) => nodeIndex.has(e.from_node_id) && nodeIndex.has(e.to_node_id)
    );
    const arr = new Int32Array(valid.length * 2);
    valid.forEach((e, i) => {
      arr[i * 2] = nodeIndex.get(e.from_node_id)!;
      arr[i * 2 + 1] = nodeIndex.get(e.to_node_id)!;
    });
    return arr;
  }, [graph, nodeIndex]);

  // Preallocate edge position buffer (updated each frame)
  const edgePosBuf = useMemo<Float32Array>(() => {
    const count = edgePairs.length / 2;
    return new Float32Array(count * 6); // 2 verts × 3 floats
  }, [edgePairs]);

  // ── Build label sprites once graph is ready ──────────────────────────────────

  useEffect(() => {
    const group = labelsRef.current;
    if (!group || !graph) return;

    // Clear old sprites
    while (group.children.length) group.remove(group.children[0]);

    // Only label companies and organizations (not every person/KS — too cluttered)
    graph.nodes.forEach((node, i) => {
      if (node.node_type !== 'company' && node.node_type !== 'organization')
        return;
      const sprite = makeLabel(node.label);
      sprite.userData.nodeIndex = i;
      group.add(sprite);
    });
  }, [graph]);

  // ── Animation loop ────────────────────────────────────────────────────────────

  useFrame((_, delta) => {
    if (!graph || !posRef.current || !velRef.current) return;

    const n = graph.nodes.length;
    const pos = posRef.current;
    const vel = velRef.current;
    const ec = edgePairs.length / 2;

    // Slow rotation of the entire group
    if (groupRef.current) {
      groupRef.current.rotation.y += delta * 0.04;
    }

    // Physics simulation (stops after PHYSICS_TICKS)
    if (!frozenRef.current) {
      frameRef.current++;
      if (frameRef.current >= PHYSICS_TICKS) {
        frozenRef.current = true;
      }

      if (frameRef.current % TICK_EVERY === 0) {
        // Dampen
        for (let i = 0; i < n * 3; i++) vel[i] *= DAMPING;

        // Repulsion (O(n²) — fine for ≤250 nodes)
        for (let i = 0; i < n; i++) {
          const ix = i * 3,
            iy = ix + 1,
            iz = ix + 2;
          for (let j = i + 1; j < n; j++) {
            const jx = j * 3,
              jy = jx + 1,
              jz = jx + 2;
            const dx = pos[jx] - pos[ix];
            const dy = pos[jy] - pos[iy];
            const dz = pos[jz] - pos[iz];
            const d2 = dx * dx + dy * dy + dz * dz + 0.01;
            const d = Math.sqrt(d2);
            const f = REPULSION / d2;
            const fx = (dx / d) * f,
              fy = (dy / d) * f,
              fz = (dz / d) * f;
            vel[ix] -= fx;
            vel[iy] -= fy;
            vel[iz] -= fz;
            vel[jx] += fx;
            vel[jy] += fy;
            vel[jz] += fz;
          }
          // Pull toward center
          vel[ix] -= pos[ix] * CENTER_PULL;
          vel[iy] -= pos[iy] * CENTER_PULL;
          vel[iz] -= pos[iz] * CENTER_PULL;
        }

        // Attraction along edges
        for (let e = 0; e < ec; e++) {
          const fi = edgePairs[e * 2],
            ti = edgePairs[e * 2 + 1];
          const dx = pos[ti * 3] - pos[fi * 3];
          const dy = pos[ti * 3 + 1] - pos[fi * 3 + 1];
          const dz = pos[ti * 3 + 2] - pos[fi * 3 + 2];
          vel[fi * 3] += dx * ATTRACTION;
          vel[fi * 3 + 1] += dy * ATTRACTION;
          vel[fi * 3 + 2] += dz * ATTRACTION;
          vel[ti * 3] -= dx * ATTRACTION;
          vel[ti * 3 + 1] -= dy * ATTRACTION;
          vel[ti * 3 + 2] -= dz * ATTRACTION;
        }

        // Integrate positions
        for (let i = 0; i < n; i++) {
          pos[i * 3] += vel[i * 3];
          pos[i * 3 + 1] += vel[i * 3 + 1];
          pos[i * 3 + 2] += vel[i * 3 + 2];
        }
      }
    }

    // ── Update InstancedMesh ─────────────────────────────────────────────────

    const mesh = meshRef.current;
    if (mesh) {
      for (let i = 0; i < n; i++) {
        const type = graph.nodes[i].node_type;
        const radius = NODE_RADIUS[type] ?? DEFAULT_RADIUS;
        const isHov = hovered === i;

        dummy.position.set(pos[i * 3], pos[i * 3 + 1], pos[i * 3 + 2]);
        dummy.scale.setScalar(isHov ? radius * 1.5 : radius);
        dummy.updateMatrix();
        mesh.setMatrixAt(i, dummy.matrix);

        col.copy(NODE_COLOR[type] ?? DEFAULT_COLOR);
        if (isHov) col.multiplyScalar(1.8);
        mesh.setColorAt(i, col);
      }
      mesh.instanceMatrix.needsUpdate = true;
      if (mesh.instanceColor) mesh.instanceColor.needsUpdate = true;
    }

    // ── Update edges ─────────────────────────────────────────────────────────

    const lines = linesRef.current;
    if (lines && ec > 0) {
      const attr = lines.geometry.attributes.position as THREE.BufferAttribute;
      for (let e = 0; e < ec; e++) {
        const fi = edgePairs[e * 2],
          ti = edgePairs[e * 2 + 1];
        attr.setXYZ(e * 2, pos[fi * 3], pos[fi * 3 + 1], pos[fi * 3 + 2]);
        attr.setXYZ(e * 2 + 1, pos[ti * 3], pos[ti * 3 + 1], pos[ti * 3 + 2]);
      }
      attr.needsUpdate = true;
    }

    // ── Update label positions ────────────────────────────────────────────────

    const labels = labelsRef.current;
    if (labels) {
      labels.children.forEach((sprite) => {
        const idx: number = sprite.userData.nodeIndex;
        if (idx !== undefined && pos[idx * 3] !== undefined) {
          const r = NODE_RADIUS[graph.nodes[idx]?.node_type] ?? DEFAULT_RADIUS;
          sprite.position.set(
            pos[idx * 3],
            pos[idx * 3 + 1] + r + 2.5,
            pos[idx * 3 + 2]
          );
        }
      });
    }
  });

  // ── Click / hover ──────────────────────────────────────────────────────────

  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (!meshRef.current || !graph) return;
      const rect = gl.domElement.getBoundingClientRect();
      const x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
      const y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
      raycaster.setFromCamera({ x, y }, camera);
      const hits = raycaster.intersectObject(meshRef.current);
      setHovered(hits.length > 0 ? (hits[0].instanceId ?? null) : null);
    },
    [camera, gl, graph, raycaster]
  );

  // ── Render ────────────────────────────────────────────────────────────────

  if (!graph || graph.nodes.length === 0) return null;

  const nodeCount = graph.nodes.length;
  const edgeCount = edgePairs.length / 2;

  return (
    <group ref={groupRef} position={position} onPointerMove={onPointerMove}>
      {/* Ambient glow at graph center */}
      <pointLight color="#002244" intensity={6} distance={250} />
      <pointLight color="#004422" intensity={3} distance={180} />

      {/* ── Node spheres (instanced) ── */}
      <instancedMesh
        ref={meshRef}
        args={[undefined, undefined, nodeCount]}
        frustumCulled={false}
      >
        <sphereGeometry args={[1, 14, 14]} />
        <meshStandardMaterial
          vertexColors
          roughness={0.15}
          metalness={0.75}
          emissive={new THREE.Color('#111111')}
          emissiveIntensity={0.3}
        />
      </instancedMesh>

      {/* ── Edges ── */}
      {edgeCount > 0 && (
        <lineSegments ref={linesRef} frustumCulled={false}>
          <bufferGeometry>
            <bufferAttribute
              attach="attributes-position"
              count={edgeCount * 2}
              array={edgePosBuf}
              itemSize={3}
            />
          </bufferGeometry>
          <lineBasicMaterial
            color="#1a3a5a"
            opacity={0.5}
            transparent
            blending={THREE.AdditiveBlending}
            depthWrite={false}
          />
        </lineSegments>
      )}

      {/* ── Labels (company + org only) ── */}
      <group ref={labelsRef} />

      {/* ── Legend billboard (always faces camera) ── */}
      <LegendBillboard
        position={[-80, 60, 0]}
        nodeCount={nodeCount}
        edgeCount={edgeCount}
      />
    </group>
  );
}

// ── Tiny legend that floats near the graph ─────────────────────────────────────

function LegendBillboard({
  position,
  nodeCount,
  edgeCount,
}: {
  position: [number, number, number];
  nodeCount: number;
  edgeCount: number;
}) {
  const spriteRef = useRef<THREE.Sprite>(null);
  const texRef = useRef<THREE.CanvasTexture | null>(null);

  useEffect(() => {
    const canvas = document.createElement('canvas');
    canvas.width = 320;
    canvas.height = 180;
    const ctx = canvas.getContext('2d')!;

    ctx.fillStyle = 'rgba(0,5,15,0.85)';
    ctx.roundRect(0, 0, 320, 180, 10);
    ctx.fill();

    ctx.strokeStyle = '#003366';
    ctx.lineWidth = 1.5;
    ctx.roundRect(0, 0, 320, 180, 10);
    ctx.stroke();

    ctx.font = 'bold 15px monospace';
    ctx.fillStyle = '#00ccff';
    ctx.fillText('TOPOS KNOWLEDGE GRAPH', 14, 22);

    ctx.font = '12px monospace';
    ctx.fillStyle = '#556677';
    ctx.fillText(`${nodeCount} nodes  ·  ${edgeCount} edges`, 14, 42);

    const legend = [
      { color: '#ffd700', label: 'Company' },
      { color: '#4499ff', label: 'Organization' },
      { color: '#00ffee', label: 'Person' },
      { color: '#00ff88', label: 'Knowledge Source' },
    ];
    legend.forEach(({ color, label }, i) => {
      const y = 68 + i * 26;
      ctx.fillStyle = color;
      ctx.beginPath();
      ctx.arc(22, y, 7, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#aabbcc';
      ctx.font = '12px monospace';
      ctx.fillText(label, 36, y + 4);
    });

    const tex = new THREE.CanvasTexture(canvas);
    texRef.current = tex;
    if (spriteRef.current) {
      (spriteRef.current.material as THREE.SpriteMaterial).map = tex;
      (spriteRef.current.material as THREE.SpriteMaterial).needsUpdate = true;
    }
    return () => tex.dispose();
  }, [nodeCount, edgeCount]);

  return (
    <sprite ref={spriteRef} position={position} scale={[32, 18, 1]}>
      <spriteMaterial transparent depthTest={false} />
    </sprite>
  );
}
