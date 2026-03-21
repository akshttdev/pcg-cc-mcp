import type {
  AgentCostRow,
  DailyCostRow,
  ModelCostRow,
  OrgCostSummary,
  ProjectCostRow,
  ProviderCostRow,
} from 'shared/types';

import { handleApiResponse, makeRequest } from './client';

function costUrl(orgId: string, path: string, days?: number): string {
  const base = `/api/organizations/${orgId}/costs/${path}`;
  return days ? `${base}?days=${days}` : base;
}

export const costsApi = {
  orgSummary: async (orgId: string, days?: number): Promise<OrgCostSummary> => {
    const response = await makeRequest(costUrl(orgId, 'summary', days));
    return handleApiResponse<OrgCostSummary>(response);
  },

  orgDaily: async (orgId: string, days?: number): Promise<DailyCostRow[]> => {
    const response = await makeRequest(costUrl(orgId, 'daily', days));
    return handleApiResponse<DailyCostRow[]>(response);
  },

  orgByModel: async (orgId: string, days?: number): Promise<ModelCostRow[]> => {
    const response = await makeRequest(costUrl(orgId, 'by-model', days));
    return handleApiResponse<ModelCostRow[]>(response);
  },

  orgByProject: async (orgId: string, days?: number): Promise<ProjectCostRow[]> => {
    const response = await makeRequest(costUrl(orgId, 'by-project', days));
    return handleApiResponse<ProjectCostRow[]>(response);
  },

  orgByProvider: async (orgId: string, days?: number): Promise<ProviderCostRow[]> => {
    const response = await makeRequest(costUrl(orgId, 'by-provider', days));
    return handleApiResponse<ProviderCostRow[]>(response);
  },

  orgByAgent: async (orgId: string, days?: number): Promise<AgentCostRow[]> => {
    const response = await makeRequest(costUrl(orgId, 'by-agent', days));
    return handleApiResponse<AgentCostRow[]>(response);
  },
};
