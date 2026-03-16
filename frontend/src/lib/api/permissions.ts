import type { Project } from 'shared/types';
import { makeRequest, handleApiResponse } from './client';

export interface VibeBudgetResponse {
  vibe_budget_limit: number | null;
  vibe_spent_amount: number;
  vibe_remaining: number | null;
}

export const permissionsApi = {
  getProjectMemberCount: async (projectId: string): Promise<number> => {
    try {
      const response = await makeRequest(`/api/permissions/projects/${projectId}/members`);
      const data = await handleApiResponse<unknown[]>(response);
      return data?.length || 0;
    } catch {
      return 0;
    }
  },

  getProjectBudget: async (projectId: string): Promise<VibeBudgetResponse> => {
    const response = await makeRequest(`/api/projects/${projectId}/budget`);
    return handleApiResponse<VibeBudgetResponse>(response);
  },

  setProjectBudget: async (
    projectId: string,
    budgetLimit: number | null,
  ): Promise<VibeBudgetResponse> => {
    const response = await makeRequest(`/api/projects/${projectId}/budget`, {
      method: 'PUT',
      body: JSON.stringify({ vibe_budget_limit: budgetLimit }),
    });
    return handleApiResponse<VibeBudgetResponse>(response);
  },

  listProjects: async (filters?: { search?: string }): Promise<Project[]> => {
    const params = new URLSearchParams();
    if (filters?.search) params.append('search', filters.search);
    const response = await makeRequest(`/api/projects?${params}`);
    return handleApiResponse<Project[]>(response);
  },
};
