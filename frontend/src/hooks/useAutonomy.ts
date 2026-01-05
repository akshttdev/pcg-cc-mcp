/**
 * Autonomy Modes - React Query Hooks
 *
 * Hooks for managing autonomy modes, checkpoints, and approval gates.
 */

import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';

// ========== Types ==========

export type AutonomyMode = 'agent_driven' | 'agent_assisted' | 'review_driven';
export type CheckpointType = 'file_change' | 'external_call' | 'cost_threshold' | 'time_threshold' | 'custom';
export type CheckpointStatus = 'pending' | 'approved' | 'rejected' | 'auto_approved' | 'skipped' | 'expired';
export type GateType = 'pre_execution' | 'post_plan' | 'pre_commit' | 'post_execution' | 'custom';
export type ApprovalDecision = 'approved' | 'rejected' | 'abstained';
export type PendingGateStatus = 'pending' | 'approved' | 'rejected' | 'bypassed';

export interface CheckpointDefinition {
  id: string;
  project_id: string | null;
  name: string;
  description: string | null;
  checkpoint_type: CheckpointType;
  config: Record<string, unknown>;
  requires_approval: boolean;
  auto_approve_after_minutes: number | null;
  priority: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface ExecutionCheckpoint {
  id: string;
  execution_process_id: string;
  checkpoint_definition_id: string | null;
  checkpoint_data: Record<string, unknown>;
  trigger_reason: string | null;
  status: CheckpointStatus;
  reviewer_id: string | null;
  reviewer_name: string | null;
  review_note: string | null;
  reviewed_at: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface ApprovalGate {
  id: string;
  project_id: string | null;
  task_id: string | null;
  name: string;
  gate_type: GateType;
  required_approvers: string[];
  min_approvals: number;
  conditions: Record<string, unknown>;
  is_active: boolean;
  created_at: string;
}

export interface PendingGate {
  id: string;
  approval_gate_id: string;
  execution_process_id: string;
  status: PendingGateStatus;
  trigger_context: Record<string, unknown> | null;
  approval_count: number;
  rejection_count: number;
  resolved_at: string | null;
  created_at: string;
}

export interface GateApproval {
  id: string;
  approval_gate_id: string;
  execution_process_id: string;
  approver_id: string;
  approver_name: string | null;
  decision: ApprovalDecision;
  comment: string | null;
  created_at: string;
}

export interface PendingApprovalsSummary {
  pending_checkpoints: number;
  pending_gates: number;
  total_pending: number;
  oldest_pending_at: string | null;
}

// ========== Request Types ==========

export interface CreateCheckpointDefinitionRequest {
  project_id?: string;
  name: string;
  description?: string;
  checkpoint_type: CheckpointType;
  config?: Record<string, unknown>;
  requires_approval?: boolean;
  auto_approve_after_minutes?: number;
  priority?: number;
}

export interface UpdateCheckpointDefinitionRequest {
  name?: string;
  description?: string;
  config?: Record<string, unknown>;
  requires_approval?: boolean;
  auto_approve_after_minutes?: number | null;
  priority?: number;
  is_active?: boolean;
}

export interface ReviewCheckpointRequest {
  reviewer_id: string;
  reviewer_name?: string;
  decision: CheckpointStatus;
  review_note?: string;
}

export interface CreateApprovalGateRequest {
  project_id?: string;
  task_id?: string;
  name: string;
  gate_type: GateType;
  required_approvers: string[];
  min_approvals?: number;
  conditions?: Record<string, unknown>;
}

export interface SubmitApprovalRequest {
  approver_id: string;
  approver_name?: string;
  decision: ApprovalDecision;
  comment?: string;
}

// ========== Fetch Functions ==========

async function fetchTaskAutonomyMode(taskId: string): Promise<AutonomyMode> {
  const response = await fetch(`/api/tasks/${taskId}/autonomy-mode`);
  if (!response.ok) throw new Error('Failed to fetch autonomy mode');
  const json = await response.json();
  return json.data;
}

async function setTaskAutonomyMode(taskId: string, mode: AutonomyMode): Promise<void> {
  const response = await fetch(`/api/tasks/${taskId}/autonomy-mode`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mode }),
  });
  if (!response.ok) throw new Error('Failed to set autonomy mode');
}

