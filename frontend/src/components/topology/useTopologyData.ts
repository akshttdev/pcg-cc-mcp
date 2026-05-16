import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type { EntitySubgraph, SyncGlobalStats } from 'shared/types';

import { topologyApi } from '@/lib/api';
import { topologyKeys } from '@/lib/query-keys';

export type TopologyScope =
  | { kind: 'global' }
  | { kind: 'org'; orgId: string }
  | { kind: 'focused'; focusType: string; focusId: string; depth?: number };

const STALE_MS = 60 * 1000; // 60s — topology is expensive; let sync drive refreshes.

/**
 * Fetch the entity-graph subgraph for the given scope. One hook; the scope
 * determines which endpoint it hits.
 *
 * - `global`  → GET /api/graph/global  (filter applied server-side)
 * - `org`     → GET /api/graph/org/:orgId
 * - `focused` → GET /api/graph/subgraph?focus_type=&focus_id=&depth=
 */
export function useTopologyData(scope: TopologyScope, enabled = true) {
  return useQuery<EntitySubgraph>({
    queryKey: scopeToKey(scope),
    queryFn: () => fetchForScope(scope),
    enabled,
    staleTime: STALE_MS,
    gcTime: 5 * STALE_MS,
  });
}

/** Trigger a global resync. Admin-only server-side; callers hide the
 *  affordance for non-admins. Invalidates all topology keys on success. */
export function useSyncGlobalTopology() {
  const qc = useQueryClient();
  return useMutation<SyncGlobalStats, Error, void>({
    mutationFn: () => topologyApi.syncGlobal(),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: topologyKeys.all });
    },
  });
}

function scopeToKey(scope: TopologyScope) {
  switch (scope.kind) {
    case 'global':
      return topologyKeys.global();
    case 'org':
      return topologyKeys.org(scope.orgId);
    case 'focused':
      return topologyKeys.focused(
        scope.focusType,
        scope.focusId,
        scope.depth ?? 2
      );
  }
}

function fetchForScope(scope: TopologyScope): Promise<EntitySubgraph> {
  switch (scope.kind) {
    case 'global':
      return topologyApi.getGlobal();
    case 'org':
      return topologyApi.getOrg(scope.orgId);
    case 'focused':
      return topologyApi.getFocused(
        scope.focusType,
        scope.focusId,
        scope.depth ?? 2
      );
  }
}
