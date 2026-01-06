import { useQuery } from '@tanstack/react-query';

export interface ProjectCapacity {
  project_id: string;
  max_concurrent_agents: number;
  max_concurrent_browser_agents: number;
  active_agent_slots: number;
  active_browser_slots: number;
  available_agent_slots: number;
  available_browser_slots: number;
}

export interface ExecutionSlot {
  id: string;
  task_attempt_id: string;
  slot_type: 'coding_agent' | 'browser_agent' | 'script';
  resource_weight: number;
  acquired_at: string;
  released_at: string | null;
  created_at: string;
}

export interface ActiveExecution {
  id: string;
  task_attempt_id: string;
  status: string;
  run_reason: string;
  started_at: string;
}

export interface ActiveExecutionsResponse {
  count: number;
  executions: ActiveExecution[];
}

async function fetchProjectCapacity(projectId: string): Promise<ProjectCapacity> {
  const response = await fetch(`/api/projects/${projectId}/capacity`);
  if (!response.ok) {
    throw new Error('Failed to fetch project capacity');
  }
  const json = await response.json();
  return json.data;
}

async function fetchActiveSlots(projectId: string): Promise<ExecutionSlot[]> {
  const response = await fetch(`/api/projects/${projectId}/slots`);
  if (!response.ok) {
    throw new Error('Failed to fetch active slots');
  }
  const json = await response.json();
  return json.data;
}

async function fetchActiveExecutions(): Promise<ActiveExecutionsResponse> {
  const response = await fetch('/api/execution/active');
  if (!response.ok) {
    throw new Error('Failed to fetch active executions');
  }
  const json = await response.json();
  return json.data;
}

/**
 * Hook to get project capacity and slot utilization
 */
export function useProjectCapacity(projectId: string | undefined) {
  return useQuery({
    queryKey: ['projectCapacity', projectId],
    queryFn: () => fetchProjectCapacity(projectId!),
    enabled: !!projectId,
    refetchInterval: 5000, // Refresh every 5 seconds to track slot changes
  });
}

/**
 * Hook to get active execution slots for a project
 */
export function useActiveSlots(projectId: string | undefined) {
  return useQuery({
    queryKey: ['activeSlots', projectId],
    queryFn: () => fetchActiveSlots(projectId!),
    enabled: !!projectId,
    refetchInterval: 5000,
  });
}

/**
 * Hook to get all active executions across all projects
 */
export function useActiveExecutions() {
  return useQuery({
    queryKey: ['activeExecutions'],
    queryFn: fetchActiveExecutions,
    refetchInterval: 5000,
  });
}

/**
 * Check if a new execution can be started for a project
 */
export function canStartExecution(
  capacity: ProjectCapacity | undefined,
  slotType: 'coding_agent' | 'browser_agent' = 'coding_agent'
): boolean {
  if (!capacity) return false;

  if (slotType === 'browser_agent') {
    return capacity.available_browser_slots > 0;
  }

  return capacity.available_agent_slots > 0;
}
