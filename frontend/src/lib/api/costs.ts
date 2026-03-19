import type {
  ModelCostRow,
  OrgCostSummary,
  ProjectCostRow,
} from 'shared/types';

import { handleApiResponse, makeRequest } from './client';

export const costsApi = {
  /** Get total VIBE + USD cost summary for an organization */
  orgSummary: async (orgId: string): Promise<OrgCostSummary> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/costs/summary`
    );
    return handleApiResponse<OrgCostSummary>(response);
  },

  /** Get cost breakdown by LLM model for an organization */
  orgByModel: async (orgId: string): Promise<ModelCostRow[]> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/costs/by-model`
    );
    return handleApiResponse<ModelCostRow[]>(response);
  },

  /** Get cost breakdown by project for an organization */
  orgByProject: async (orgId: string): Promise<ProjectCostRow[]> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/costs/by-project`
    );
    return handleApiResponse<ProjectCostRow[]>(response);
  },
};
