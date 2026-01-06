import { useQuery } from '@tanstack/react-query';

export interface AgentTaskPlan {
  id: string;
  execution_process_id: string;
  plan_json: string;
  current_step: number | null;
  total_steps: number | null;
  status: 'planning' | 'executing' | 'completed' | 'failed' | 'paused';
  created_at: string;
  updated_at: string;
}

export interface ExecutionArtifact {
  id: string;
  execution_process_id: string;
  artifact_type: 'plan' | 'screenshot' | 'walkthrough' | 'diff_summary' | 'test_result' | 'checkpoint' | 'error_report';
  title: string;
  content: string | null;
  file_path: string | null;
  metadata: string | null;
  created_at: string;
}

export interface ActiveExecutionInfo {
  process: {
    id: string;
    task_attempt_id: string;
    run_reason: string;
    status: string;
    started_at: string;
    completed_at: string | null;
  };
  task_id: string;
  task_title: string;
  project_id: string;
  project_name: string;
  executor: string;
  plan: AgentTaskPlan | null;
  artifacts_count: number;
}

export interface ProjectExecutionSummary {
  project_id: string;
  project_name: string;
  active_count: number;
  capacity: {
    max_concurrent_agents: number;
    max_concurrent_browser_agents: number;
    active_agent_slots: number;
    active_browser_slots: number;
    available_agent_slots: number;
    available_browser_slots: number;
  };
}

export interface ActiveWorkflowInfo {
  flow: {
    id: string;
    task_id: string;
    flow_type: string;
    status: string;
    current_phase: string;
    created_at: string;
    updated_at: string;
  };
  task_id: string;
  task_title: string;
  project_id: string | null;
  project_name: string | null;
  events_count: number;
}

export interface MissionControlDashboard {
  active_executions: ActiveExecutionInfo[];
  active_workflows: ActiveWorkflowInfo[];
  total_active: number;
  by_project: ProjectExecutionSummary[];
}

async function fetchMissionControlDashboard(): Promise<MissionControlDashboard> {
  const response = await fetch('/api/mission-control');
  if (!response.ok) {
    throw new Error('Failed to fetch Mission Control dashboard');
  }
  const json = await response.json();
  return json.data;
}

async function fetchExecutionArtifacts(executionId: string): Promise<ExecutionArtifact[]> {
  const response = await fetch(`/api/mission-control/executions/${executionId}/artifacts`);
  if (!response.ok) {
    throw new Error('Failed to fetch execution artifacts');
  }
  const json = await response.json();
  return json.data;
}

async function fetchExecutionPlan(executionId: string): Promise<AgentTaskPlan | null> {
  const response = await fetch(`/api/mission-control/executions/${executionId}/plan`);
  if (!response.ok) {
    throw new Error('Failed to fetch execution plan');
  }
  const json = await response.json();
  return json.data;
}

async function fetchActivePlans(): Promise<AgentTaskPlan[]> {
  const response = await fetch('/api/mission-control/plans/active');
  if (!response.ok) {
    throw new Error('Failed to fetch active plans');
  }
  const json = await response.json();
  return json.data;
}

/**
 * Hook to get Mission Control dashboard data
 */
export function useMissionControlDashboard() {
  return useQuery({
    queryKey: ['missionControl', 'dashboard'],
    queryFn: fetchMissionControlDashboard,
    refetchInterval: 3000, // Refresh every 3 seconds for real-time updates
  });
}

/**
 * Hook to get artifacts for a specific execution
 */
export function useExecutionArtifacts(executionId: string | undefined) {
  return useQuery({
    queryKey: ['missionControl', 'artifacts', executionId],
    queryFn: () => fetchExecutionArtifacts(executionId!),
    enabled: !!executionId,
    refetchInterval: 5000,
  });
}

/**
 * Hook to get plan for a specific execution
 */
export function useExecutionPlan(executionId: string | undefined) {
  return useQuery({
    queryKey: ['missionControl', 'plan', executionId],
    queryFn: () => fetchExecutionPlan(executionId!),
    enabled: !!executionId,
    refetchInterval: 5000,
  });
}

/**
 * Hook to get all active plans
 */
export function useActivePlans() {
  return useQuery({
    queryKey: ['missionControl', 'activePlans'],
    queryFn: fetchActivePlans,
    refetchInterval: 5000,
  });
}

/**
 * Parse plan steps from JSON string
 */
export function parsePlanSteps(planJson: string): Array<{
  index: number;
  title: string;
  description: string | null;
  status: 'pending' | 'in_progress' | 'completed' | 'failed' | 'skipped';
  started_at: string | null;
  completed_at: string | null;
}> {
  try {
    return JSON.parse(planJson);
  } catch {
    return [];
  }
}
