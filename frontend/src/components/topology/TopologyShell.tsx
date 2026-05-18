import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { Skeleton } from '@/components/ui/skeleton';
import { toGraphViz, type VizNode } from '@/lib/graph/adapter';

import { NodePreview, openProfileHref } from './previews';
import { FilterPane, type FilterState } from './toolbar/FilterPane';
import { ScopeBar } from './toolbar/ScopeBar';
import { ViewToggle } from './toolbar/ViewToggle';
import { TopologyCanvas } from './TopologyCanvas';
import {
  type TopologyScope,
  useSyncGlobalTopology,
  useTopologyData,
} from './useTopologyData';

export interface TopologyShellProps {
  scope: TopologyScope;
  /** Org id used when resolving "Open profile" routes for nodes that live
   *  inside an org's URL namespace (companies, clients, deals…). */
  orgId?: string;
  /** Admin sessions only — controls whether the Sync button renders. */
  canSync?: boolean;
  /** Optional class on the outer wrapper. Defaults to full-bleed. */
  className?: string;
}

/**
 * Composed topology surface: toolbar + filter pane + canvas + preview card.
 *
 * This is the component Phase 3 mounts inside the Intelligence › Topology
 * tab and Phase 7 reuses inside the `/interface` shell.
 */
export function TopologyShell({
  scope,
  orgId,
  canSync = false,
  className,
}: TopologyShellProps) {
  const { data, isLoading, error } = useTopologyData(scope);
  const sync = useSyncGlobalTopology();
  const navigate = useNavigate();

  const [mode, setMode] = useState<'2d' | '3d'>('3d');
  const [selected, setSelected] = useState<VizNode | null>(null);
  const [filter, setFilter] = useState<FilterState>(() => ({
    nodeTypes: new Set(),
    edgeTypes: new Set(),
    query: '',
  }));

  // Discover the available node/edge types from the current payload; default
  // every type to visible the first time we see them.
  const available = useMemo(() => {
    const ns = new Set<string>();
    const es = new Set<string>();
    for (const n of data?.nodes ?? []) ns.add(n.node_type);
    for (const e of data?.edges ?? []) es.add(e.edge_type);
    return {
      nodeTypes: Array.from(ns).sort(),
      edgeTypes: Array.from(es).sort(),
    };
  }, [data]);

  // Seed the filter's node/edge-type sets on first data load so everything
  // is visible by default (checkboxes ON).
  const seeded = useMemo(() => {
    if (filter.nodeTypes.size > 0 || filter.edgeTypes.size > 0) return filter;
    return {
      nodeTypes: new Set(available.nodeTypes),
      edgeTypes: new Set(available.edgeTypes),
      query: filter.query,
    };
  }, [filter, available]);

  // Apply filters client-side — keeps the view snappy.
  const viz = useMemo(() => {
    if (!data) return { nodes: [], links: [] };
    const q = seeded.query.trim().toLowerCase();
    const nodes = data.nodes.filter(
      (n) =>
        seeded.nodeTypes.has(n.node_type) &&
        (q === '' || n.label.toLowerCase().includes(q))
    );
    const nodeIds = new Set(nodes.map((n) => n.id));
    const edges = data.edges.filter(
      (e) =>
        seeded.edgeTypes.has(e.edge_type) &&
        nodeIds.has(e.from_node_id) &&
        nodeIds.has(e.to_node_id)
    );
    return toGraphViz({ nodes, edges });
  }, [data, seeded]);

  const handleOpenProfile = (node: VizNode) => {
    const href = openProfileHref(node, orgId);
    if (href) navigate(href);
  };

  return (
    <div
      className={className ?? 'flex h-full w-full flex-col'}
      data-testid="topology-shell"
    >
      <div className="flex items-center justify-between gap-3 border-b border-slate-800 bg-slate-900/70 px-3 py-1.5">
        <div className="flex-1">
          <ScopeBar
            scope={scope}
            onSyncClick={() => sync.mutate()}
            isSyncing={sync.isPending}
            canSync={canSync}
            stats={
              data
                ? { nodes: data.nodes.length, edges: data.edges.length }
                : undefined
            }
          />
        </div>
        <ViewToggle mode={mode} onChange={setMode} />
      </div>

      <div className="flex min-h-0 flex-1">
        <FilterPane available={available} state={seeded} onChange={setFilter} />

        <div className="relative flex-1">
          {isLoading && (
            <div className="absolute inset-0 flex items-center justify-center">
              <Skeleton className="h-64 w-64 rounded-full" />
            </div>
          )}
          {error && (
            <div className="absolute inset-0 flex items-center justify-center text-sm text-red-400">
              Failed to load topology: {error.message}
            </div>
          )}
          {data && (
            <TopologyCanvas
              graph={viz}
              mode={mode}
              onNodeClick={setSelected}
              highlightId={selected?.id ?? null}
            />
          )}

          {selected && (
            <div className="absolute right-3 top-3 z-10">
              <NodePreview
                node={selected}
                onClose={() => setSelected(null)}
                onOpenProfile={
                  openProfileHref(selected, orgId)
                    ? handleOpenProfile
                    : undefined
                }
              />
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
