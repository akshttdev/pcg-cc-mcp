import { useQuery } from '@tanstack/react-query';
import { agentFlowsApi, type AgentFlow } from '@/lib/api';
import { agentFlowKeys } from '@/lib/query-keys';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';

/**
 * Fetch all agent flows, optionally filtered by task_id or status
 */
export function useAgentFlows(params?: { taskId?: string; status?: string }) {
  return useQuery({
    queryKey: agentFlowKeys.list(params?.taskId, params?.status),
    queryFn: () => agentFlowsApi.list({
      task_id: params?.taskId,
      status: params?.status,
    }),
    staleTime: 30 * 1000, // 30 seconds
  });
}

/**
 * Fetch a single agent flow by ID
 */
export function useAgentFlow(flowId: string | undefined) {
  return useQuery({
    queryKey: agentFlowKeys.detail(flowId),
    queryFn: () => agentFlowsApi.getById(flowId!),
    enabled: !!flowId,
    staleTime: 30 * 1000,
  });
}

/**
 * Fetch agent flows awaiting approval
 */
export function useAgentFlowsAwaitingApproval() {
  return useQuery({
    queryKey: agentFlowKeys.awaitingApproval(),
    queryFn: () => agentFlowsApi.listAwaitingApproval(),
    staleTime: 15 * 1000, // 15 seconds for approval queue
  });
}

/**
 * Fetch agent flow events
 */
export function useAgentFlowEvents(
  flowId: string | undefined,
  params?: { since?: string; eventType?: string }
) {
  return useQuery({
    queryKey: agentFlowKeys.events(flowId, params?.since, params?.eventType),
    queryFn: () => agentFlowsApi.getEvents(flowId!, {
      since: params?.since,
      event_type: params?.eventType,
    }),
    enabled: !!flowId,
    staleTime: 10 * 1000,
  });
}

/**
 * Build a map of task IDs to their latest agent flow
 * This is useful for displaying flow status on task cards
 */
export function useTaskAgentFlowMap(taskIds: string[]) {
  const { data: flows = [], ...rest } = useQuery({
    queryKey: agentFlowKeys.byTasks(taskIds),
    queryFn: async () => {
      // Fetch all flows (we could optimize this with a batch endpoint)
      const allFlows = await agentFlowsApi.list();
      return allFlows.filter(flow => taskIds.includes(flow.task_id));
    },
    enabled: taskIds.length > 0,
    staleTime: 30 * 1000,
  });

  // Build map: task_id -> latest flow
  const flowMap = new Map<string, AgentFlow>();
  for (const flow of flows) {
    const existing = flowMap.get(flow.task_id);
    // Keep the most recent flow
    if (!existing || new Date(flow.created_at) > new Date(existing.created_at)) {
      flowMap.set(flow.task_id, flow);
    }
  }

  return { flowMap, flows, ...rest };
}

/**
 * Mutations for agent flow operations
 */
export function useAgentFlowMutations() {
  const invalidateKeys = [agentFlowKeys.all, agentFlowKeys.detailAll] as const;

  const approveMutation = useMutationWithToast({
    mutationFn: ({ flowId, approvedBy }: { flowId: string; approvedBy: string }) =>
      agentFlowsApi.approve(flowId, approvedBy),
    successMessage: 'Agent flow approved',
    errorMessage: 'Failed to approve agent flow',
    invalidateKeys,
  });

  const transitionPhaseMutation = useMutationWithToast({
    mutationFn: ({ flowId, phase }: { flowId: string; phase: string }) =>
      agentFlowsApi.transitionPhase(flowId, phase),
    successMessage: 'Phase transitioned successfully',
    errorMessage: 'Failed to transition phase',
    invalidateKeys,
  });

  const completeMutation = useMutationWithToast({
    mutationFn: ({ flowId, score }: { flowId: string; score?: number }) =>
      agentFlowsApi.complete(flowId, score),
    successMessage: 'Agent flow completed',
    errorMessage: 'Failed to complete agent flow',
    invalidateKeys,
  });

  const requestApprovalMutation = useMutationWithToast({
    mutationFn: (flowId: string) => agentFlowsApi.requestApproval(flowId),
    successMessage: 'Approval requested',
    errorMessage: 'Failed to request approval',
    invalidateKeys,
  });

  const respondClarificationMutation = useMutationWithToast({
    mutationFn: ({
      flowId,
      response,
      resumeStatus,
    }: {
      flowId: string;
      response: string;
      resumeStatus?: string;
    }) =>
      agentFlowsApi.respondClarification(flowId, {
        response,
        resume_status: resumeStatus,
      }),
    successMessage: 'Clarification response sent',
    errorMessage: 'Failed to send clarification response',
    invalidateKeys,
  });

  return {
    approve: approveMutation,
    transitionPhase: transitionPhaseMutation,
    complete: completeMutation,
    requestApproval: requestApprovalMutation,
    respondClarification: respondClarificationMutation,
  };
}
