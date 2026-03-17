import { makeRequest, handleApiResponse } from './client';
import type {
  CheckpointDefinition,
  ExecutionCheckpoint,
  ApprovalGate,
  PendingGate,
  GateApproval,
  PendingApprovalsSummary,
  CreateCheckpointDefinitionRequest,
  UpdateCheckpointDefinitionRequest,
  ReviewCheckpointRequest,
  CreateApprovalGateRequest,
  SubmitApprovalRequest,
  AutonomyMode,
} from '@/hooks/useAutonomy';

export const autonomyApi = {
  // Autonomy mode
  getTaskMode: (taskId: string): Promise<AutonomyMode> =>
    makeRequest(`/api/tasks/${taskId}/autonomy-mode`).then(handleApiResponse<AutonomyMode>),

  setTaskMode: (taskId: string, mode: AutonomyMode): Promise<void> =>
    makeRequest(`/api/tasks/${taskId}/autonomy-mode`, {
      method: 'PUT',
      body: JSON.stringify({ mode }),
    }).then(handleApiResponse<void>),

  // Checkpoint definitions
  listCheckpointDefinitions: (projectId: string): Promise<CheckpointDefinition[]> =>
    makeRequest(`/api/projects/${projectId}/checkpoint-definitions`).then(handleApiResponse<CheckpointDefinition[]>),

  createCheckpointDefinition: (req: CreateCheckpointDefinitionRequest): Promise<CheckpointDefinition> =>
    makeRequest('/api/autonomy/checkpoint-definitions', {
      method: 'POST',
      body: JSON.stringify(req),
    }).then(handleApiResponse<CheckpointDefinition>),

  updateCheckpointDefinition: (definitionId: string, req: UpdateCheckpointDefinitionRequest): Promise<CheckpointDefinition> =>
    makeRequest(`/api/autonomy/checkpoint-definitions/${definitionId}`, {
      method: 'PUT',
      body: JSON.stringify(req),
    }).then(handleApiResponse<CheckpointDefinition>),

  deleteCheckpointDefinition: (definitionId: string): Promise<void> =>
    makeRequest(`/api/autonomy/checkpoint-definitions/${definitionId}`, {
      method: 'DELETE',
    }).then(handleApiResponse<void>),

  // Execution checkpoints
  listExecutionCheckpoints: (executionId: string): Promise<ExecutionCheckpoint[]> =>
    makeRequest(`/api/executions/${executionId}/checkpoints`).then(handleApiResponse<ExecutionCheckpoint[]>),

  listPendingCheckpoints: (executionId: string): Promise<ExecutionCheckpoint[]> =>
    makeRequest(`/api/executions/${executionId}/checkpoints/pending`).then(handleApiResponse<ExecutionCheckpoint[]>),

  reviewCheckpoint: (checkpointId: string, req: ReviewCheckpointRequest): Promise<ExecutionCheckpoint> =>
    makeRequest(`/api/checkpoints/${checkpointId}/review`, {
      method: 'POST',
      body: JSON.stringify(req),
    }).then(handleApiResponse<ExecutionCheckpoint>),

  skipCheckpoint: (checkpointId: string): Promise<ExecutionCheckpoint> =>
    makeRequest(`/api/checkpoints/${checkpointId}/skip`, {
      method: 'POST',
    }).then(handleApiResponse<ExecutionCheckpoint>),

  // Approval gates
  listProjectGates: (projectId: string): Promise<ApprovalGate[]> =>
    makeRequest(`/api/projects/${projectId}/approval-gates`).then(handleApiResponse<ApprovalGate[]>),

  createApprovalGate: (req: CreateApprovalGateRequest): Promise<ApprovalGate> =>
    makeRequest('/api/autonomy/approval-gates', {
      method: 'POST',
      body: JSON.stringify(req),
    }).then(handleApiResponse<ApprovalGate>),

  deleteApprovalGate: (gateId: string): Promise<void> =>
    makeRequest(`/api/autonomy/approval-gates/${gateId}`, {
      method: 'DELETE',
    }).then(handleApiResponse<void>),

  // Pending gates
  listPendingGates: (executionId: string): Promise<PendingGate[]> =>
    makeRequest(`/api/executions/${executionId}/gates`).then(handleApiResponse<PendingGate[]>),

  submitGateApproval: (pendingGateId: string, req: SubmitApprovalRequest): Promise<GateApproval> =>
    makeRequest(`/api/pending-gates/${pendingGateId}/approve`, {
      method: 'POST',
      body: JSON.stringify(req),
    }).then(handleApiResponse<GateApproval>),

  bypassGate: (pendingGateId: string): Promise<PendingGate> =>
    makeRequest(`/api/pending-gates/${pendingGateId}/bypass`, {
      method: 'POST',
    }).then(handleApiResponse<PendingGate>),

  // Summary
  getPendingApprovalsSummary: (): Promise<PendingApprovalsSummary> =>
    makeRequest('/api/autonomy/pending-approvals').then(handleApiResponse<PendingApprovalsSummary>),

  canProceed: (executionId: string): Promise<boolean> =>
    makeRequest(`/api/executions/${executionId}/can-proceed`).then(handleApiResponse<boolean>),
};
