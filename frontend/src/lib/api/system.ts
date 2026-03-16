import type {
  DirectoryListResponse,
  DirectoryEntry,
  Config,
  UserSystemInfo,
  CheckTokenResponse,
  DeviceFlowStartResponse,
  DevicePollStatus,
  TaskTemplate,
  CreateTaskTemplate,
  UpdateTaskTemplate,
  McpServerQuery,
  UpdateMcpServersBody,
  GetMcpServerResponse,
  ImageResponse,
  TaskComment,
  CreateTaskComment,
  ActivityLog,
  CreateActivityLog,
  AgentWallet,
  AgentWalletTransaction,
  UpsertAgentWallet,
  CreateWalletTransaction,
  Task,
  ApprovalResponse,
  ToolApprovalStatus,
  RepositoryInfo,
} from 'shared/types';
import { makeRequest, handleApiResponse, ApiError, resolveApiUrl } from './client';

// File System APIs
export const fileSystemApi = {
  list: async (path?: string): Promise<DirectoryListResponse> => {
    const queryParam = path ? `?path=${encodeURIComponent(path)}` : '';
    const response = await makeRequest(
      `/api/filesystem/directory${queryParam}`
    );
    return handleApiResponse<DirectoryListResponse>(response);
  },

  listGitRepos: async (path?: string): Promise<DirectoryEntry[]> => {
    const queryParam = path ? `?path=${encodeURIComponent(path)}` : '';
    const response = await makeRequest(
      `/api/filesystem/git-repos${queryParam}`
    );
    return handleApiResponse<DirectoryEntry[]>(response);
  },
};

// System Status APIs
export const systemApi = {
  getOrchaStatus: async (): Promise<{
    orchestratorName: string;
    agentId: string;
    device: string;
    isAdmin: boolean;
    subAgents: Array<{ id: string; shortName: string; designation: string; status: string }>;
  } | null> => {
    const response = await makeRequest('/api/orcha/status');
    if (!response.ok) return null;
    return response.json();
  },
};

// Config APIs (backwards compatible)
export const configApi = {
  getConfig: async (): Promise<UserSystemInfo> => {
    const response = await makeRequest('/api/info');
    return handleApiResponse<UserSystemInfo>(response);
  },
  saveConfig: async (config: Config): Promise<Config> => {
    const response = await makeRequest('/api/config', {
      method: 'PUT',
      body: JSON.stringify(config),
    });
    return handleApiResponse<Config>(response);
  },
};

// GitHub Device Auth APIs
export const githubAuthApi = {
  checkGithubToken: async (): Promise<CheckTokenResponse> => {
    const response = await makeRequest('/api/auth/github/check');
    return handleApiResponse<CheckTokenResponse>(response);
  },
  start: async (): Promise<DeviceFlowStartResponse> => {
    const response = await makeRequest('/api/auth/github/device/start', {
      method: 'POST',
    });
    return handleApiResponse<DeviceFlowStartResponse>(response);
  },
  poll: async (): Promise<DevicePollStatus> => {
    const response = await makeRequest('/api/auth/github/device/poll', {
      method: 'POST',
    });
    return handleApiResponse<DevicePollStatus>(response);
  },
};

// GitHub APIs (only available in cloud mode)
export const githubApi = {
  listRepositories: async (page: number = 1): Promise<RepositoryInfo[]> => {
    const response = await makeRequest(`/api/github/repositories?page=${page}`);
    return handleApiResponse<RepositoryInfo[]>(response);
  },
};

