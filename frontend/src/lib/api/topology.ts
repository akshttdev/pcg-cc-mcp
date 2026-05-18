import type { EntitySubgraph, SyncGlobalStats } from 'shared/types';

import { handleApiResponse, makeRequest } from './client';

export interface ActionHandler {
  action: string;
  route: string;
  method: string;
  description: string;
  implemented: boolean;
}

export interface ActionsResponse {
  actions: ActionHandler[];
}

export const topologyApi = {
  getGlobal: (): Promise<EntitySubgraph> =>
    makeRequest('/api/graph/global').then(handleApiResponse<EntitySubgraph>),

  getOrg: (orgId: string): Promise<EntitySubgraph> =>
    makeRequest(`/api/graph/org/${orgId}`).then(
      handleApiResponse<EntitySubgraph>
    ),

  getFocused: (
    focusType: string,
    focusId: string,
    depth = 2
  ): Promise<EntitySubgraph> => {
    const params = new URLSearchParams({
      focus_type: focusType,
      focus_id: focusId,
      depth: String(depth),
    });
    return makeRequest(`/api/graph/subgraph?${params}`).then(
      handleApiResponse<EntitySubgraph>
    );
  },

  syncGlobal: (): Promise<SyncGlobalStats> =>
    makeRequest('/api/graph/sync/global', { method: 'POST' }).then(
      handleApiResponse<SyncGlobalStats>
    ),

  getActions: (nodeType?: string): Promise<ActionsResponse> => {
    const qs = nodeType ? `?node_type=${encodeURIComponent(nodeType)}` : '';
    return makeRequest(`/api/graph/actions${qs}`).then(
      handleApiResponse<ActionsResponse>
    );
  },
};
