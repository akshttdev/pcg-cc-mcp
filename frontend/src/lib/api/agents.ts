import type {
  AgentWithParsedFields,
  AgentStatus,
  CreateAgent,
  UpdateAgent,
  AgentChatRequest,
  AgentChatResponse,
  ConversationSummary,
  AgentWatcherInfo,
} from 'shared/types';
import { makeRequest, handleApiResponse, ApiError } from './client';

// Agent search query params
export interface AgentSearchParams {
  q?: string;
  status?: AgentStatus;
  capability?: string;
  sort_by?: 'name' | 'designation' | 'status' | 'priority' | 'tasks_completed';
  sort_dir?: 'asc' | 'desc';
}

export const agentsApi = {
  // List all agents
  list: async (): Promise<AgentWithParsedFields[]> => {
    const response = await makeRequest('/api/agents');
    if (!response.ok) {
      throw new ApiError('Failed to load agents', response.status, response);
    }
    return response.json();
  },

  // Search agents with filters
  search: async (params: AgentSearchParams): Promise<AgentWithParsedFields[]> => {
    const searchParams = new URLSearchParams();
    if (params.q) searchParams.set('q', params.q);
    if (params.status) searchParams.set('status', params.status);
    if (params.capability) searchParams.set('capability', params.capability);
    if (params.sort_by) searchParams.set('sort_by', params.sort_by);
    if (params.sort_dir) searchParams.set('sort_dir', params.sort_dir);

    const query = searchParams.toString() ? `?${searchParams.toString()}` : '';
    const response = await makeRequest(`/api/agents/search${query}`);
    if (!response.ok) {
      throw new ApiError('Failed to search agents', response.status, response);
    }
    const body = await response.json();
    return body.data ?? body;
  },

  // List active agents only
  listActive: async (): Promise<AgentWithParsedFields[]> => {
    const response = await makeRequest('/api/agents/active');
    if (!response.ok) {
      throw new ApiError('Failed to load active agents', response.status, response);
    }
    const body = await response.json();
    return body.data ?? body;
  },

  // Get agent by ID
  getById: async (agentId: string): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}`);
    if (!response.ok) {
      throw new ApiError('Failed to load agent', response.status, response);
    }
    return response.json();
  },

  // Get agent by name
  getByName: async (name: string): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/by-name/${encodeURIComponent(name)}`);
    if (!response.ok) {
      throw new ApiError('Failed to load agent', response.status, response);
    }
    return response.json();
  },

  // Create a new agent
  create: async (agent: CreateAgent): Promise<AgentWithParsedFields> => {
    const response = await makeRequest('/api/agents', {
      method: 'POST',
      body: JSON.stringify(agent),
    });
    if (!response.ok) {
      throw new ApiError('Failed to create agent', response.status, response);
    }
    return response.json();
  },

  // Update an agent
  update: async (agentId: string, agent: UpdateAgent): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}`, {
      method: 'PUT',
      body: JSON.stringify(agent),
    });
    if (!response.ok) {
      throw new ApiError('Failed to update agent', response.status, response);
    }
    return response.json();
  },

  // Delete an agent
  delete: async (agentId: string): Promise<void> => {
    const response = await makeRequest(`/api/agents/${agentId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      throw new ApiError('Failed to delete agent', response.status, response);
    }
  },

  // Seed core agents (Nora, Maci, Editron)
  seedCoreAgents: async (): Promise<AgentWithParsedFields[]> => {
    const response = await makeRequest('/api/agents/seed', {
      method: 'POST',
    });
    if (!response.ok) {
      throw new ApiError('Failed to seed agents', response.status, response);
    }
    return response.json();
  },

  // Update agent status
  updateStatus: async (agentId: string, status: AgentStatus): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status }),
    });
    if (!response.ok) {
      throw new ApiError('Failed to update agent status', response.status, response);
    }
    return response.json();
  },

  // Assign wallet to agent
  assignWallet: async (agentId: string, walletAddress: string): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}/wallet`, {
      method: 'PUT',
      body: JSON.stringify({ wallet_address: walletAddress }),
    });
    if (!response.ok) {
      throw new ApiError('Failed to assign wallet', response.status, response);
    }
    return response.json();
  },

  // Chat with an agent
  chat: async (agentId: string, request: AgentChatRequest): Promise<AgentChatResponse> => {
    const response = await makeRequest(`/api/agents/${agentId}/chat`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
    if (!response.ok) {
      throw new ApiError('Failed to chat with agent', response.status, response);
    }
    return response.json();
  },

  // Chat with an agent (streaming)
  chatStream: async (
    agentId: string,
    request: AgentChatRequest,
    onChunk: (chunk: string) => void,
    onComplete?: (fullResponse: string) => void
  ): Promise<void> => {
    const response = await fetch(`/api/agents/${agentId}/chat/stream`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ...request, stream: true }),
    });

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const reader = response.body?.getReader();
    if (!reader) {
      throw new Error('No response body');
    }

    const decoder = new TextDecoder();
    let fullResponse = '';

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const text = decoder.decode(value, { stream: true });
      const lines = text.split('\n');

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const data = line.slice(6);
          if (data === '[DONE]') {
            if (onComplete) onComplete(fullResponse);
            return;
          }
          try {
            const parsed = JSON.parse(data);
            if (parsed.content) {
              fullResponse += parsed.content;
              onChunk(parsed.content);
            }
          } catch {
            // Ignore parse errors for incomplete chunks
          }
        }
      }
    }

    if (onComplete) onComplete(fullResponse);
  },

  // List agent conversations
  listConversations: async (
    agentId: string,
    projectId?: string,
    limit?: number
  ): Promise<ConversationSummary[]> => {
    const params = new URLSearchParams();
    if (projectId) params.set('projectId', projectId);
    if (limit) params.set('limit', limit.toString());
    const query = params.toString() ? `?${params.toString()}` : '';
    const response = await makeRequest(`/api/agents/${agentId}/conversations${query}`);
    return handleApiResponse<ConversationSummary[]>(response);
  },

  // Get conversation messages
  getConversationMessages: async (
    agentId: string,
    conversationId: string
  ): Promise<{ id: string; role: string; content: string; createdAt: string }[]> => {
    const response = await makeRequest(`/api/agents/${agentId}/conversations/${conversationId}/messages`);
    return handleApiResponse(response);
  },

  // Get conversation by session ID (with messages)
  getConversationBySession: async (
    agentId: string,
    sessionId: string
  ): Promise<{
    conversation: { id: string; sessionId: string; messageCount: number };
    messages: { id: string; role: string; content: string; createdAt: string }[];
  } | null> => {
    const response = await makeRequest(
      `/api/agents/${agentId}/conversations/session/${encodeURIComponent(sessionId)}`
    );
    return handleApiResponse(response);
  },
};

// Agent Watcher APIs — type auto-generated from Rust via `npm run generate-types`
export type { AgentWatcherInfo };

export const agentWatchersApi = {
  list: async (taskId: string): Promise<AgentWatcherInfo[]> => {
    const response = await makeRequest(`/api/tasks/${taskId}/agent-watchers`);
    if (!response.ok) {
      throw new ApiError('Failed to list agent watchers', response.status, response);
    }
    const result = await response.json();
    return result.data;
  },

  add: async (taskId: string, agentId: string): Promise<void> => {
    const response = await makeRequest(`/api/tasks/${taskId}/agent-watchers`, {
      method: 'POST',
      body: JSON.stringify({ agent_id: agentId }),
    });
    if (!response.ok) {
      throw new ApiError('Failed to add agent watcher', response.status, response);
    }
  },

  remove: async (taskId: string, agentId: string): Promise<void> => {
    const response = await makeRequest(`/api/tasks/${taskId}/agent-watchers/${agentId}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      throw new ApiError('Failed to remove agent watcher', response.status, response);
    }
  },
};