// Task Templates APIs
export const templatesApi = {
  list: async (): Promise<TaskTemplate[]> => {
    const response = await makeRequest('/api/templates');
    return handleApiResponse<TaskTemplate[]>(response);
  },

  listGlobal: async (): Promise<TaskTemplate[]> => {
    const response = await makeRequest('/api/templates?global=true');
    return handleApiResponse<TaskTemplate[]>(response);
  },

  listByProject: async (projectId: string): Promise<TaskTemplate[]> => {
    const response = await makeRequest(
      `/api/templates?project_id=${projectId}`
    );
    return handleApiResponse<TaskTemplate[]>(response);
  },

  get: async (templateId: string): Promise<TaskTemplate> => {
    const response = await makeRequest(`/api/templates/${templateId}`);
    return handleApiResponse<TaskTemplate>(response);
  },

  create: async (data: CreateTaskTemplate): Promise<TaskTemplate> => {
    const response = await makeRequest('/api/templates', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<TaskTemplate>(response);
  },

  update: async (
    templateId: string,
    data: UpdateTaskTemplate
  ): Promise<TaskTemplate> => {
    const response = await makeRequest(`/api/templates/${templateId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<TaskTemplate>(response);
  },

  delete: async (templateId: string): Promise<void> => {
    const response = await makeRequest(`/api/templates/${templateId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },
};

// MCP Servers APIs
export const mcpServersApi = {
  load: async (query: McpServerQuery): Promise<GetMcpServerResponse> => {
    const params = new URLSearchParams(query);
    const response = await makeRequest(`/api/mcp-config?${params.toString()}`);
    return handleApiResponse<GetMcpServerResponse>(response);
  },
  save: async (
    query: McpServerQuery,
    data: UpdateMcpServersBody
  ): Promise<void> => {
    const params = new URLSearchParams(query);
    // params.set('profile', profile);
    const response = await makeRequest(`/api/mcp-config?${params.toString()}`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      const errorData = await response.json();
      console.error('[API Error] Failed to save MCP servers', {
        message: errorData.message,
        status: response.status,
        response,
        timestamp: new Date().toISOString(),
      });
      throw new ApiError(
        errorData.message || 'Failed to save MCP servers',
        response.status,
        response
      );
    }
  },
};

// Profiles API
export const profilesApi = {
  load: async (): Promise<{ content: string; path: string }> => {
    const response = await makeRequest('/api/profiles');
    return handleApiResponse<{ content: string; path: string }>(response);
  },
  save: async (content: string): Promise<string> => {
    const response = await makeRequest('/api/profiles', {
      method: 'PUT',
      body: content,
      headers: {
        'Content-Type': 'application/json',
      },
    });
    return handleApiResponse<string>(response);
  },
};

// Agent Wallet API
export const agentWalletApi = {
  list: async (): Promise<AgentWallet[]> => {
    const response = await makeRequest('/api/agent-wallets');
    return handleApiResponse<AgentWallet[]>(response);
  },

  upsert: async (data: UpsertAgentWallet): Promise<AgentWallet> => {
    const response = await makeRequest('/api/agent-wallets', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentWallet>(response);
  },

  update: async (
    profileKey: string,
    data: UpsertAgentWallet
  ): Promise<AgentWallet> => {
    const response = await makeRequest(`/api/agent-wallets/${profileKey}`, {
      method: 'PUT',
      body: JSON.stringify({ ...data, profile_key: profileKey }),
    });
    return handleApiResponse<AgentWallet>(response);
  },

  listTransactions: async (
    profileKey: string,
    limit = 25
  ): Promise<AgentWalletTransaction[]> => {
    const response = await makeRequest(
      `/api/agent-wallets/${profileKey}/transactions?limit=${limit}`
    );
    return handleApiResponse<AgentWalletTransaction[]>(response);
  },

  createTransaction: async (
    profileKey: string,
    data: CreateWalletTransaction
  ): Promise<AgentWalletTransaction> => {
    const response = await makeRequest(
      `/api/agent-wallets/${profileKey}/transactions`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<AgentWalletTransaction>(response);
  },
};

// Images API
export const imagesApi = {
  upload: async (file: File): Promise<ImageResponse> => {
    const formData = new FormData();
    formData.append('image', file);

    const response = await fetch(resolveApiUrl('/api/images/upload'), {
      method: 'POST',
      body: formData,
      credentials: 'include',
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new ApiError(
        `Failed to upload image: ${errorText}`,
        response.status,
        response
      );
    }

    return handleApiResponse<ImageResponse>(response);
  },

  delete: async (imageId: string): Promise<void> => {
    const response = await makeRequest(`/api/images/${imageId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  getTaskImages: async (taskId: string): Promise<ImageResponse[]> => {
    const response = await makeRequest(`/api/images/task/${taskId}`);
    return handleApiResponse<ImageResponse[]>(response);
  },

  getImageUrl: (imageId: string): string => {
    return `/api/images/${imageId}/file`;
  },
};

// Comments API
export const commentsApi = {
  getAll: async (taskId: string): Promise<TaskComment[]> => {
    const response = await makeRequest(`/api/${taskId}/comments`);
    return handleApiResponse<TaskComment[]>(response);
  },

  create: async (comment: CreateTaskComment): Promise<TaskComment> => {
    const response = await makeRequest(`/api/${comment.task_id}/comments`, {
      method: 'POST',
      body: JSON.stringify(comment),
    });
    return handleApiResponse<TaskComment>(response);
  },

  delete: async (commentId: string): Promise<void> => {
    const response = await makeRequest(`/api/comments/${commentId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },
};

// Activity Log API
export const activityApi = {
  getAll: async (taskId: string): Promise<ActivityLog[]> => {
    const response = await makeRequest(`/api/${taskId}/activity`);
    return handleApiResponse<ActivityLog[]>(response);
  },

  create: async (activity: CreateActivityLog): Promise<ActivityLog> => {
    const response = await makeRequest(`/api/${activity.task_id}/activity`, {
      method: 'POST',
      body: JSON.stringify(activity),
    });
    return handleApiResponse<ActivityLog>(response);
  },
};

// Task Approval API
export const taskApprovalApi = {
  approve: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}/approve`, {
      method: 'POST',
    });
    return handleApiResponse<Task>(response);
  },

  requestChanges: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}/request-changes`, {
      method: 'POST',
    });
    return handleApiResponse<Task>(response);
  },

  reject: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}/reject`, {
      method: 'POST',
    });
    return handleApiResponse<Task>(response);
  },
};

// Approval API
export const approvalsApi = {
  respond: async (
    approvalId: string,
    payload: ApprovalResponse,
    signal?: AbortSignal
  ): Promise<ToolApprovalStatus> => {
    const res = await makeRequest(`/api/approvals/${approvalId}/respond`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload),
      signal,
    });

    return handleApiResponse<ToolApprovalStatus>(res);
  },
};
