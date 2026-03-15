import { makeRequest, handleApiResponse } from './client';
import type { RalphLoopState, RalphIteration } from './organizations';

export const ralphApi = {
  getLoopState: async (loopId: string): Promise<RalphLoopState> => {
    const response = await makeRequest(`/api/ralph/loops/${loopId}`);
    return handleApiResponse<RalphLoopState>(response);
  },
  getByAttempt: async (taskAttemptId: string): Promise<RalphLoopState | { type: 'NotFound' }> => {
    const response = await makeRequest(`/api/ralph/by-attempt/${taskAttemptId}`);
    return handleApiResponse<RalphLoopState | { type: 'NotFound' }>(response);
  },
  getIterations: async (loopId: string): Promise<RalphIteration[]> => {
    const response = await makeRequest(`/api/ralph/loops/${loopId}/iterations`);
    return handleApiResponse<RalphIteration[]>(response);
  },
  cancelLoop: async (loopId: string): Promise<void> => {
    const response = await makeRequest(`/api/ralph/loops/${loopId}/cancel`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
};

// ============================================================================
// COMMUNICATIONS (CALLS & SMS)
// ============================================================================

export interface CallLogRecord {
  id: string;
  project_id: string;
  call_sid: string;
  parent_call_sid: string | null;
  from_number: string;
  to_number: string;
  from_formatted: string | null;
  to_formatted: string | null;
  caller_name: string | null;
  direction: string;
  status: string;
  answered_by: string | null;
  start_time: string | null;
  end_time: string | null;
  duration_seconds: number | null;
  recording_url: string | null;
  transcription: string | null;
  summary: string | null;
  sentiment: string | null;
  crm_contact_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface CallStats {
  total: number;
  inbound: number;
  outbound: number;
  completed: number;
  missed: number;
  total_duration_seconds: number;
}

export interface SmsMessageRecord {
  id: string;
  project_id: string;
  message_sid: string;
  from_number: string;
  to_number: string;
  body: string;
  direction: string;
  status: string;
  sentiment: string | null;
  crm_contact_id: string | null;
  is_read: number;
  is_starred: number;
  needs_response: number;
  auto_response: string | null;
  date_sent: string | null;
  created_at: string;
  updated_at: string;
}

export interface SmsStats {
  total: number;
  inbound: number;
  outbound: number;
  unread: number;
  needs_response: number;
}

export const communicationsApi = {
  listCalls: async (params: { project_id: string; limit?: number }): Promise<CallLogRecord[]> => {
    const qs = new URLSearchParams({ project_id: params.project_id, limit: String(params.limit ?? 50) });
    const response = await makeRequest(`/api/communications/calls?${qs}`);
    return handleApiResponse<CallLogRecord[]>(response);
  },
  getCallStats: async (projectId: string): Promise<CallStats> => {
    const response = await makeRequest(`/api/communications/calls/stats/${projectId}`);
    return handleApiResponse<CallStats>(response);
  },
  listSms: async (params: { project_id: string; limit?: number }): Promise<SmsMessageRecord[]> => {
    const qs = new URLSearchParams({ project_id: params.project_id, limit: String(params.limit ?? 50) });
    const response = await makeRequest(`/api/communications/sms?${qs}`);
    return handleApiResponse<SmsMessageRecord[]>(response);
  },
  getSmsStats: async (projectId: string): Promise<SmsStats> => {
    const response = await makeRequest(`/api/communications/sms/stats/${projectId}`);
    return handleApiResponse<SmsStats>(response);
  },
  markSmsRead: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/communications/sms/${id}/read`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
  toggleSmsStar: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/communications/sms/${id}/star`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
};
