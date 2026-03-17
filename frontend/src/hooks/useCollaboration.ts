import { useQuery } from '@tanstack/react-query';
import { collaborationApi } from '@/lib/api';
import { collaborationKeys } from '@/lib/query-keys';
import { useMutationWithToast } from './useMutationWithToast';

// ========== Types ==========

export type ControlState = 'running' | 'paused' | 'human_takeover' | 'awaiting_input';
export type ActorType = 'agent' | 'human' | 'system';
export type HandoffType = 'takeover' | 'return' | 'escalation' | 'delegation' | 'assistance' | 'review_request';
export type InjectionType = 'note' | 'correction' | 'approval' | 'rejection' | 'directive' | 'question' | 'answer';
export type PauseAction = 'pause' | 'resume';

export interface ContextInjection {
  id: string;
  execution_process_id: string;
  injector_id: string;
  injector_name: string | null;
  injection_type: InjectionType;
  content: string;
  metadata: string | null;
  acknowledged: boolean;
  acknowledged_at: string | null;
  created_at: string;
}

export interface ExecutionHandoff {
  id: string;
  execution_process_id: string;
  from_actor_type: ActorType;
  from_actor_id: string;
  from_actor_name: string | null;
  to_actor_type: ActorType;
  to_actor_id: string;
  to_actor_name: string | null;
  handoff_type: HandoffType;
  reason: string | null;
  context_snapshot: string | null;
  created_at: string;
}

export interface ExecutionPauseHistory {
  id: string;
  execution_process_id: string;
  action: PauseAction;
  reason: string | null;
  initiated_by: string;
  initiated_by_name: string | null;
  created_at: string;
}

export interface ActorInfo {
  actor_type: ActorType;
  actor_id: string;
  actor_name: string | null;
}

export interface CollaborationState {
  execution_process_id: string;
  control_state: ControlState;
  current_controller: ActorInfo | null;
  pending_injections: ContextInjection[];
  recent_handoffs: ExecutionHandoff[];
  pause_history: ExecutionPauseHistory[];
}

// ========== Query Hooks ==========

export function useCollaborationState(executionId: string | undefined) {
  return useQuery({
    queryKey: collaborationKeys.state(executionId!),
    queryFn: () => collaborationApi.getState(executionId!),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function usePauseHistory(executionId: string | undefined) {
  return useQuery({
    queryKey: collaborationKeys.pauseHistory(executionId!),
    queryFn: () => collaborationApi.getPauseHistory(executionId!),
    enabled: !!executionId,
  });
}

export function useHandoffs(executionId: string | undefined) {
  return useQuery({
    queryKey: collaborationKeys.handoffs(executionId!),
    queryFn: () => collaborationApi.listHandoffs(executionId!),
    enabled: !!executionId,
  });
}

export function useInjections(executionId: string | undefined) {
  return useQuery({
    queryKey: collaborationKeys.injections(executionId!),
    queryFn: () => collaborationApi.listInjections(executionId!),
    enabled: !!executionId,
    refetchInterval: 3000,
  });
}

export function usePendingInjections(executionId: string | undefined) {
  return useQuery({
    queryKey: collaborationKeys.pendingInjections(executionId!),
    queryFn: () => collaborationApi.listPendingInjections(executionId!),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

// ========== Mutation Hooks ==========

export function usePauseExecution(executionId: string) {
  return useMutationWithToast({
    mutationFn: (data: { reason?: string; initiated_by: string; initiated_by_name?: string }) =>
      collaborationApi.pause(executionId, data),
    successMessage: 'Execution paused',
    errorMessage: 'Failed to pause execution',
    invalidateKeys: [collaborationKeys.state(executionId)],
  });
}

export function useResumeExecution(executionId: string) {
  return useMutationWithToast({
    mutationFn: (data: { initiated_by: string; initiated_by_name?: string }) =>
      collaborationApi.resume(executionId, data),
    successMessage: 'Execution resumed',
    errorMessage: 'Failed to resume execution',
    invalidateKeys: [collaborationKeys.state(executionId)],
  });
}

export function useTakeoverExecution(executionId: string) {
  return useMutationWithToast({
    mutationFn: (data: { human_id: string; human_name?: string; reason?: string }) =>
      collaborationApi.takeover(executionId, data),
    successMessage: 'Execution taken over',
    errorMessage: 'Failed to takeover execution',
    invalidateKeys: [collaborationKeys.state(executionId)],
  });
}

export function useReturnControl(executionId: string) {
  return useMutationWithToast({
    mutationFn: (data: {
      human_id: string;
      human_name?: string;
      to_agent_id: string;
      to_agent_name?: string;
      context_notes?: string;
    }) => collaborationApi.returnControl(executionId, data),
    successMessage: 'Control returned to agent',
    errorMessage: 'Failed to return control',
    invalidateKeys: [collaborationKeys.state(executionId)],
  });
}

export function useInjectContext(executionId: string) {
  return useMutationWithToast({
    mutationFn: (data: {
      injector_id: string;
      injector_name?: string;
      injection_type: InjectionType;
      content: string;
      metadata?: Record<string, unknown>;
    }) => collaborationApi.inject(executionId, data),
    successMessage: 'Context injected',
    errorMessage: 'Failed to inject context',
    invalidateKeys: [collaborationKeys.state(executionId)],
  });
}

export function useAcknowledgeInjection() {
  return useMutationWithToast({
    mutationFn: collaborationApi.acknowledgeInjection,
    successMessage: 'Injection acknowledged',
    errorMessage: 'Failed to acknowledge injection',
    invalidateKeys: [collaborationKeys.all],
  });
}

export function useAcknowledgeAllInjections(executionId: string) {
  return useMutationWithToast({
    mutationFn: () => collaborationApi.acknowledgeAll(executionId),
    successMessage: 'All injections acknowledged',
    errorMessage: 'Failed to acknowledge injections',
    invalidateKeys: [collaborationKeys.state(executionId)],
  });
}
