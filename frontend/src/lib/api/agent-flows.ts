import { makeRequest, handleApiResponse, resolveApiUrl } from './client';

// ============================================
// Agent Flow APIs
// ============================================

export interface AgentFlow {
  id: string;
  task_id: string;
  flow_type: string;
  status: string;
  current_phase: string;
  planner_agent_id?: string;
  executor_agent_id?: string;
  verifier_agent_id?: string;
  flow_config?: string;
  handoff_instructions?: string;
  planning_started_at?: string;
  planning_completed_at?: string;
  execution_started_at?: string;
  execution_completed_at?: string;
  verification_started_at?: string;
  verification_completed_at?: string;
  verification_score?: number;
  human_approval_required: boolean;
  approved_by?: string;
  approved_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateAgentFlow {
  task_id: string;
  flow_type: string;
  planner_agent_id?: string;
  executor_agent_id?: string;
  verifier_agent_id?: string;
  flow_config?: Record<string, unknown>;
  human_approval_required?: boolean;
}

export interface UpdateAgentFlow {
  status?: string;
  current_phase?: string;
  planner_agent_id?: string;
  executor_agent_id?: string;
  verifier_agent_id?: string;
  handoff_instructions?: string;
  verification_score?: number;
  approved_by?: string;
}

export interface RespondClarification {
  response: string;
  resume_status?: string;
}

export const agentFlowsApi = {
  list: async (params?: {
    task_id?: string;
    status?: string;
  }): Promise<AgentFlow[]> => {
    const searchParams = new URLSearchParams();
    if (params?.task_id) searchParams.set('task_id', params.task_id);
    if (params?.status) searchParams.set('status', params.status);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/agent-flows${query ? `?${query}` : ''}`
    );
    return handleApiResponse<AgentFlow[]>(response);
  },

  getById: async (flowId: string): Promise<AgentFlow> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}`);
    return handleApiResponse<AgentFlow>(response);
  },

  create: async (data: CreateAgentFlow): Promise<AgentFlow> => {
    const response = await makeRequest('/api/agent-flows', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentFlow>(response);
  },

  update: async (flowId: string, data: UpdateAgentFlow): Promise<AgentFlow> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentFlow>(response);
  },

