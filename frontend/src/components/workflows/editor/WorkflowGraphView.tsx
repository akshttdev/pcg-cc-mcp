import { useMemo } from 'react';
import { Network } from 'lucide-react';
import type { WorkflowNode, WorkflowConnection } from '@/lib/api';
import { getNodeTypeDef } from './node-types';

interface WorkflowGraphViewProps {
  nodes: WorkflowNode[];
  connections: WorkflowConnection[];
  onSelectNode?: (id: string) => void;
}

export function WorkflowGraphView({ nodes, connections, onSelectNode }: WorkflowGraphViewProps) {
  // Build layout: topological layers
  const layers = useMemo(() => {
    const deps: Record<string, string[]> = {};
    for (const c of connections) {
      if (!deps[c.target]) deps[c.target] = [];
      deps[c.target].push(c.source);
    }

    const nodeDepth: Record<string, number> = {};
    const getDepth = (id: string, visited: Set<string> = new Set()): number => {
      if (nodeDepth[id] !== undefined) return nodeDepth[id];
      if (visited.has(id)) return 0;
      visited.add(id);
      const nodeDeps = deps[id] ?? [];
      const depth = nodeDeps.length === 0 ? 0 : Math.max(...nodeDeps.map(d => getDepth(d, visited))) + 1;
      nodeDepth[id] = depth;
      return depth;
    };
    nodes.forEach(n => getDepth(n.id));

    const maxDepth = Math.max(0, ...Object.values(nodeDepth));
    const result: WorkflowNode[][] = [];
    for (let d = 0; d <= maxDepth; d++) {
      result.push(nodes.filter(n => (nodeDepth[n.id] ?? 0) === d));
    }
    return result;
  }, [nodes, connections]);

  if (nodes.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <Network className="h-8 w-8 mx-auto mb-2 opacity-30" />
        <p className="text-sm">No nodes to visualize</p>
      </div>
    );
  }

  const nodeWidth = 160;
  const nodeHeight = 56;
  const layerGapX = 220;
  const nodeGapY = 80;
  const padX = 40;
  const padY = 40;

  const totalWidth = padX * 2 + layers.length * layerGapX;
  const maxNodesInLayer = Math.max(1, ...layers.map(l => l.length));
  const totalHeight = padY * 2 + maxNodesInLayer * nodeGapY;

  // Compute positions
  const nodePositions: Record<string, { x: number; y: number }> = {};
  layers.forEach((layer, li) => {
    const layerHeight = layer.length * nodeGapY;
    const startY = (totalHeight - layerHeight) / 2;
    layer.forEach((node, ni) => {
      nodePositions[node.id] = {
        x: padX + li * layerGapX,
        y: startY + ni * nodeGapY,
      };
    });
  });

  return (
    <div className="overflow-auto">
      <svg width={totalWidth} height={totalHeight} className="mx-auto">
        <defs>
          <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
            <path d="M0,0 L8,3 L0,6" fill="currentColor" className="text-muted-foreground" />
          </marker>
        </defs>

        {/* Connection lines */}
        {connections.map((conn, i) => {
          const from = nodePositions[conn.source];
          const to = nodePositions[conn.target];
          if (!from || !to) return null;
          const x1 = from.x + nodeWidth;
          const y1 = from.y + nodeHeight / 2;
          const x2 = to.x;
          const y2 = to.y + nodeHeight / 2;
          const midX = (x1 + x2) / 2;
          return (
            <path
              key={i}
              d={`M${x1},${y1} C${midX},${y1} ${midX},${y2} ${x2},${y2}`}
              stroke="currentColor"
              className="text-muted-foreground/50"
              strokeWidth="2"
              fill="none"
              markerEnd="url(#arrowhead)"
            />
          );
        })}

        {/* Node boxes */}
        {nodes.map((node) => {
          const pos = nodePositions[node.id];
          if (!pos) return null;
          const typeDef = getNodeTypeDef(node.type);
          const colorMap: Record<string, string> = {
            'bg-blue-500': '#3b82f6', 'bg-purple-500': '#a855f7',
            'bg-emerald-500': '#10b981', 'bg-amber-500': '#f59e0b',
            'bg-orange-500': '#f97316', 'bg-teal-500': '#14b8a6',
            'bg-green-500': '#22c55e', 'bg-emerald-600': '#059669',
          };
          const fill = colorMap[typeDef?.color ?? ''] ?? '#6b7280';
          return (
            <g
              key={node.id}
              onClick={() => onSelectNode?.(node.id)}
              className={onSelectNode ? 'cursor-pointer' : undefined}
            >
              <rect
                x={pos.x} y={pos.y}
                width={nodeWidth} height={nodeHeight}
                rx="8" ry="8"
                fill="hsl(var(--card))"
                stroke={fill}
                strokeWidth="2"
              />
              {onSelectNode && (
                <rect
                  x={pos.x} y={pos.y}
                  width={nodeWidth} height={nodeHeight}
                  rx="8" ry="8"
                  fill="transparent"
                  className="hover:fill-primary/5"
                />
              )}
              <rect
                x={pos.x} y={pos.y}
                width="6" height={nodeHeight}
                rx="8" ry="0"
                fill={fill}
              />
              <title>{node.name} ({typeDef?.label ?? node.type})</title>
              <text
                x={pos.x + 16} y={pos.y + 22}
                fontSize="12" fontWeight="600"
                fill="currentColor" className="text-foreground"
              >
                {node.name.length > 16 ? node.name.slice(0, 15) + '...' : node.name}
              </text>
              <text
                x={pos.x + 16} y={pos.y + 40}
                fontSize="10"
                fill="currentColor" className="text-muted-foreground"
              >
                {typeDef?.label ?? node.type}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
  );
}
