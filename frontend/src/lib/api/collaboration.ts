import { makeRequest, handleApiResponse } from './client';
import type {
  CollaborationState,
  ExecutionPauseHistory,
  ExecutionHandoff,
  ContextInjection,
  InjectionType,
} from '@/hooks/useCollaboration';

export const collaborationApi = {
  // State
  getState: (executionId: string): Promise<CollaborationState> =>
    makeRequest(`/api/executions/${executionId}/collaboration`).then(handleApiResponse<CollaborationState>),

  // Pause / Resume
  getPauseHistory: (executionId: string): Promise<ExecutionPauseHistory[]> =>
    makeRequest(`/api/executions/${executionId}/pause-history`).then(handleApiResponse<ExecutionPauseHistory[]>),

  pause: (executionId: string, data: {
    reason?: string;
    initiated_by: string;
    initiated_by_name?: string;
  }): Promise<ExecutionPauseHistory> =>
    makeRequest(`/api/executions/${executionId}/pause`, {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<ExecutionPauseHistory>),

  resume: (executionId: string, data: {
    initiated_by: string;
    initiated_by_name?: string;
  }): Promise<ExecutionPauseHistory> =>
    makeRequest(`/api/executions/${executionId}/resume`, {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<ExecutionPauseHistory>),

  // Handoffs
  listHandoffs: (executionId: string): Promise<ExecutionHandoff[]> =>
    makeRequest(`/api/executions/${executionId}/handoffs`).then(handleApiResponse<ExecutionHandoff[]>),

  takeover: (executionId: string, data: {
    human_id: string;
    human_name?: string;
    reason?: string;
  }): Promise<ExecutionHandoff> =>
    makeRequest(`/api/executions/${executionId}/takeover`, {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<ExecutionHandoff>),

  returnControl: (executionId: string, data: {
    human_id: string;
    human_name?: string;
    to_agent_id: string;
    to_agent_name?: string;
    context_notes?: string;
  }): Promise<ExecutionHandoff> =>
    makeRequest(`/api/executions/${executionId}/return-control`, {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<ExecutionHandoff>),

  // Injections
  listInjections: (executionId: string): Promise<ContextInjection[]> =>
    makeRequest(`/api/executions/${executionId}/injections`).then(handleApiResponse<ContextInjection[]>),

  listPendingInjections: (executionId: string): Promise<ContextInjection[]> =>
    makeRequest(`/api/executions/${executionId}/injections/pending`).then(handleApiResponse<ContextInjection[]>),

  inject: (executionId: string, data: {
    injector_id: string;
    injector_name?: string;
    injection_type: InjectionType;
    content: string;
    metadata?: Record<string, unknown>;
  }): Promise<ContextInjection> =>
    makeRequest(`/api/executions/${executionId}/inject`, {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<ContextInjection>),

  acknowledgeInjection: (injectionId: string): Promise<ContextInjection> =>
    makeRequest(`/api/injections/${injectionId}/acknowledge`, {
      method: 'POST',
    }).then(handleApiResponse<ContextInjection>),

  acknowledgeAll: (executionId: string): Promise<number> =>
    makeRequest(`/api/executions/${executionId}/injections/acknowledge-all`, {
      method: 'POST',
    }).then(handleApiResponse<number>),
};