  delete: async (flowId: string): Promise<void> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  transitionPhase: async (
    flowId: string,
    phase: string
  ): Promise<AgentFlow> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}/transition`, {
      method: 'POST',
      body: JSON.stringify({ phase }),
    });
    return handleApiResponse<AgentFlow>(response);
  },

  complete: async (
    flowId: string,
    verificationScore?: number
  ): Promise<AgentFlow> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}/complete`, {
      method: 'POST',
      body: JSON.stringify({ verification_score: verificationScore }),
    });
    return handleApiResponse<AgentFlow>(response);
  },

  requestApproval: async (flowId: string): Promise<AgentFlow> => {
    const response = await makeRequest(
      `/api/agent-flows/${flowId}/request-approval`,
      { method: 'POST' }
    );
    return handleApiResponse<AgentFlow>(response);
  },

  approve: async (flowId: string, approvedBy: string): Promise<AgentFlow> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ approved_by: approvedBy }),
    });
    return handleApiResponse<AgentFlow>(response);
  },

  respondClarification: async (
    flowId: string,
    data: RespondClarification
  ): Promise<AgentFlow> => {
    const response = await makeRequest(
      `/api/agent-flows/${flowId}/respond-clarification`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<AgentFlow>(response);
  },

  listAwaitingApproval: async (): Promise<AgentFlow[]> => {
    const response = await makeRequest('/api/agent-flows/awaiting-approval');
    return handleApiResponse<AgentFlow[]>(response);
  },

  getEvents: async (
    flowId: string,
    params?: { since?: string; event_type?: string }
  ): Promise<AgentFlowEvent[]> => {
    const searchParams = new URLSearchParams();
    if (params?.since) searchParams.set('since', params.since);
    if (params?.event_type) searchParams.set('event_type', params.event_type);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/agent-flows/${flowId}/events${query ? `?${query}` : ''}`
    );
    return handleApiResponse<AgentFlowEvent[]>(response);
  },

  createEvent: async (
    flowId: string,
    eventType: string,
    eventData: Record<string, unknown>
  ): Promise<AgentFlowEvent> => {
    const response = await makeRequest(`/api/agent-flows/${flowId}/events`, {
      method: 'POST',
      body: JSON.stringify({ event_type: eventType, event_data: eventData }),
    });
    return handleApiResponse<AgentFlowEvent>(response);
  },

  streamEvents: (flowId: string): EventSource => {
    return new EventSource(resolveApiUrl(`/api/agent-flows/${flowId}/events/stream`));
  },
};

export interface AgentFlowEvent {
  id: string;
  agent_flow_id: string;
  event_type: string;
  event_data?: string;
  created_at: string;
}

// ============================================
// Wide Research APIs
// ============================================

export interface WideResearchSession {
  id: string;
  agent_flow_id?: string;
  parent_agent_id?: string;
  task_description: string;
  total_subagents: number;
  completed_count: number;
  failed_count: number;
  parallelism_limit: number;
  timeout_per_subagent?: number;
  status: string;
  aggregated_result_artifact_id?: string;
  created_at: string;
  updated_at: string;
}

export interface WideResearchSubagent {
  id: string;
  session_id: string;
  subagent_index: number;
  target_item: string;
  metadata?: string;
  status: string;
  execution_process_id?: string;
  result_artifact_id?: string;
  error_message?: string;
  started_at?: string;
  completed_at?: string;
  created_at: string;
}

export interface CreateWideResearchSession {
  agent_flow_id?: string;
  parent_agent_id?: string;
  task_description: string;
  targets: Array<{ target_item: string; metadata?: Record<string, unknown> }>;
  parallelism_limit?: number;
  timeout_per_subagent?: number;
}

export interface SessionWithSubagents {
  session: WideResearchSession;
  subagents: WideResearchSubagent[];
  progress_percent: number;
}

export const wideResearchApi = {
  list: async (params?: {
    agent_flow_id?: string;
    status?: string;
  }): Promise<WideResearchSession[]> => {
    const searchParams = new URLSearchParams();
    if (params?.agent_flow_id)
      searchParams.set('agent_flow_id', params.agent_flow_id);
    if (params?.status) searchParams.set('status', params.status);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/wide-research${query ? `?${query}` : ''}`
    );
    return handleApiResponse<WideResearchSession[]>(response);
  },

  getById: async (sessionId: string): Promise<SessionWithSubagents> => {
    const response = await makeRequest(`/api/wide-research/${sessionId}`);
    return handleApiResponse<SessionWithSubagents>(response);
  },

  create: async (
    data: CreateWideResearchSession
  ): Promise<SessionWithSubagents> => {
    const response = await makeRequest('/api/wide-research', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<SessionWithSubagents>(response);
  },

  delete: async (sessionId: string): Promise<void> => {
    const response = await makeRequest(`/api/wide-research/${sessionId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  getSubagents: async (sessionId: string): Promise<WideResearchSubagent[]> => {
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/subagents`
    );
    return handleApiResponse<WideResearchSubagent[]>(response);
  },

  getNextPending: async (
    sessionId: string,
    limit?: number
  ): Promise<WideResearchSubagent[]> => {
    const query = limit ? `?limit=${limit}` : '';
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/subagents/next${query}`
    );
    return handleApiResponse<WideResearchSubagent[]>(response);
  },

  startSubagent: async (
    sessionId: string,
    subagentId: string,
    executionProcessId: string
  ): Promise<WideResearchSubagent> => {
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/subagents/${subagentId}/start`,
      {
        method: 'POST',
        body: JSON.stringify({ execution_process_id: executionProcessId }),
      }
    );
    return handleApiResponse<WideResearchSubagent>(response);
  },

  completeSubagent: async (
    sessionId: string,
    subagentId: string,
    resultArtifactId: string
  ): Promise<WideResearchSubagent> => {
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/subagents/${subagentId}/complete`,
      {
        method: 'POST',
        body: JSON.stringify({ result_artifact_id: resultArtifactId }),
      }
    );
    return handleApiResponse<WideResearchSubagent>(response);
  },

  failSubagent: async (
    sessionId: string,
    subagentId: string,
    errorMessage: string
  ): Promise<WideResearchSubagent> => {
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/subagents/${subagentId}/fail`,
      {
        method: 'POST',
        body: JSON.stringify({ error_message: errorMessage }),
      }
    );
    return handleApiResponse<WideResearchSubagent>(response);
  },

  updateStatus: async (
    sessionId: string,
    status: string
  ): Promise<WideResearchSession> => {
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/status`,
      {
        method: 'POST',
        body: JSON.stringify({ status }),
      }
    );
    return handleApiResponse<WideResearchSession>(response);
  },

  setAggregatedResult: async (
    sessionId: string,
    artifactId: string
  ): Promise<WideResearchSession> => {
    const response = await makeRequest(
      `/api/wide-research/${sessionId}/aggregated-result`,
      {
        method: 'POST',
        body: JSON.stringify({ artifact_id: artifactId }),
      }
    );
    return handleApiResponse<WideResearchSession>(response);
  },
};
