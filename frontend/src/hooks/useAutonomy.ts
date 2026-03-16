/**
 * Autonomy Modes - React Query Hooks
 *
 * Hooks for managing autonomy modes, checkpoints, and approval gates.
 */

import { useQuery } from '@tanstack/react-query';
import { autonomyApi } from '@/lib/api';
import { autonomyKeys } from '@/lib/query-keys';
import { useMutationWithToast } from './useMutationWithToast';

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

// ========== Query Hooks ==========

export function useTaskAutonomyMode(taskId: string) {
  return useQuery({
    queryKey: autonomyKeys.taskMode(taskId),
    queryFn: () => autonomyApi.getTaskMode(taskId),
    enabled: !!taskId,
  });
}

export function useCheckpointDefinitions(projectId: string) {
  return useQuery({
    queryKey: autonomyKeys.checkpointDefinitions(projectId),
    queryFn: () => autonomyApi.listCheckpointDefinitions(projectId),
    enabled: !!projectId,
  });
}

export function useExecutionCheckpoints(executionId: string) {
  return useQuery({
    queryKey: autonomyKeys.executionCheckpoints(executionId),
    queryFn: () => autonomyApi.listExecutionCheckpoints(executionId),
    enabled: !!executionId,
    refetchInterval: 3000,
  });
}

export function usePendingCheckpoints(executionId: string) {
  return useQuery({
    queryKey: autonomyKeys.pendingCheckpoints(executionId),
    queryFn: () => autonomyApi.listPendingCheckpoints(executionId),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function useProjectGates(projectId: string) {
  return useQuery({
    queryKey: autonomyKeys.projectGates(projectId),
    queryFn: () => autonomyApi.listProjectGates(projectId),
    enabled: !!projectId,
  });
}

export function usePendingGates(executionId: string) {
  return useQuery({
    queryKey: autonomyKeys.pendingGates(executionId),
    queryFn: () => autonomyApi.listPendingGates(executionId),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

export function usePendingApprovalsSummary() {
  return useQuery({
    queryKey: autonomyKeys.pendingSummary(),
    queryFn: autonomyApi.getPendingApprovalsSummary,
    refetchInterval: 5000,
  });
}

export function useCanProceed(executionId: string) {
  return useQuery({
    queryKey: autonomyKeys.canProceed(executionId),
    queryFn: () => autonomyApi.canProceed(executionId),
    enabled: !!executionId,
    refetchInterval: 2000,
  });
}

// ========== Mutation Hooks ==========

export function useSetTaskAutonomyMode() {
  return useMutationWithToast({
    mutationFn: ({ taskId, mode }: { taskId: string; mode: AutonomyMode }) =>
      autonomyApi.setTaskMode(taskId, mode),
    successMessage: 'Autonomy mode updated',
    errorMessage: 'Failed to set autonomy mode',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useCreateCheckpointDefinition() {
  return useMutationWithToast({
    mutationFn: autonomyApi.createCheckpointDefinition,
    successMessage: 'Checkpoint definition created',
    errorMessage: 'Failed to create checkpoint definition',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useUpdateCheckpointDefinition() {
  return useMutationWithToast({
    mutationFn: ({ definitionId, ...req }: UpdateCheckpointDefinitionRequest & { definitionId: string }) =>
      autonomyApi.updateCheckpointDefinition(definitionId, req),
    successMessage: 'Checkpoint definition updated',
    errorMessage: 'Failed to update checkpoint definition',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useDeleteCheckpointDefinition() {
  return useMutationWithToast({
    mutationFn: autonomyApi.deleteCheckpointDefinition,
    successMessage: 'Checkpoint definition deleted',
    errorMessage: 'Failed to delete checkpoint definition',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useReviewCheckpoint() {
  return useMutationWithToast({
    mutationFn: ({ checkpointId, ...req }: ReviewCheckpointRequest & { checkpointId: string }) =>
      autonomyApi.reviewCheckpoint(checkpointId, req),
    successMessage: 'Checkpoint reviewed',
    errorMessage: 'Failed to review checkpoint',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useSkipCheckpoint() {
  return useMutationWithToast({
    mutationFn: autonomyApi.skipCheckpoint,
    successMessage: 'Checkpoint skipped',
    errorMessage: 'Failed to skip checkpoint',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useCreateApprovalGate() {
  return useMutationWithToast({
    mutationFn: autonomyApi.createApprovalGate,
    successMessage: 'Approval gate created',
    errorMessage: 'Failed to create approval gate',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useDeleteApprovalGate() {
  return useMutationWithToast({
    mutationFn: autonomyApi.deleteApprovalGate,
    successMessage: 'Approval gate deleted',
    errorMessage: 'Failed to delete approval gate',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useSubmitGateApproval() {
  return useMutationWithToast({
    mutationFn: ({ pendingGateId, ...req }: SubmitApprovalRequest & { pendingGateId: string }) =>
      autonomyApi.submitGateApproval(pendingGateId, req),
    successMessage: 'Approval submitted',
    errorMessage: 'Failed to submit approval',
    invalidateKeys: [autonomyKeys.all],
  });
}

export function useBypassGate() {
  return useMutationWithToast({
    mutationFn: autonomyApi.bypassGate,
    successMessage: 'Gate bypassed',
    errorMessage: 'Failed to bypass gate',
    invalidateKeys: [autonomyKeys.all],
  });
}
