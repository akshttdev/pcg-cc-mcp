import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

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

// ========== API Functions ==========

async function fetchCollaborationState(executionId: string): Promise<CollaborationState> {
  const response = await fetch(`/api/executions/${executionId}/collaboration`);
  if (!response.ok) throw new Error('Failed to fetch collaboration state');
  const json = await response.json();
  return json.data;
}

async function fetchPauseHistory(executionId: string): Promise<ExecutionPauseHistory[]> {
  const response = await fetch(`/api/executions/${executionId}/pause-history`);
  if (!response.ok) throw new Error('Failed to fetch pause history');
  const json = await response.json();
  return json.data;
}

async function fetchHandoffs(executionId: string): Promise<ExecutionHandoff[]> {
  const response = await fetch(`/api/executions/${executionId}/handoffs`);
  if (!response.ok) throw new Error('Failed to fetch handoffs');
  const json = await response.json();
  return json.data;
}

async function fetchInjections(executionId: string): Promise<ContextInjection[]> {
  const response = await fetch(`/api/executions/${executionId}/injections`);
  if (!response.ok) throw new Error('Failed to fetch injections');
  const json = await response.json();
  return json.data;
}

async function fetchPendingInjections(executionId: string): Promise<ContextInjection[]> {
  const response = await fetch(`/api/executions/${executionId}/injections/pending`);
  if (!response.ok) throw new Error('Failed to fetch pending injections');
  const json = await response.json();
  return json.data;
}

async function pauseExecution(executionId: string, data: {
  reason?: string;
  initiated_by: string;
  initiated_by_name?: string;
}): Promise<ExecutionPauseHistory> {
  const response = await fetch(`/api/executions/${executionId}/pause`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to pause execution');
  const json = await response.json();
  return json.data;
}

async function resumeExecution(executionId: string, data: {
  initiated_by: string;
  initiated_by_name?: string;
}): Promise<ExecutionPauseHistory> {
  const response = await fetch(`/api/executions/${executionId}/resume`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to resume execution');
  const json = await response.json();
  return json.data;
}

async function takeoverExecution(executionId: string, data: {
  human_id: string;
  human_name?: string;
  reason?: string;
}): Promise<ExecutionHandoff> {
  const response = await fetch(`/api/executions/${executionId}/takeover`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to takeover execution');
  const json = await response.json();
  return json.data;
}

async function returnControl(executionId: string, data: {
  human_id: string;
  human_name?: string;
  to_agent_id: string;
  to_agent_name?: string;
  context_notes?: string;
}): Promise<ExecutionHandoff> {
  const response = await fetch(`/api/executions/${executionId}/return-control`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to return control');
  const json = await response.json();
  return json.data;
}

async function injectContext(executionId: string, data: {
  injector_id: string;
  injector_name?: string;
  injection_type: InjectionType;
  content: string;
  metadata?: Record<string, unknown>;
}): Promise<ContextInjection> {
  const response = await fetch(`/api/executions/${executionId}/inject`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to inject context');
  const json = await response.json();
  return json.data;
}

async function acknowledgeInjection(injectionId: string): Promise<ContextInjection> {
  const response = await fetch(`/api/injections/${injectionId}/acknowledge`, {
    method: 'POST',
  });
  if (!response.ok) throw new Error('Failed to acknowledge injection');
  const json = await response.json();
  return json.data;
}

async function acknowledgeAllInjections(executionId: string): Promise<number> {
  const response = await fetch(`/api/executions/${executionId}/injections/acknowledge-all`, {
    method: 'POST',
  });
  if (!response.ok) throw new Error('Failed to acknowledge all injections');
  const json = await response.json();
  return json.data;
}

// ========== Hooks ==========

export function useCollaborationState(executionId: string | undefined) {
  return useQuery({
    queryKey: ['collaboration', executionId],
    queryFn: () => fetchCollaborationState(executionId!),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function usePauseHistory(executionId: string | undefined) {
  return useQuery({
    queryKey: ['collaboration', executionId, 'pause-history'],
    queryFn: () => fetchPauseHistory(executionId!),
    enabled: !!executionId,
  });
}

export function useHandoffs(executionId: string | undefined) {
  return useQuery({
    queryKey: ['collaboration', executionId, 'handoffs'],
    queryFn: () => fetchHandoffs(executionId!),
    enabled: !!executionId,
  });
}

export function useInjections(executionId: string | undefined) {
  return useQuery({
    queryKey: ['collaboration', executionId, 'injections'],
    queryFn: () => fetchInjections(executionId!),
    enabled: !!executionId,
    refetchInterval: 3000,
  });
}

export function usePendingInjections(executionId: string | undefined) {
  return useQuery({
    queryKey: ['collaboration', executionId, 'pending-injections'],
    queryFn: () => fetchPendingInjections(executionId!),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function usePauseExecution(executionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: { reason?: string; initiated_by: string; initiated_by_name?: string }) =>
      pauseExecution(executionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration', executionId] });
    },
  });
}

export function useResumeExecution(executionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: { initiated_by: string; initiated_by_name?: string }) =>
      resumeExecution(executionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration', executionId] });
    },
  });
}

export function useTakeoverExecution(executionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: { human_id: string; human_name?: string; reason?: string }) =>
      takeoverExecution(executionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration', executionId] });
    },
  });
}

export function useReturnControl(executionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      human_id: string;
      human_name?: string;
      to_agent_id: string;
      to_agent_name?: string;
      context_notes?: string;
    }) => returnControl(executionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration', executionId] });
    },
  });
}

export function useInjectContext(executionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: {
      injector_id: string;
      injector_name?: string;
      injection_type: InjectionType;
      content: string;
      metadata?: Record<string, unknown>;
    }) => injectContext(executionId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration', executionId] });
    },
  });
}

export function useAcknowledgeInjection() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: acknowledgeInjection,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration'] });
    },
  });
}

export function useAcknowledgeAllInjections(executionId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => acknowledgeAllInjections(executionId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['collaboration', executionId] });
    },
  });
}
