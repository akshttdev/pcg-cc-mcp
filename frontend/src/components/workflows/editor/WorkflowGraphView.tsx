import { useEffect, useMemo, useState } from 'react';
import { Network, ZoomIn, ZoomOut, Maximize2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import type { WorkflowNode, WorkflowConnection } from '@/lib/api';
import { getNodeTypeDef } from './node-types';

interface WorkflowGraphViewProps {
  nodes: WorkflowNode[];
  connections: WorkflowConnection[];
  selectedNodeId?: string | null;
  onSelectNode?: (id: string) => void;
}

export function WorkflowGraphView({ nodes, connections, selectedNodeId, onSelectNode }: WorkflowGraphViewProps) {
  const [zoom, setZoom] = useState(1);

  // Warn about connections referencing non-existent nodes
  useEffect(() => {
    const nodeIds = new Set(nodes.map((n) => n.id));
    for (const c of connections) {
      if (!nodeIds.has(c.source)) {
        console.warn(`[WorkflowGraphView] Connection references non-existent source node: ${c.source}`);
      }
      if (!nodeIds.has(c.target)) {
        console.warn(`[WorkflowGraphView] Connection references non-existent target node: ${c.target}`);
      }
    }
  }, [nodes, connections]);

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

  // Connection count per node
  const connectionCounts = useMemo(() => {
    const counts: Record<string, number> = {};
    for (const c of connections) {
      counts[c.source] = (counts[c.source] || 0) + 1;
      counts[c.target] = (counts[c.target] || 0) + 1;
    }
    return counts;
  }, [connections]);

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

  const handleZoomIn = () => setZoom((z) => Math.min(z + 0.2, 2));
  const handleZoomOut = () => setZoom((z) => Math.max(z - 0.2, 0.4));
  const handleFit = () => setZoom(1);

  return (
    <div className="relative">
      {/* Zoom controls */}
      <div className="absolute top-2 right-2 z-10 flex items-center gap-1">
        <Button size="sm" variant="outline" className="h-7 w-7 p-0" onClick={handleZoomOut} title="Zoom out">
          <ZoomOut className="h-3.5 w-3.5" />
        </Button>
        <span className="text-xs text-muted-foreground w-8 text-center">{Math.round(zoom * 100)}%</span>
        <Button size="sm" variant="outline" className="h-7 w-7 p-0" onClick={handleZoomIn} title="Zoom in">
          <ZoomIn className="h-3.5 w-3.5" />
        </Button>
        <Button size="sm" variant="outline" className="h-7 w-7 p-0" onClick={handleFit} title="Fit to view">
          <Maximize2 className="h-3.5 w-3.5" />
        </Button>
      </div>

      <div className="overflow-auto">
        <svg
          width={totalWidth * zoom}
          height={totalHeight * zoom}
          className="mx-auto"
          viewBox={`0 0 ${totalWidth} ${totalHeight}`}
        >
          <defs>
            <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
              <path d="M0,0 L8,3 L0,6" fill="currentColor" className="text-muted-foreground" />
            </marker>
            <marker id="arrowhead-selected" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
              <path d="M0,0 L8,3 L0,6" fill="hsl(var(--primary))" />
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
            const isHighlighted = selectedNodeId === conn.source || selectedNodeId === conn.target;
            return (
              <path
                key={i}
                d={`M${x1},${y1} C${midX},${y1} ${midX},${y2} ${x2},${y2}`}
                stroke={isHighlighted ? 'hsl(var(--primary))' : 'currentColor'}
                className={isHighlighted ? '' : 'text-muted-foreground/50'}
                strokeWidth={isHighlighted ? 3 : 2}
                fill="none"
                markerEnd={isHighlighted ? 'url(#arrowhead-selected)' : 'url(#arrowhead)'}
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
              'bg-slate-600': '#475569', 'bg-yellow-600': '#ca8a04',
              'bg-pink-500': '#ec4899', 'bg-indigo-600': '#4f46e5',
              'bg-sky-500': '#0ea5e9', 'bg-cyan-600': '#0891b2',
            };
            const fill = colorMap[typeDef?.color ?? ''] ?? '#6b7280';
            const isSelected = selectedNodeId === node.id;
            const connCount = connectionCounts[node.id] || 0;
            return (
              <g
                key={node.id}
                onClick={() => onSelectNode?.(node.id)}
                className={onSelectNode ? 'cursor-pointer' : undefined}
              >
                {/* Selected glow */}
                {isSelected && (
                  <rect
                    x={pos.x - 3} y={pos.y - 3}
                    width={nodeWidth + 6} height={nodeHeight + 6}
                    rx="10" ry="10"
                    fill="none"
                    stroke="hsl(var(--primary))"
                    strokeWidth="2"
                    opacity="0.5"
                  />
                )}
                <rect
                  x={pos.x} y={pos.y}
                  width={nodeWidth} height={nodeHeight}
                  rx="8" ry="8"
                  fill="hsl(var(--card))"
                  stroke={isSelected ? 'hsl(var(--primary))' : fill}
                  strokeWidth={isSelected ? 3 : 2}
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
                <title>{node.name} ({typeDef?.label ?? node.type}) — {connCount} connection{connCount !== 1 ? 's' : ''}</title>
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
    </div>
  );
}