async function fetchCheckpointDefinitions(projectId: string): Promise<CheckpointDefinition[]> {
  const response = await fetch(`/api/projects/${projectId}/checkpoint-definitions`);
  if (!response.ok) throw new Error('Failed to fetch checkpoint definitions');
  const json = await response.json();
  return json.data;
}

async function createCheckpointDefinition(req: CreateCheckpointDefinitionRequest): Promise<CheckpointDefinition> {
  const response = await fetch('/api/autonomy/checkpoint-definitions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!response.ok) throw new Error('Failed to create checkpoint definition');
  const json = await response.json();
  return json.data;
}

async function updateCheckpointDefinition(
  definitionId: string,
  req: UpdateCheckpointDefinitionRequest
): Promise<CheckpointDefinition> {
  const response = await fetch(`/api/autonomy/checkpoint-definitions/${definitionId}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!response.ok) throw new Error('Failed to update checkpoint definition');
  const json = await response.json();
  return json.data;
}

async function deleteCheckpointDefinition(definitionId: string): Promise<void> {
  const response = await fetch(`/api/autonomy/checkpoint-definitions/${definitionId}`, {
    method: 'DELETE',
  });
  if (!response.ok) throw new Error('Failed to delete checkpoint definition');
}

async function fetchExecutionCheckpoints(executionId: string): Promise<ExecutionCheckpoint[]> {
  const response = await fetch(`/api/executions/${executionId}/checkpoints`);
  if (!response.ok) throw new Error('Failed to fetch checkpoints');
  const json = await response.json();
  return json.data;
}

async function fetchPendingCheckpoints(executionId: string): Promise<ExecutionCheckpoint[]> {
  const response = await fetch(`/api/executions/${executionId}/checkpoints/pending`);
  if (!response.ok) throw new Error('Failed to fetch pending checkpoints');
  const json = await response.json();
  return json.data;
}

async function reviewCheckpoint(checkpointId: string, req: ReviewCheckpointRequest): Promise<ExecutionCheckpoint> {
  const response = await fetch(`/api/checkpoints/${checkpointId}/review`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!response.ok) throw new Error('Failed to review checkpoint');
  const json = await response.json();
  return json.data;
}

async function skipCheckpoint(checkpointId: string): Promise<ExecutionCheckpoint> {
  const response = await fetch(`/api/checkpoints/${checkpointId}/skip`, {
    method: 'POST',
  });
  if (!response.ok) throw new Error('Failed to skip checkpoint');
  const json = await response.json();
  return json.data;
}

async function fetchProjectGates(projectId: string): Promise<ApprovalGate[]> {
  const response = await fetch(`/api/projects/${projectId}/approval-gates`);
  if (!response.ok) throw new Error('Failed to fetch approval gates');
  const json = await response.json();
  return json.data;
}

async function createApprovalGate(req: CreateApprovalGateRequest): Promise<ApprovalGate> {
  const response = await fetch('/api/autonomy/approval-gates', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!response.ok) throw new Error('Failed to create approval gate');
  const json = await response.json();
  return json.data;
}

async function deleteApprovalGate(gateId: string): Promise<void> {
  const response = await fetch(`/api/autonomy/approval-gates/${gateId}`, {
    method: 'DELETE',
  });
  if (!response.ok) throw new Error('Failed to delete approval gate');
}

async function fetchPendingGates(executionId: string): Promise<PendingGate[]> {
  const response = await fetch(`/api/executions/${executionId}/gates`);
  if (!response.ok) throw new Error('Failed to fetch pending gates');
  const json = await response.json();
  return json.data;
}

async function submitGateApproval(pendingGateId: string, req: SubmitApprovalRequest): Promise<GateApproval> {
  const response = await fetch(`/api/pending-gates/${pendingGateId}/approve`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!response.ok) throw new Error('Failed to submit approval');
  const json = await response.json();
  return json.data;
}

async function bypassGate(pendingGateId: string): Promise<PendingGate> {
  const response = await fetch(`/api/pending-gates/${pendingGateId}/bypass`, {
    method: 'POST',
  });
  if (!response.ok) throw new Error('Failed to bypass gate');
  const json = await response.json();
  return json.data;
}

async function fetchPendingApprovalsSummary(): Promise<PendingApprovalsSummary> {
  const response = await fetch('/api/autonomy/pending-approvals');
  if (!response.ok) throw new Error('Failed to fetch pending approvals');
  const json = await response.json();
  return json.data;
}

async function fetchCanProceed(executionId: string): Promise<boolean> {
  const response = await fetch(`/api/executions/${executionId}/can-proceed`);
  if (!response.ok) throw new Error('Failed to check proceed status');
  const json = await response.json();
  return json.data;
}

// ========== Query Hooks ==========

export function useTaskAutonomyMode(taskId: string) {
  return useQuery({
    queryKey: ['autonomy', 'task', taskId, 'mode'],
    queryFn: () => fetchTaskAutonomyMode(taskId),
    enabled: !!taskId,
  });
}

export function useCheckpointDefinitions(projectId: string) {
  return useQuery({
    queryKey: ['autonomy', 'checkpoints', 'definitions', projectId],
    queryFn: () => fetchCheckpointDefinitions(projectId),
    enabled: !!projectId,
  });
}

export function useExecutionCheckpoints(executionId: string) {
  return useQuery({
    queryKey: ['autonomy', 'checkpoints', executionId],
    queryFn: () => fetchExecutionCheckpoints(executionId),
    enabled: !!executionId,
    refetchInterval: 3000,
  });
}

export function usePendingCheckpoints(executionId: string) {
  return useQuery({
    queryKey: ['autonomy', 'checkpoints', executionId, 'pending'],
    queryFn: () => fetchPendingCheckpoints(executionId),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function useProjectGates(projectId: string) {
  return useQuery({
    queryKey: ['autonomy', 'gates', projectId],
    queryFn: () => fetchProjectGates(projectId),
    enabled: !!projectId,
  });
}

export function usePendingGates(executionId: string) {
  return useQuery({
    queryKey: ['autonomy', 'gates', executionId, 'pending'],
    queryFn: () => fetchPendingGates(executionId),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function usePendingApprovalsSummary() {
  return useQuery({
    queryKey: ['autonomy', 'pending-summary'],
    queryFn: fetchPendingApprovalsSummary,
    refetchInterval: 5000,
  });
}

export function useCanProceed(executionId: string) {
  return useQuery({
    queryKey: ['autonomy', 'can-proceed', executionId],
    queryFn: () => fetchCanProceed(executionId),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

// ========== Mutation Hooks ==========

export function useSetTaskAutonomyMode() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ taskId, mode }: { taskId: string; mode: AutonomyMode }) =>
      setTaskAutonomyMode(taskId, mode),
    onSuccess: (_, { taskId }) => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'task', taskId] });
    },
  });
}

export function useCreateCheckpointDefinition() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: createCheckpointDefinition,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'checkpoints', 'definitions'] });
    },
  });
}

export function useUpdateCheckpointDefinition() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ definitionId, ...req }: UpdateCheckpointDefinitionRequest & { definitionId: string }) =>
      updateCheckpointDefinition(definitionId, req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'checkpoints', 'definitions'] });
    },
  });
}

export function useDeleteCheckpointDefinition() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: deleteCheckpointDefinition,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'checkpoints', 'definitions'] });
    },
  });
}

export function useReviewCheckpoint() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ checkpointId, ...req }: ReviewCheckpointRequest & { checkpointId: string }) =>
      reviewCheckpoint(checkpointId, req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'checkpoints'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'pending-summary'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'can-proceed'] });
    },
  });
}

export function useSkipCheckpoint() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: skipCheckpoint,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'checkpoints'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'pending-summary'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'can-proceed'] });
    },
  });
}

export function useCreateApprovalGate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: createApprovalGate,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'gates'] });
    },
  });
}

export function useDeleteApprovalGate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: deleteApprovalGate,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'gates'] });
    },
  });
}

export function useSubmitGateApproval() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ pendingGateId, ...req }: SubmitApprovalRequest & { pendingGateId: string }) =>
      submitGateApproval(pendingGateId, req),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'gates'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'pending-summary'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'can-proceed'] });
    },
  });
}

export function useBypassGate() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: bypassGate,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'gates'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'pending-summary'] });
      queryClient.invalidateQueries({ queryKey: ['autonomy', 'can-proceed'] });
    },
  });
}
