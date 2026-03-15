import type { GraphPlan, GraphPlanSummary, GraphNodeStatus } from 'shared/types';
import { makeRequest, ApiError, type NoraModeSummary, type RapidPlaybookResult } from './client';

export const syncNoraContext = async () => {
  const response = await makeRequest('/api/nora/context/sync', {
    method: 'POST',
  });
  if (!response.ok) {
    throw new ApiError('Failed to sync Nora context', response.status, response);
  }
  return (await response.json()) as { projects_refreshed: number };
};

export const fetchNoraModes = async (): Promise<NoraModeSummary[]> => {
  const response = await makeRequest('/api/nora/modes');
  if (!response.ok) {
    throw new ApiError('Failed to load Nora modes', response.status, response);
  }
  return (await response.json()) as NoraModeSummary[];
};

export const applyNoraMode = async (
  modeId: string,
  preserveMemory = true
): Promise<{ active_mode: string; nora_id: string }> => {
  const response = await makeRequest('/api/nora/modes/apply', {
    method: 'POST',
    body: JSON.stringify({ mode_id: modeId, preserve_memory: preserveMemory }),
  });
  if (!response.ok) {
    throw new ApiError('Failed to apply Nora mode', response.status, response);
  }
  return (await response.json()) as { active_mode: string; nora_id: string };
};

export interface RapidPlaybookPayload {
  project_name: string;
  objectives: string[];
  repo_hint?: string;
  notes?: string;
}

export const runRapidPlaybook = async (
  payload: RapidPlaybookPayload
): Promise<RapidPlaybookResult> => {
  const response = await makeRequest('/api/nora/playbooks/rapid', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new ApiError('Failed to run rapid playbook', response.status, response);
  }
  return (await response.json()) as RapidPlaybookResult;
};

export const fetchNoraPlans = async (): Promise<GraphPlanSummary[]> => {
  const response = await makeRequest('/api/nora/graph/plans');
  if (!response.ok) {
    throw new ApiError('Failed to load orchestration plans', response.status, response);
  }
  return (await response.json()) as GraphPlanSummary[];
};

export const fetchNoraPlan = async (planId: string): Promise<GraphPlan> => {
  const response = await makeRequest(`/api/nora/graph/plans/${planId}`);
  if (!response.ok) {
    throw new ApiError('Failed to load plan detail', response.status, response);
  }
  return (await response.json()) as GraphPlan;
};

export const updateNoraPlanNode = async (
  planId: string,
  nodeId: string,
  status: GraphNodeStatus
): Promise<GraphPlan> => {
  const response = await makeRequest(
    `/api/nora/graph/plans/${planId}/nodes/${nodeId}`,
    {
      method: 'PATCH',
      body: JSON.stringify({ status }),
    }
  );
  if (!response.ok) {
    throw new ApiError('Failed to update node status', response.status, response);
  }
  return (await response.json()) as GraphPlan;
};
