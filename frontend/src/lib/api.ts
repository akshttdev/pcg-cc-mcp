// Import all necessary types from shared types

import {
  ApiResponse,
  BranchStatus,
  BrandProfile,
  CheckTokenResponse,
  Config,
  CommitInfo,
  CreateFollowUpAttempt,
  CreateGitHubPrRequest,
  CreateTask,
  CreateAndStartTaskRequest,
  CreateTaskAttemptBody,
  CreateTaskTemplate,
  DeviceFlowStartResponse,
  DevicePollStatus,
  DirectoryListResponse,
  DirectoryEntry,
  EditorType,
  ExecutionProcess,
  ExecutionSummary,
  GitBranch,
  Project,
  ProjectBoard,
  CreateProjectBoard,
  UpdateProjectBoard,
  ProjectAsset,
  CreateProjectAsset,
  UpdateProjectAsset,
  CreateProject,
  UpsertBrandProfile,
  RebaseTaskAttemptRequest,
  RepositoryInfo,
  SearchResult,
  Task,
  TaskAttempt,
  TaskRelationships,
  TaskTemplate,
  TaskWithAttemptStatus,
  UpdateProject,
  UpdateTask,
  UpdateTaskTemplate,
  UserSystemInfo,
  GitHubServiceError,
  McpServerQuery,
  UpdateMcpServersBody,
  GetMcpServerResponse,
  ImageResponse,
  FollowUpDraftResponse,
  UpdateFollowUpDraftRequest,
  GitOperationError,
  ApprovalResponse,
  ToolApprovalStatus,
  TaskComment,
  CreateTaskComment,
  ActivityLog,
  CreateActivityLog,
  AgentWallet,
  AgentWalletTransaction,
  UpsertAgentWallet,
  CreateWalletTransaction,
  GraphPlan,
  GraphPlanSummary,
  GraphNodeStatus,
  AgentWithParsedFields,
  AgentStatus,
  CreateAgent,
  UpdateAgent,
  AgentChatRequest,
  AgentChatResponse,
  ConversationSummary,
  // Airtable integration types
  AirtableBase,
  CreateAirtableBase,
  UpdateAirtableBase,
  AirtableRecordLink,
  AirtableBaseInfo,
  AirtableTable,
  AirtableRecord,
  AirtableConnectionWithBase,
  AirtableVerifyRequest,
  AirtableVerifyResponse,
  AirtableImportRequest,
  AirtableImportResult,
  AirtablePushTaskRequest,
} from 'shared/types';

// Extend TaskWithAttemptStatus with archived_at (frontend feature, not yet in DB/backend)
export type TaskWithArchive = TaskWithAttemptStatus & { archived_at?: string | null };

// CRM Pipeline & Deal Types
import type {
  CrmPipeline,
  CrmPipelineStage,
  CrmPipelineWithStages,
  CreateCrmPipeline,
  UpdateCrmPipeline,
  CreateCrmPipelineStage,
  UpdateCrmPipelineStage,
  KanbanBoardData,
  CrmDealRecord,
  CreateCrmDeal,
  UpdateCrmDeal,
  MoveDealRequest,
  PipelineType,
} from '@/types/crm';

// Re-export types for convenience
export type { RepositoryInfo } from 'shared/types';
export type {
  FollowUpDraftResponse,
  UpdateFollowUpDraftRequest,
} from 'shared/types';
export type { ProjectBoard, ProjectBoardType } from 'shared/types';
export type { BrandProfile, UpsertBrandProfile } from 'shared/types';
export type { AgentChatRequest, AgentChatResponse, ConversationSummary } from 'shared/types';

export interface NoraModeSummary {
  id: string;
  label: string;
  description: string;
}

export interface RapidPlaybookResult {
  summary: string;
  created_project: boolean;
  created_message?: string | null;
  projects_synced: number;
}

class ApiError<E = unknown> extends Error {
  public status?: number;
  public error_data?: E;

  constructor(
    message: string,
    public statusCode?: number,
    public response?: Response,
    error_data?: E
  ) {
    super(message);
    this.name = 'ApiError';
    this.status = statusCode;
    this.error_data = error_data;
  }
}

const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
const API_BASE = isTauri
  ? `http://localhost:${(window as any).__ORCHA_BACKEND_PORT__ || 58297}`
  : '';

/**
 * Resolve an API path (e.g. "/api/events/all") to a full URL.
 * In Tauri mode, prepends the backend origin; in web mode, returns the path as-is.
 */
export function resolveApiUrl(path: string): string {
  return path.startsWith('/') ? `${API_BASE}${path}` : path;
}

/**
 * Resolve a WebSocket path (e.g. "/api/ws") to a full ws:// URL.
 * In Tauri mode, uses the backend port; in web mode, uses window.location.
 */
export function resolveWsUrl(path: string): string {
  if (isTauri) {
    const port = (window as any).__ORCHA_BACKEND_PORT__ || 58297;
    return `ws://localhost:${port}${path}`;
  }
  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${window.location.host}${path}`;
}

export const makeRequest = async (url: string, options: RequestInit = {}) => {
  // In Tauri mode, cookies don't work cross-origin (tauri:// → http://localhost).
  // Send the session_id as a Bearer token instead.
  const authHeaders: Record<string, string> = {};
  if (isTauri) {
    const sessionId = localStorage.getItem('session_id');
    if (sessionId) {
      authHeaders['Authorization'] = `Bearer ${sessionId}`;
    }
  }

  const headers = {
    'Content-Type': 'application/json',
    ...authHeaders,
    ...(options.headers || {}),
  };

  const isFileProtocol =
    typeof window !== 'undefined' && window.location.protocol === 'file:';

  if (isFileProtocol && !isTauri) {
    throw new Error(
      'The PCG CC dashboard assets were opened directly from disk. Please start the PCG CC server (`node npx-cli/bin/cli.js`) and access it via the URL printed in the terminal.'
    );
  }

  const resolvedUrl = url.startsWith('/') ? `${API_BASE}${url}` : url;

  try {
    return await fetch(resolvedUrl, {
      ...options,
      headers,
      credentials: 'include',
    });
  } catch (error) {
    if (typeof window !== 'undefined' && error instanceof TypeError) {
      throw new Error(
        `Failed to reach the PCG CC server at ${API_BASE || window.location.origin}. Make sure the server process is running and reachable.`
      );
    }

    throw error;
  }
};

export interface FollowUpResponse {
  message: string;
  actual_attempt_id: string;
  created_new_attempt: boolean;
}

export type Ok<T> = { success: true; data: T };
export type Err<E> = { success: false; error: E | undefined; message?: string };

// Result type for endpoints that need typed errors
export type Result<T, E> = Ok<T> | Err<E>;

// Special handler for Result-returning endpoints
const handleApiResponseAsResult = async <T, E>(
  response: Response
): Promise<Result<T, E>> => {
  if (!response.ok) {
    if (response.status === 401) {
      localStorage.removeItem('session_id');
      document.cookie = 'session_id=; Path=/; Max-Age=0';
      window.location.href = '/login?expired=1';
    }
    // HTTP error - no structured error data
    let errorMessage = `Request failed with status ${response.status}`;

    try {
      const errorData = await response.json();
      if (errorData.message) {
        errorMessage = errorData.message;
      }
    } catch {
      errorMessage = response.statusText || errorMessage;
    }

    return {
      success: false,
      error: undefined,
      message: errorMessage,
    };
  }

  const result: ApiResponse<T, E> = await response.json();

  if (!result.success) {
    return {
      success: false,
      error: result.error_data || undefined,
      message: result.message || undefined,
    };
  }

  return { success: true, data: result.data as T };
};

const handleApiResponse = async <T, E = T>(response: Response): Promise<T> => {
  if (!response.ok) {
    if (response.status === 401) {
      localStorage.removeItem('session_id');
      document.cookie = 'session_id=; Path=/; Max-Age=0';
      window.location.href = '/login?expired=1';
      throw new Error('Session expired');
    }
    let errorMessage = `Request failed with status ${response.status}`;

    try {
      const errorData = await response.json();
      if (errorData.message) {
        errorMessage = errorData.message;
      }
    } catch {
      // Fallback to status text if JSON parsing fails
      errorMessage = response.statusText || errorMessage;
    }

    console.error('[API Error]', {
      message: errorMessage,
      status: response.status,
      response,
      endpoint: response.url,
      timestamp: new Date().toISOString(),
    });
    throw new ApiError<E>(errorMessage, response.status, response);
  }

  const result: ApiResponse<T, E> = await response.json();

  if (!result.success) {
    // Check for error_data first (structured errors), then fall back to message
    if (result.error_data) {
      console.error('[API Error with data]', {
        error_data: result.error_data,
        message: result.message,
        status: response.status,
        response,
        endpoint: response.url,
        timestamp: new Date().toISOString(),
      });
      // Throw a properly typed error with the error data
      throw new ApiError<E>(
        result.message || 'API request failed',
        response.status,
        response,
        result.error_data
      );
    }

    console.error('[API Error]', {
      message: result.message || 'API request failed',
      status: response.status,
      response,
      endpoint: response.url,
      timestamp: new Date().toISOString(),
    });
    throw new ApiError<E>(
      result.message || 'API request failed',
      response.status,
      response
    );
  }

  return result.data as T;
};

export const syncNoraContext = async () => {
  const response = await makeRequest('/api/nora/context/sync', {
    method: 'POST',
  });
  if (!response.ok) {
    throw new ApiError('Failed to sync Nora context', response.status, response);
  }
  return (await response.json()) as { projects_refreshed: number };
};

export const fetchNoraModes = async (): Promise<NoraModeSummary[]> => {
  const response = await makeRequest('/api/nora/modes');
  if (!response.ok) {
    throw new ApiError('Failed to load Nora modes', response.status, response);
  }
  return (await response.json()) as NoraModeSummary[];
};

export const applyNoraMode = async (
  modeId: string,
  preserveMemory = true
): Promise<{ active_mode: string; nora_id: string }> => {
  const response = await makeRequest('/api/nora/modes/apply', {
    method: 'POST',
    body: JSON.stringify({ mode_id: modeId, preserve_memory: preserveMemory }),
  });
  if (!response.ok) {
    throw new ApiError('Failed to apply Nora mode', response.status, response);
  }
  return (await response.json()) as { active_mode: string; nora_id: string };
};

export interface RapidPlaybookPayload {
  project_name: string;
  objectives: string[];
  repo_hint?: string;
  notes?: string;
}

export const runRapidPlaybook = async (
  payload: RapidPlaybookPayload
): Promise<RapidPlaybookResult> => {
  const response = await makeRequest('/api/nora/playbooks/rapid', {
    method: 'POST',
    body: JSON.stringify(payload),
  });
  if (!response.ok) {
    throw new ApiError('Failed to run rapid playbook', response.status, response);
  }
  return (await response.json()) as RapidPlaybookResult;
};

export const fetchNoraPlans = async (): Promise<GraphPlanSummary[]> => {
  const response = await makeRequest('/api/nora/graph/plans');
  if (!response.ok) {
    throw new ApiError('Failed to load orchestration plans', response.status, response);
  }
  return (await response.json()) as GraphPlanSummary[];
};

export const fetchNoraPlan = async (planId: string): Promise<GraphPlan> => {
  const response = await makeRequest(`/api/nora/graph/plans/${planId}`);
  if (!response.ok) {
    throw new ApiError('Failed to load plan detail', response.status, response);
  }
  return (await response.json()) as GraphPlan;
};

export const updateNoraPlanNode = async (
  planId: string,
  nodeId: string,
  status: GraphNodeStatus
): Promise<GraphPlan> => {
  const response = await makeRequest(
    `/api/nora/graph/plans/${planId}/nodes/${nodeId}`,
    {
      method: 'PATCH',
      body: JSON.stringify({ status }),
    }
  );
  if (!response.ok) {
    throw new ApiError('Failed to update node status', response.status, response);
  }
  return (await response.json()) as GraphPlan;
};

type ProjectBoardCreateInput = Omit<CreateProjectBoard, 'project_id'>;
type ProjectBoardUpdateInput = UpdateProjectBoard;
type ProjectAssetCreateInput = Omit<CreateProjectAsset, 'project_id'>;
type ProjectAssetUpdateInput = UpdateProjectAsset;

// Project Management APIs
export const projectsApi = {
  getAll: async (): Promise<Project[]> => {
    const response = await makeRequest('/api/projects');
    return handleApiResponse<Project[]>(response);
  },

  getById: async (id: string): Promise<Project> => {
    const response = await makeRequest(`/api/projects/${id}`);
    return handleApiResponse<Project>(response);
  },

  getByClientId: async (clientId: string): Promise<Project[]> => {
    const response = await makeRequest(`/api/projects/by-client/${encodeURIComponent(clientId)}`);
    return handleApiResponse<Project[]>(response);
  },

  create: async (data: CreateProject): Promise<Project> => {
    const response = await makeRequest('/api/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<Project>(response);
  },

  update: async (id: string, data: UpdateProject): Promise<Project> => {
    const response = await makeRequest(`/api/projects/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<Project>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/projects/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  openEditor: async (id: string, editorType?: EditorType): Promise<void> => {
    const requestBody: any = {};
    if (editorType) requestBody.editor_type = editorType;

    const response = await makeRequest(`/api/projects/${id}/open-editor`, {
      method: 'POST',
      body: JSON.stringify(
        Object.keys(requestBody).length > 0 ? requestBody : null
      ),
    });
    return handleApiResponse<void>(response);
  },

  getBranches: async (id: string): Promise<GitBranch[]> => {
    const response = await makeRequest(`/api/projects/${id}/branches`);
    return handleApiResponse<GitBranch[]>(response);
  },

  searchFiles: async (
    id: string,
    query: string,
    mode?: string,
    options?: RequestInit
  ): Promise<SearchResult[]> => {
    const modeParam = mode ? `&mode=${encodeURIComponent(mode)}` : '';
    const response = await makeRequest(
      `/api/projects/${id}/search?q=${encodeURIComponent(query)}${modeParam}`,
      options
    );
    return handleApiResponse<SearchResult[]>(response);
  },

  listBoards: async (projectId: string): Promise<ProjectBoard[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/boards`);
    return handleApiResponse<ProjectBoard[]>(response);
  },

  createBoard: async (
    projectId: string,
    data: ProjectBoardCreateInput
  ): Promise<ProjectBoard> => {
    const response = await makeRequest(`/api/projects/${projectId}/boards`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProjectBoard>(response);
  },

  updateBoard: async (
    projectId: string,
    boardId: string,
    data: ProjectBoardUpdateInput
  ): Promise<ProjectBoard> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/boards/${boardId}`,
      {
        method: 'PATCH',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<ProjectBoard>(response);
  },

  deleteBoard: async (
    projectId: string,
    boardId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/boards/${boardId}`,
      {
        method: 'DELETE',
      }
    );
    return handleApiResponse<void>(response);
  },

  listAssets: async (projectId: string): Promise<ProjectAsset[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/assets`);
    return handleApiResponse<ProjectAsset[]>(response);
  },

  createAsset: async (
    projectId: string,
    data: ProjectAssetCreateInput
  ): Promise<ProjectAsset> => {
    // Convert bigint to number for JSON serialization
    const payload = {
      ...data,
      byte_size: data.byte_size !== undefined ? Number(data.byte_size) : undefined,
    };
    const response = await makeRequest(`/api/projects/${projectId}/assets`, {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    return handleApiResponse<ProjectAsset>(response);
  },

  updateAsset: async (
    projectId: string,
    assetId: string,
    data: ProjectAssetUpdateInput
  ): Promise<ProjectAsset> => {
    // Convert bigint to number for JSON serialization
    const payload = {
      ...data,
      byte_size: data.byte_size !== undefined ? Number(data.byte_size) : undefined,
    };
    const response = await makeRequest(
      `/api/projects/${projectId}/assets/${assetId}`,
      {
        method: 'PATCH',
        body: JSON.stringify(payload),
      }
    );
    return handleApiResponse<ProjectAsset>(response);
  },

  deleteAsset: async (
    projectId: string,
    assetId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/assets/${assetId}`,
      {
        method: 'DELETE',
      }
    );
    return handleApiResponse<void>(response);
  },

  // Client Assignment
  setClient: async (projectId: string, clientId: string | null): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/client`, {
      method: 'PUT',
      body: JSON.stringify({ client_id: clientId }),
    });
    return handleApiResponse<void>(response);
  },

  // Brand Profile APIs
  getBrandProfile: async (projectId: string): Promise<BrandProfile | null> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/brand-profile`
    );
    return handleApiResponse<BrandProfile | null>(response);
  },

  upsertBrandProfile: async (
    projectId: string,
    data: UpsertBrandProfile
  ): Promise<BrandProfile> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/brand-profile`,
      {
        method: 'PUT',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<BrandProfile>(response);
  },

  // Project hierarchy APIs
  setParent: async (projectId: string, parentProjectId: string | null): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/parent`, {
      method: 'PUT',
      body: JSON.stringify({ parent_project_id: parentProjectId }),
    });
    return handleApiResponse<void>(response);
  },

  reorder: async (projectId: string, sortOrder: number): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/reorder`, {
      method: 'PUT',
      body: JSON.stringify({ sort_order: sortOrder }),
    });
    return handleApiResponse<void>(response);
  },
};

// Project Controller Types
export interface ProjectControllerConfig {
  id: string;
  project_id: string;
  name: string;
  personality: string;
  system_prompt: string | null;
  voice_id: string | null;
  avatar_url: string | null;
  model: string | null;
  temperature: number | null;
  max_tokens: number | null;
  created_at: string;
  updated_at: string;
}

export interface UpdateControllerConfig {
  name?: string;
  personality?: string;
  system_prompt?: string;
  voice_id?: string;
  avatar_url?: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
}

export interface ProjectControllerConversation {
  id: string;
  project_id: string;
  user_id: string;
  title: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectControllerMessage {
  id: string;
  conversation_id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  tokens_used: number | null;
  created_at: string;
}

export interface ControllerChatResponse {
  message: ProjectControllerMessage;
  conversation_id: string;
}

// Project Controller APIs
export const projectControllersApi = {
  getConfig: async (projectId: string): Promise<ProjectControllerConfig> => {
    const response = await makeRequest(`/api/projects/${projectId}/controller`);
    return handleApiResponse<ProjectControllerConfig>(response);
  },

  updateConfig: async (
    projectId: string,
    data: UpdateControllerConfig
  ): Promise<ProjectControllerConfig> => {
    const response = await makeRequest(`/api/projects/${projectId}/controller`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProjectControllerConfig>(response);
  },

  getConversations: async (
    projectId: string
  ): Promise<ProjectControllerConversation[]> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/conversations`
    );
    return handleApiResponse<ProjectControllerConversation[]>(response);
  },

  getConversation: async (
    projectId: string,
    conversationId: string
  ): Promise<{ conversation: ProjectControllerConversation; messages: ProjectControllerMessage[] }> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/conversations/${conversationId}`
    );
    return handleApiResponse<{ conversation: ProjectControllerConversation; messages: ProjectControllerMessage[] }>(response);
  },

  deleteConversation: async (
    projectId: string,
    conversationId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/conversations/${conversationId}`,
      { method: 'DELETE' }
    );
    return handleApiResponse<void>(response);
  },

  sendMessage: async (
    projectId: string,
    content: string,
    conversationId?: string
  ): Promise<ControllerChatResponse> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/chat`,
      {
        method: 'POST',
        body: JSON.stringify({ content, conversation_id: conversationId }),
      }
    );
    return handleApiResponse<ControllerChatResponse>(response);
  },
};

// Assigned Task type for My Tasks
export interface AssignedTask {
  id: string;
  title: string;
  status: string;
  priority: string;
  due_date: string | null;
  project_id: string;
  project_name: string;
  description?: string | null;
  assigned_agent?: string | null;
  assignee_id?: string | null;
  created_by?: string | null;
  tags?: string | null;
}

// Task Management APIs
export const tasksApi = {
  getAssignedToMe: async (): Promise<AssignedTask[]> => {
    const response = await makeRequest('/api/tasks/assigned-to-me');
    return handleApiResponse<AssignedTask[]>(response);
  },
  getCreatedByMe: async (): Promise<AssignedTask[]> => {
    const response = await makeRequest('/api/tasks/created-by-me');
    return handleApiResponse<AssignedTask[]>(response);
  },
  getWatchedTasks: async (): Promise<AssignedTask[]> => {
    const response = await makeRequest('/api/tasks/watched');
    return handleApiResponse<AssignedTask[]>(response);
  },
  getAll: async (projectId: string): Promise<TaskWithArchive[]> => {
    const response = await makeRequest(`/api/tasks?project_id=${projectId}`);
    return handleApiResponse<TaskWithArchive[]>(response);
  },

  getById: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}`);
    return handleApiResponse<Task>(response);
  },

  create: async (data: CreateTask): Promise<Task> => {
    const response = await makeRequest(`/api/tasks`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<Task>(response);
  },

  createAndStart: async (
    data: CreateAndStartTaskRequest
  ): Promise<TaskWithAttemptStatus> => {
    const response = await makeRequest(`/api/tasks/create-and-start`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<TaskWithAttemptStatus>(response);
  },

  update: async (taskId: string, data: Partial<UpdateTask>): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<Task>(response);
  },

  delete: async (taskId: string): Promise<void> => {
    const response = await makeRequest(`/api/tasks/${taskId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  archive: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}/archive`, {
      method: 'POST',
    });
    return handleApiResponse<Task>(response);
  },

  unarchive: async (taskId: string): Promise<Task> => {
    const response = await makeRequest(`/api/tasks/${taskId}/archive`, {
      method: 'DELETE',
    });
    return handleApiResponse<Task>(response);
  },
};

// Task Attempts APIs
export const attemptsApi = {
  getChildren: async (attemptId: string): Promise<TaskRelationships> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/children`
    );
    return handleApiResponse<TaskRelationships>(response);
  },

  getAll: async (taskId: string): Promise<TaskAttempt[]> => {
    const response = await makeRequest(`/api/task-attempts?task_id=${taskId}`);
    return handleApiResponse<TaskAttempt[]>(response);
  },

  get: async (attemptId: string): Promise<TaskAttempt> => {
    const response = await makeRequest(`/api/task-attempts/${attemptId}`);
    return handleApiResponse<TaskAttempt>(response);
  },

  create: async (data: CreateTaskAttemptBody): Promise<TaskAttempt> => {
    const response = await makeRequest(`/api/task-attempts`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<TaskAttempt>(response);
  },

  stop: async (attemptId: string): Promise<void> => {
    const response = await makeRequest(`/api/task-attempts/${attemptId}/stop`, {
      method: 'POST',
    });
    return handleApiResponse<void>(response);
  },

  replaceProcess: async (
    attemptId: string,
    data: {
      process_id: string;
      prompt: string;
      variant?: string | null;
      force_when_dirty?: boolean;
      perform_git_reset?: boolean;
    }
  ): Promise<unknown> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/replace-process`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse(response);
  },

  followUp: async (
    attemptId: string,
    data: CreateFollowUpAttempt
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/follow-up`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<void>(response);
  },

  getFollowUpDraft: async (
    attemptId: string
  ): Promise<FollowUpDraftResponse> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/follow-up-draft`
    );
    return handleApiResponse<FollowUpDraftResponse>(response);
  },

  saveFollowUpDraft: async (
    attemptId: string,
    data: UpdateFollowUpDraftRequest
  ): Promise<FollowUpDraftResponse> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/follow-up-draft`,
      {
        // Server expects PUT for saving/updating the draft
        method: 'PUT',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<FollowUpDraftResponse>(response);
  },

  setFollowUpQueue: async (
    attemptId: string,
    queued: boolean,
    expectedQueued?: boolean,
    expectedVersion?: number
  ): Promise<FollowUpDraftResponse> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/follow-up-draft/queue`,
      {
        method: 'POST',
        body: JSON.stringify({
          queued,
          expected_queued: expectedQueued,
          expected_version: expectedVersion,
        }),
      }
    );
    return handleApiResponse<FollowUpDraftResponse>(response);
  },

  deleteFile: async (
    attemptId: string,
    fileToDelete: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/delete-file?file_path=${encodeURIComponent(
        fileToDelete
      )}`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<void>(response);
  },

  openEditor: async (
    attemptId: string,
    editorType?: EditorType,
    filePath?: string
  ): Promise<void> => {
    const requestBody: { editor_type?: EditorType; file_path?: string } = {};
    if (editorType) requestBody.editor_type = editorType;
    if (filePath) requestBody.file_path = filePath;

    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/open-editor`,
      {
        method: 'POST',
        body: JSON.stringify(
          Object.keys(requestBody).length > 0 ? requestBody : null
        ),
      }
    );
    return handleApiResponse<void>(response);
  },

  getBranchStatus: async (attemptId: string): Promise<BranchStatus> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/branch-status`
    );
    return handleApiResponse<BranchStatus>(response);
  },

  merge: async (attemptId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/merge`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<void>(response);
  },

  push: async (attemptId: string): Promise<void> => {
    const response = await makeRequest(`/api/task-attempts/${attemptId}/push`, {
      method: 'POST',
    });
    return handleApiResponse<void>(response);
  },

  rebase: async (
    attemptId: string,
    data: RebaseTaskAttemptRequest
  ): Promise<Result<void, GitOperationError>> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/rebase`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponseAsResult<void, GitOperationError>(response);
  },

  abortConflicts: async (attemptId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/conflicts/abort`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<void>(response);
  },

  createPR: async (
    attemptId: string,
    data: CreateGitHubPrRequest
  ): Promise<Result<string, GitHubServiceError>> => {
    const response = await makeRequest(`/api/task-attempts/${attemptId}/pr`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponseAsResult<string, GitHubServiceError>(response);
  },

  startDevServer: async (attemptId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/start-dev-server`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<void>(response);
  },
};

// Extra helpers
export const commitsApi = {
  getInfo: async (attemptId: string, sha: string): Promise<CommitInfo> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/commit-info?sha=${encodeURIComponent(
        sha
      )}`
    );
    return handleApiResponse<CommitInfo>(response);
  },
  compareToHead: async (
    attemptId: string,
    sha: string
  ): Promise<{
    head_oid: string;
    target_oid: string;
    ahead_from_head: number;
    behind_from_head: number;
    is_linear: boolean;
  }> => {
    const response = await makeRequest(
      `/api/task-attempts/${attemptId}/commit-compare?sha=${encodeURIComponent(
        sha
      )}`
    );
    return handleApiResponse(response);
  },
};

// Execution Process Logs type
export interface ExecutionProcessLogs {
  execution_id: string;
  logs: string; // JSONL format
  byte_size: number;
  inserted_at: Date;
}

// Execution Process APIs
export const executionProcessesApi = {
  getExecutionProcesses: async (
    attemptId: string
  ): Promise<ExecutionProcess[]> => {
    const response = await makeRequest(
      `/api/execution-processes?task_attempt_id=${attemptId}`
    );
    return handleApiResponse<ExecutionProcess[]>(response);
  },

  getDetails: async (processId: string): Promise<ExecutionProcess> => {
    const response = await makeRequest(`/api/execution-processes/${processId}`);
    return handleApiResponse<ExecutionProcess>(response);
  },

  getStoredLogs: async (processId: string): Promise<ExecutionProcessLogs | null> => {
    const response = await makeRequest(`/api/execution-processes/${processId}/logs`);
    return handleApiResponse<ExecutionProcessLogs | null>(response);
  },

  stopExecutionProcess: async (processId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/execution-processes/${processId}/stop`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<void>(response);
  },
};

// Execution Summary APIs
export const executionSummaryApi = {
  getByAttemptId: async (attemptId: string): Promise<ExecutionSummary | null> => {
    try {
      const response = await makeRequest(`/api/task-attempts/${attemptId}/summary`);
      return handleApiResponse<ExecutionSummary>(response);
    } catch (error) {
      // Return null if not found
      if (error instanceof ApiError && error.status === 404) {
        return null;
      }
      throw error;
    }
  },

  getById: async (summaryId: string): Promise<ExecutionSummary> => {
    const response = await makeRequest(`/api/execution-summaries/${summaryId}`);
    return handleApiResponse<ExecutionSummary>(response);
  },

  updateFeedback: async (
    summaryId: string,
    feedback: {
      human_rating?: number | null;
      human_notes?: string | null;
      is_reference_example?: boolean | null;
    }
  ): Promise<ExecutionSummary> => {
    const response = await makeRequest(
      `/api/execution-summaries/${summaryId}/feedback`,
      {
        method: 'POST',
        body: JSON.stringify(feedback),
      }
    );
    return handleApiResponse<ExecutionSummary>(response);
  },
};

// User type for the users API
export interface UserListItem {
  id: string;
  username: string;
  email: string;
  full_name: string;
  avatar_url: string | null;
  is_active: number;
  is_admin: number;
  last_login_at: string | null;
  created_at: string;
}

// Users API
export const usersApi = {
  // List/search users
  list: async (params?: {
    search?: string;
    is_active?: boolean;
    is_admin?: boolean;
    limit?: number;
    offset?: number;
  }): Promise<UserListItem[]> => {
    const searchParams = new URLSearchParams();
    if (params?.search) searchParams.set('search', params.search);
    if (params?.is_active !== undefined) searchParams.set('is_active', String(params.is_active));
    if (params?.is_admin !== undefined) searchParams.set('is_admin', String(params.is_admin));
    if (params?.limit) searchParams.set('limit', String(params.limit));
    if (params?.offset) searchParams.set('offset', String(params.offset));

    const queryString = searchParams.toString();
    const url = queryString ? `/api/users?${queryString}` : '/api/users';
    const response = await makeRequest(url);
    return handleApiResponse<UserListItem[]>(response);
  },

  // Get user by ID
  getById: async (userId: string): Promise<UserListItem> => {
    const response = await makeRequest(`/api/users/${userId}`);
    return handleApiResponse<UserListItem>(response);
  },
};

// Agent Registry APIs
// Note: These endpoints return raw data, not wrapped in ApiResponse format
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
import type { AgentWatcherInfo } from 'shared/types';
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

// Airtable Integration API
export const airtableApi = {
  // Verify Airtable Personal Access Token
  verifyCredentials: async (
    credentials: AirtableVerifyRequest
  ): Promise<AirtableVerifyResponse> => {
    const response = await makeRequest('/api/airtable/verify', {
      method: 'POST',
      body: JSON.stringify(credentials),
    });
    return handleApiResponse<AirtableVerifyResponse>(response);
  },

  // List user's Airtable bases (from Airtable API)
  listUserBases: async (): Promise<AirtableBaseInfo[]> => {
    const response = await makeRequest('/api/airtable/bases');
    return handleApiResponse<AirtableBaseInfo[]>(response);
  },

  // List base connections (from our DB)
  listConnections: async (
    projectId?: string
  ): Promise<AirtableBase[]> => {
    const url = projectId
      ? `/api/airtable/connections?project_id=${projectId}`
      : '/api/airtable/connections';
    const response = await makeRequest(url);
    return handleApiResponse<AirtableBase[]>(response);
  },

  // Get a single connection with base info
  getConnection: async (
    connectionId: string
  ): Promise<AirtableConnectionWithBase> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}`
    );
    return handleApiResponse<AirtableConnectionWithBase>(response);
  },

  // Create a base connection
  createConnection: async (
    connection: CreateAirtableBase
  ): Promise<AirtableBase> => {
    const response = await makeRequest('/api/airtable/connections', {
      method: 'POST',
      body: JSON.stringify(connection),
    });
    return handleApiResponse<AirtableBase>(response);
  },

  // Update a base connection
  updateConnection: async (
    connectionId: string,
    update: UpdateAirtableBase
  ): Promise<AirtableBase> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}`,
      {
        method: 'PATCH',
        body: JSON.stringify(update),
      }
    );
    return handleApiResponse<AirtableBase>(response);
  },

  // Delete a base connection
  deleteConnection: async (connectionId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}`,
      {
        method: 'DELETE',
      }
    );
    return handleApiResponse<void>(response);
  },

  // Get tables in a connected base
  getBaseTables: async (connectionId: string): Promise<AirtableTable[]> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}/tables`
    );
    return handleApiResponse<AirtableTable[]>(response);
  },

  // Get records from a table in a connected base
  getTableRecords: async (
    connectionId: string,
    tableId: string
  ): Promise<AirtableRecord[]> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}/records?table_id=${tableId}`
    );
    return handleApiResponse<AirtableRecord[]>(response);
  },

  // Import records from an Airtable table as PCG tasks
  importRecords: async (
    connectionId: string,
    request: AirtableImportRequest
  ): Promise<AirtableImportResult> => {
    const response = await makeRequest(
      `/api/airtable/connections/${connectionId}/import`,
      {
        method: 'POST',
        body: JSON.stringify(request),
      }
    );
    return handleApiResponse<AirtableImportResult>(response);
  },

  // Get Airtable link for a task
  getTaskLink: async (taskId: string): Promise<AirtableRecordLink | null> => {
    const response = await makeRequest(`/api/airtable/tasks/${taskId}/link`);
    return handleApiResponse<AirtableRecordLink | null>(response);
  },

  // Push a PCG task to Airtable
  pushTaskToAirtable: async (
    taskId: string,
    request: AirtablePushTaskRequest
  ): Promise<AirtableRecordLink> => {
    const response = await makeRequest(`/api/airtable/tasks/${taskId}/push`, {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return handleApiResponse<AirtableRecordLink>(response);
  },

  // Sync task deliverables to Airtable as a comment
  syncDeliverables: async (taskId: string): Promise<AirtableRecordLink> => {
    const response = await makeRequest(
      `/api/airtable/tasks/${taskId}/sync-deliverables`,
      {
        method: 'POST',
      }
    );
    return handleApiResponse<AirtableRecordLink>(response);
  },
};

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

// ============================================
// Artifact Review APIs
// ============================================

export interface ArtifactReview {
  id: string;
  artifact_id: string;
  reviewer_id?: string;
  reviewer_agent_id?: string;
  reviewer_name?: string;
  review_type: string;
  status: string;
  feedback_text?: string;
  rating?: number;
  revision_notes?: string;
  revision_deadline?: string;
  resolved_at?: string;
  resolved_by?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateArtifactReview {
  artifact_id: string;
  reviewer_id?: string;
  reviewer_agent_id?: string;
  reviewer_name?: string;
  review_type: string;
  feedback_text?: string;
  rating?: number;
  revision_notes?: Record<string, unknown>;
  revision_deadline?: string;
}

export interface ResolveReview {
  status: string;
  feedback_text?: string;
  rating?: number;
  resolved_by: string;
}

export const artifactReviewsApi = {
  list: async (params?: {
    artifact_id?: string;
    reviewer_id?: string;
    status?: string;
    pending_only?: boolean;
  }): Promise<ArtifactReview[]> => {
    const searchParams = new URLSearchParams();
    if (params?.artifact_id)
      searchParams.set('artifact_id', params.artifact_id);
    if (params?.reviewer_id)
      searchParams.set('reviewer_id', params.reviewer_id);
    if (params?.status) searchParams.set('status', params.status);
    if (params?.pending_only)
      searchParams.set('pending_only', params.pending_only.toString());
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/artifact-reviews${query ? `?${query}` : ''}`
    );
    return handleApiResponse<ArtifactReview[]>(response);
  },

  listPending: async (): Promise<ArtifactReview[]> => {
    const response = await makeRequest('/api/artifact-reviews/pending');
    return handleApiResponse<ArtifactReview[]>(response);
  },

  getById: async (reviewId: string): Promise<ArtifactReview> => {
    const response = await makeRequest(`/api/artifact-reviews/${reviewId}`);
    return handleApiResponse<ArtifactReview>(response);
  },

  create: async (data: CreateArtifactReview): Promise<ArtifactReview> => {
    const response = await makeRequest('/api/artifact-reviews', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ArtifactReview>(response);
  },

  delete: async (reviewId: string): Promise<void> => {
    const response = await makeRequest(`/api/artifact-reviews/${reviewId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  resolve: async (
    reviewId: string,
    data: ResolveReview
  ): Promise<ArtifactReview> => {
    const response = await makeRequest(
      `/api/artifact-reviews/${reviewId}/resolve`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<ArtifactReview>(response);
  },
};

// ============================================
// Task Artifacts APIs
// ============================================

export interface TaskArtifact {
  id: string;
  task_id: string;
  artifact_id: string;
  artifact_role: string;
  display_order: number;
  pinned: boolean;
  added_by?: string;
  created_at: string;
}

export interface TaskArtifactWithDetails {
  link: TaskArtifact;
  artifact?: ExecutionArtifact;
}

export interface LinkArtifactToTask {
  artifact_id: string;
  artifact_role?: string;
  display_order?: number;
  pinned?: boolean;
  added_by?: string;
}

export interface ExecutionArtifact {
  id: string;
  execution_process_id?: string;
  artifact_type: string;
  title?: string;
  content?: string;
  file_path?: string;
  metadata?: string;
  phase?: string;
  created_by_agent_id?: string;
  review_status?: string;
  parent_artifact_id?: string;
  created_at: string;
}

export const taskArtifactsApi = {
  list: async (
    taskId: string,
    params?: { role?: string; pinned_only?: boolean }
  ): Promise<TaskArtifactWithDetails[]> => {
    const searchParams = new URLSearchParams();
    if (params?.role) searchParams.set('role', params.role);
    if (params?.pinned_only)
      searchParams.set('pinned_only', params.pinned_only.toString());
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<TaskArtifactWithDetails[]>(response);
  },

  link: async (
    taskId: string,
    data: LinkArtifactToTask
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(`/api/tasks/${taskId}/artifacts`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<TaskArtifact>(response);
  },

  unlink: async (taskId: string, artifactId: string): Promise<boolean> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}`,
      { method: 'DELETE' }
    );
    return handleApiResponse<boolean>(response);
  },

  updateRole: async (
    taskId: string,
    artifactId: string,
    role: string
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}/role`,
      {
        method: 'POST',
        body: JSON.stringify({ role }),
      }
    );
    return handleApiResponse<TaskArtifact>(response);
  },

  togglePin: async (
    taskId: string,
    artifactId: string
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}/pin`,
      { method: 'POST' }
    );
    return handleApiResponse<TaskArtifact>(response);
  },

  reorder: async (
    taskId: string,
    artifactId: string,
    newOrder: number
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}/reorder`,
      {
        method: 'POST',
        body: JSON.stringify({ new_order: newOrder }),
      }
    );
    return handleApiResponse<TaskArtifact>(response);
  },

  getByArtifact: async (artifactId: string): Promise<TaskArtifact[]> => {
    const response = await makeRequest(`/api/artifacts/${artifactId}/tasks`);
    return handleApiResponse<TaskArtifact[]>(response);
  },
};

// ============================================
// Artifact Content & Download API
// ============================================

export const artifactContentApi = {
  getContentUrl: (artifactId: string): string =>
    resolveApiUrl(`/api/artifacts/${artifactId}/content`),

  getDownloadUrl: (artifactId: string): string =>
    resolveApiUrl(`/api/artifacts/${artifactId}/download`),

  getFileUrl: (artifactId: string, filename: string): string =>
    resolveApiUrl(`/api/artifacts/${artifactId}/files/${encodeURIComponent(filename)}`),

  getContent: async (artifactId: string): Promise<unknown> => {
    const response = await makeRequest(`/api/artifacts/${artifactId}/content`);
    if (!response.ok) {
      throw new Error(`Failed to fetch artifact content: ${response.statusText}`);
    }
    return response.json();
  },
};

// ============================================
// Editron Export API
// ============================================

export const editronApi = {
  getExportXmlUrl: (artifactId: string): string =>
    resolveApiUrl(`/api/editron/export/${artifactId}?format=xml`),
};

// ============================================
// Social Command APIs
// ============================================

export interface SocialAccountRecord {
  id: string;
  project_id: string;
  platform: string;
  account_type: string;
  platform_account_id: string;
  username?: string | null;
  display_name?: string | null;
  profile_url?: string | null;
  avatar_url?: string | null;
  follower_count?: number | null;
  following_count?: number | null;
  post_count?: number | null;
  metadata?: string | null;
  status: string;
  last_sync_at?: string | null;
  last_error?: string | null;
  created_at: string;
  updated_at: string;
}

export interface SocialPostRecord {
  id: string;
  project_id: string;
  social_account_id?: string | null;
  task_id?: string | null;
  content_type: string;
  caption?: string | null;
  content_blocks?: string | null;
  media_urls?: string | null;
  hashtags?: string | null;
  mentions?: string | null;
  platforms: string;
  platform_specific?: string | null;
  status: string;
  scheduled_for?: string | null;
  published_at?: string | null;
  category?: string | null;
  queue_position?: number | null;
  is_evergreen: boolean;
  recycle_after_days?: number | null;
  last_recycled_at?: string | null;
  created_by_agent_id?: string | null;
  approved_by?: string | null;
  approved_at?: string | null;
  platform_post_id?: string | null;
  platform_url?: string | null;
  publish_error?: string | null;
  impressions: number;
  reach: number;
  likes: number;
  comments: number;
  shares: number;
  saves: number;
  clicks: number;
  engagement_rate: number;
  created_at: string;
  updated_at: string;
}

export interface SocialMentionRecord {
  id: string;
  social_account_id: string;
  project_id: string;
  mention_type: string;
  platform: string;
  platform_mention_id: string;
  author_username?: string | null;
  author_display_name?: string | null;
  author_avatar_url?: string | null;
  author_follower_count?: number | null;
  author_is_verified: boolean;
  content?: string | null;
  media_urls?: string | null;
  parent_post_id?: string | null;
  parent_platform_id?: string | null;
  status: string;
  sentiment?: string | null;
  priority: string;
  replied_at?: string | null;
  replied_by?: string | null;
  reply_content?: string | null;
  assigned_agent_id?: string | null;
  auto_response_sent: boolean;
  received_at: string;
  created_at: string;
  updated_at: string;
}

export interface SocialInboxStats {
  total_unread: number;
  high_priority: number;
}

export const socialApi = {
  listAccounts: async (projectId?: string): Promise<SocialAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/social/accounts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<SocialAccountRecord[]>(response);
  },

  listPosts: async (projectId?: string): Promise<SocialPostRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/social/posts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<SocialPostRecord[]>(response);
  },

  listMentions: async (
    projectId: string,
    options?: { unreadOnly?: boolean; limit?: number; accountId?: string }
  ): Promise<SocialMentionRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('project_id', projectId);
    if (options?.unreadOnly) searchParams.set('unread_only', 'true');
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    if (options?.accountId)
      searchParams.set('social_account_id', options.accountId);
    const query = searchParams.toString();
    const response = await makeRequest(`/api/social/inbox?${query}`);
    return handleApiResponse<SocialMentionRecord[]>(response);
  },

  inboxStats: async (projectId: string): Promise<SocialInboxStats> => {
    const response = await makeRequest(`/api/social/inbox/stats/${projectId}`);
    return handleApiResponse<SocialInboxStats>(response);
  },

  listPostsFiltered: async (params: {
    projectId?: string;
    status?: string;
    category?: string;
    platform?: string;
    limit?: number;
  }): Promise<SocialPostRecord[]> => {
    const sp = new URLSearchParams();
    if (params.projectId) sp.set('project_id', params.projectId);
    if (params.status) sp.set('status', params.status);
    if (params.category) sp.set('category', params.category);
    if (params.platform) sp.set('platform', params.platform);
    if (params.limit) sp.set('limit', params.limit.toString());
    const response = await makeRequest(`/api/social/posts?${sp.toString()}`);
    return handleApiResponse<SocialPostRecord[]>(response);
  },

  createPost: async (data: {
    project_id: string;
    caption: string;
    platforms: string;
    content_type?: string;
    status?: string;
    scheduled_for?: string;
    category?: string;
    hashtags?: string;
  }): Promise<SocialPostRecord> => {
    const response = await makeRequest('/api/social/posts', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  updatePost: async (id: string, data: Partial<{
    caption: string;
    status: string;
    scheduled_for: string | null;
    category: string;
    platforms: string;
  }>): Promise<SocialPostRecord> => {
    const response = await makeRequest(`/api/social/posts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialPostRecord>(response);
  },

  deletePost: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/social/posts/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  updateMention: async (id: string, data: Partial<{
    status: string;
    priority: string;
    sentiment: string;
    reply_content: string;
    replied_by: string;
    replied_at: string;
  }>): Promise<SocialMentionRecord> => {
    const response = await makeRequest(`/api/social/inbox/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialMentionRecord>(response);
  },

  updateAccount: async (id: string, data: Partial<{
    status: string;
    username: string;
    display_name: string;
    follower_count: number;
  }>): Promise<SocialAccountRecord> => {
    const response = await makeRequest(`/api/social/accounts/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<SocialAccountRecord>(response);
  },

  deleteAccount: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/social/accounts/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// =============================================================================
// Email Account Records
// =============================================================================

export interface EmailAccountRecord {
  id: string;
  project_id: string;
  provider: string;
  account_type: string;
  email_address: string;
  display_name: string | null;
  avatar_url: string | null;
  granted_scopes: string | null;
  storage_used_bytes: number | null;
  storage_total_bytes: number | null;
  unread_count: number | null;
  status: string;
  last_sync_at: string | null;
  last_error: string | null;
  sync_enabled: number | null;
  sync_frequency_minutes: number | null;
  auto_reply_enabled: number | null;
  signature: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateEmailAccountRequest {
  project_id: string;
  provider: string;
  account_type?: string;
  email_address: string;
  display_name?: string;
  avatar_url?: string;
  access_token?: string;
  refresh_token?: string;
  token_expires_at?: string;
  imap_host?: string;
  imap_port?: number;
  smtp_host?: string;
  smtp_port?: number;
  use_ssl?: boolean;
  granted_scopes?: string[];
  metadata?: Record<string, unknown>;
}

export interface UpdateEmailAccountRequest {
  display_name?: string;
  avatar_url?: string;
  status?: string;
  sync_enabled?: boolean;
  sync_frequency_minutes?: number;
  auto_reply_enabled?: boolean;
  signature?: string;
}

export interface OAuthUrlResponse {
  auth_url: string;
  state: string;
}

export const emailApi = {
  listAccounts: async (projectId?: string, provider?: string, ownerType?: string, ownerId?: string): Promise<EmailAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    if (provider) searchParams.set('provider', provider);
    if (ownerType) searchParams.set('owner_type', ownerType);
    if (ownerId) searchParams.set('owner_id', ownerId);
    const query = searchParams.toString();
    const response = await makeRequest(`/api/email/accounts${query ? `?${query}` : ''}`);
    return handleApiResponse<EmailAccountRecord[]>(response);
  },

  getAccount: async (id: string): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}`);
    return handleApiResponse<EmailAccountRecord>(response);
  },

  createAccount: async (data: CreateEmailAccountRequest): Promise<EmailAccountRecord> => {
    const response = await makeRequest('/api/email/accounts', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  updateAccount: async (id: string, data: UpdateEmailAccountRequest): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  deleteAccount: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/accounts/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  triggerSync: async (id: string): Promise<EmailAccountRecord> => {
    const response = await makeRequest(`/api/email/accounts/${id}/sync`, {
      method: 'POST',
    });
    return handleApiResponse<EmailAccountRecord>(response);
  },

  initiateOAuth: async (
    projectId: string | null,
    provider: string,
    redirectUri: string,
    ownerType?: string,
    ownerId?: string
  ): Promise<OAuthUrlResponse> => {
    const response = await makeRequest('/api/email/oauth/initiate', {
      method: 'POST',
      body: JSON.stringify({
        ...(projectId ? { project_id: projectId } : {}),
        ...(ownerType ? { owner_type: ownerType } : {}),
        ...(ownerId ? { owner_id: ownerId } : {}),
        provider,
        redirect_uri: redirectUri,
      }),
    });
    return handleApiResponse<OAuthUrlResponse>(response);
  },
};

// =============================================================================
// CRM Contact Records
// =============================================================================

export interface CrmContactRecord {
  id: string;
  organization_id: string;
  project_id: string | null;
  client_id: string | null;
  first_name: string | null;
  last_name: string | null;
  full_name: string | null;
  email: string | null;
  phone: string | null;
  mobile: string | null;
  avatar_url: string | null;
  company_name: string | null;
  job_title: string | null;
  department: string | null;
  linkedin_url: string | null;
  twitter_handle: string | null;
  website: string | null;
  source: string | null;
  lifecycle_stage: string;
  lead_score: number;
  last_activity_at: string | null;
  last_contacted_at: string | null;
  last_replied_at: string | null;
  owner_user_id: string | null;
  assigned_agent_id: string | null;
  zoho_contact_id: string | null;
  gmail_contact_id: string | null;
  external_ids: string | null;
  tags: string | null;
  lists: string | null;
  custom_fields: string | null;
  address_line1: string | null;
  address_line2: string | null;
  city: string | null;
  state: string | null;
  postal_code: string | null;
  country: string | null;
  email_opt_in: number | null;
  sms_opt_in: number | null;
  do_not_contact: number | null;
  email_count: number;
  meeting_count: number;
  deal_count: number;
  total_revenue: number;
  created_at: string;
  updated_at: string;
}

export interface CreateCrmContactRequest {
  organization_id: string;
  client_id?: string;
  first_name?: string;
  last_name?: string;
  email?: string;
  phone?: string;
  mobile?: string;
  avatar_url?: string;
  company_name?: string;
  job_title?: string;
  department?: string;
  linkedin_url?: string;
  twitter_handle?: string;
  website?: string;
  source?: string;
  lifecycle_stage?: string;
  tags?: string[];
  custom_fields?: Record<string, unknown>;
  zoho_contact_id?: string;
  gmail_contact_id?: string;
}

export interface UpdateCrmContactRequest {
  first_name?: string;
  last_name?: string;
  email?: string;
  phone?: string;
  mobile?: string;
  avatar_url?: string;
  company_name?: string;
  job_title?: string;
  department?: string;
  linkedin_url?: string;
  twitter_handle?: string;
  website?: string;
  source?: string;
  lifecycle_stage?: string;
  lead_score?: number;
  owner_user_id?: string;
  assigned_agent_id?: string;
  tags?: string[];
  custom_fields?: Record<string, unknown>;
  address_line1?: string;
  address_line2?: string;
  city?: string;
  state?: string;
  postal_code?: string;
  country?: string;
  email_opt_in?: boolean;
  sms_opt_in?: boolean;
  do_not_contact?: boolean;
  zoho_contact_id?: string;
  gmail_contact_id?: string;
}

export interface CrmContactStats {
  total: number;
  by_stage: Array<{ stage: string; count: number }>;
  avg_lead_score: number;
  needs_follow_up: number;
}

// =============================================================================
// QuickBooks API
// =============================================================================

export interface QuickBooksAccountRecord {
  id: string;
  organization_id: string;
  realm_id: string;
  company_name?: string;
  environment: string;
  sync_enabled: number;
  sync_frequency_minutes: number;
  last_sync_at?: string;
  sync_invoices: number;
  sync_customers: number;
  sync_payments: number;
  sync_expenses: number;
  sync_time_tracking: number;
  status: string;
  last_error?: string;
  metadata?: string;
  connected_by?: string;
  created_at: string;
  updated_at: string;
}

export interface QBConnectionStatus {
  connected: boolean;
  account?: QuickBooksAccountRecord;
  needs_reauth: boolean;
}

export const quickbooksApi = {
  getStatus: async (organizationId: string): Promise<QBConnectionStatus> => {
    const response = await makeRequest(`/api/quickbooks/status?organization_id=${organizationId}`);
    return handleApiResponse<QBConnectionStatus>(response);
  },

  getConnectUrl: (organizationId: string): string => {
    return `/api/quickbooks/connect?organization_id=${organizationId}`;
  },

  disconnect: async (accountId: string): Promise<void> => {
    const response = await makeRequest(`/api/quickbooks/accounts/${accountId}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  refreshToken: async (accountId: string): Promise<void> => {
    const response = await makeRequest(`/api/quickbooks/accounts/${accountId}/refresh`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },

  triggerSync: async (accountId: string): Promise<void> => {
    const response = await makeRequest(`/api/quickbooks/accounts/${accountId}/sync`, {
      method: 'POST',
      body: JSON.stringify({}),
    });
    return handleApiResponse<void>(response);
  },
};

export const crmApi = {
  listContacts: async (
    organizationId: string,
    options?: { lifecycleStage?: string; limit?: number }
  ): Promise<CrmContactRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('organization_id', organizationId);
    if (options?.lifecycleStage) searchParams.set('lifecycle_stage', options.lifecycleStage);
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    const response = await makeRequest(`/api/crm/contacts?${searchParams.toString()}`);
    return handleApiResponse<CrmContactRecord[]>(response);
  },

  searchContacts: async (
    organizationId: string,
    query?: string,
    options?: {
      lifecycleStage?: string;
      companyName?: string;
      minLeadScore?: number;
      limit?: number;
      offset?: number;
    }
  ): Promise<CrmContactRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('organization_id', organizationId);
    if (query) searchParams.set('query', query);
    if (options?.lifecycleStage) searchParams.set('lifecycle_stage', options.lifecycleStage);
    if (options?.companyName) searchParams.set('company_name', options.companyName);
    if (options?.minLeadScore) searchParams.set('min_lead_score', options.minLeadScore.toString());
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    if (options?.offset) searchParams.set('offset', options.offset.toString());
    const response = await makeRequest(`/api/crm/contacts/search?${searchParams.toString()}`);
    return handleApiResponse<CrmContactRecord[]>(response);
  },

  getContact: async (id: string): Promise<CrmContactRecord> => {
    const response = await makeRequest(`/api/crm/contacts/${id}`);
    return handleApiResponse<CrmContactRecord>(response);
  },

  getContactByEmail: async (projectId: string, email: string): Promise<CrmContactRecord | null> => {
    const response = await makeRequest(`/api/crm/contacts/by-email/${projectId}/${encodeURIComponent(email)}`);
    return handleApiResponse<CrmContactRecord | null>(response);
  },

  createContact: async (data: CreateCrmContactRequest): Promise<CrmContactRecord> => {
    const response = await makeRequest('/api/crm/contacts', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmContactRecord>(response);
  },

  updateContact: async (id: string, data: UpdateCrmContactRequest): Promise<CrmContactRecord> => {
    const response = await makeRequest(`/api/crm/contacts/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmContactRecord>(response);
  },

  deleteContact: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  getContactStats: async (organizationId: string): Promise<CrmContactStats> => {
    const response = await makeRequest(`/api/crm/contacts/stats/${organizationId}`);
    return handleApiResponse<CrmContactStats>(response);
  },

  recordActivity: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/activity`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  recordContacted: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/contacted`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  recordReplied: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/replied`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  updateLeadScore: async (id: string, scoreDelta: number): Promise<CrmContactRecord> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/lead-score`, {
      method: 'POST',
      body: JSON.stringify({ score_delta: scoreDelta }),
    });
    return handleApiResponse<CrmContactRecord>(response);
  },
};

// ============================================================================
// CRM Pipeline & Deal API
// ============================================================================

export type {
  CrmPipeline,
  CrmPipelineStage,
  CrmPipelineWithStages,
  CreateCrmPipeline,
  UpdateCrmPipeline,
  CreateCrmPipelineStage,
  UpdateCrmPipelineStage,
  KanbanBoardData,
  CrmDealRecord,
  CreateCrmDeal,
  UpdateCrmDeal,
  MoveDealRequest,
  PipelineType,
};

export const crmPipelinesApi = {
  /** List pipelines for an organization */
  listPipelines: async (
    organizationId: string,
    options?: { pipelineType?: PipelineType }
  ): Promise<CrmPipeline[]> => {
    const params = new URLSearchParams();
    params.set('organization_id', organizationId);
    if (options?.pipelineType) params.set('pipeline_type', options.pipelineType);
    const response = await makeRequest(`/api/crm/pipelines?${params.toString()}`);
    return handleApiResponse<CrmPipeline[]>(response);
  },

  /** List pipelines for an organization */
  listOrgPipelines: async (orgId: string): Promise<CrmPipeline[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/crm/pipelines`);
    return handleApiResponse<CrmPipeline[]>(response);
  },

  /** Get a single pipeline with its stages */
  getPipeline: async (id: string): Promise<CrmPipelineWithStages> => {
    const response = await makeRequest(`/api/crm/pipelines/${id}`);
    return handleApiResponse<CrmPipelineWithStages>(response);
  },

  /** Get a pipeline with stages under an org (by pipeline id) */
  getOrgPipeline: async (orgId: string, pipelineId: string): Promise<CrmPipelineWithStages> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/crm/pipelines/${pipelineId}`
    );
    return handleApiResponse<CrmPipelineWithStages>(response);
  },

  createPipeline: async (data: CreateCrmPipeline): Promise<CrmPipeline> => {
    const response = await makeRequest('/api/crm/pipelines', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmPipeline>(response);
  },

  updatePipeline: async (id: string, data: UpdateCrmPipeline): Promise<CrmPipeline> => {
    const response = await makeRequest(`/api/crm/pipelines/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmPipeline>(response);
  },

  deletePipeline: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/pipelines/${id}`, { method: 'DELETE' });
    await handleApiResponse<void>(response);
  },

  listStages: async (pipelineId: string): Promise<CrmPipelineStage[]> => {
    const response = await makeRequest(`/api/crm/pipelines/${pipelineId}/stages`);
    return handleApiResponse<CrmPipelineStage[]>(response);
  },

  createStage: async (
    pipelineId: string,
    data: Omit<CreateCrmPipelineStage, 'pipeline_id'>
  ): Promise<CrmPipelineStage> => {
    const response = await makeRequest(`/api/crm/pipelines/${pipelineId}/stages`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmPipelineStage>(response);
  },

  updateStage: async (
    pipelineId: string,
    stageId: string,
    data: UpdateCrmPipelineStage
  ): Promise<CrmPipelineStage> => {
    const response = await makeRequest(
      `/api/crm/pipelines/${pipelineId}/stages/${stageId}`,
      { method: 'PATCH', body: JSON.stringify(data) }
    );
    return handleApiResponse<CrmPipelineStage>(response);
  },

  deleteStage: async (pipelineId: string, stageId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/crm/pipelines/${pipelineId}/stages/${stageId}`,
      { method: 'DELETE' }
    );
    await handleApiResponse<void>(response);
  },

  reorderStages: async (
    pipelineId: string,
    stageIds: string[]
  ): Promise<CrmPipelineStage[]> => {
    const response = await makeRequest(`/api/crm/pipelines/${pipelineId}/stages/reorder`, {
      method: 'POST',
      body: JSON.stringify({ stage_ids: stageIds }),
    });
    return handleApiResponse<CrmPipelineStage[]>(response);
  },
};

export const crmDealsApi = {
  listDeals: async (options: {
    organization_id?: string;
    pipeline_id?: string;
    stage_id?: string;
    contact_id?: string;
  }): Promise<CrmDealRecord[]> => {
    const params = new URLSearchParams();
    if (options.organization_id) params.set('organization_id', options.organization_id);
    if (options.pipeline_id) params.set('pipeline_id', options.pipeline_id);
    if (options.stage_id) params.set('stage_id', options.stage_id);
    if (options.contact_id) params.set('contact_id', options.contact_id);
    const response = await makeRequest(`/api/crm/deals?${params}`);
    return handleApiResponse<CrmDealRecord[]>(response);
  },

  getDeal: async (id: string): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${id}`);
    return handleApiResponse<CrmDealRecord>(response);
  },

  createDeal: async (data: CreateCrmDeal): Promise<CrmDealRecord> => {
    const response = await makeRequest('/api/crm/deals', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmDealRecord>(response);
  },

  updateDeal: async (id: string, data: UpdateCrmDeal): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmDealRecord>(response);
  },

  moveDeal: async (dealId: string, data: MoveDealRequest): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/stage`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmDealRecord>(response);
  },

  deleteDeal: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/deals/${id}`, { method: 'DELETE' });
    await handleApiResponse<void>(response);
  },

  getMetrics: async (organizationId: string, pipelineId?: string): Promise<PipelineMetricsRecord> => {
    const params = new URLSearchParams({ organization_id: organizationId });
    if (pipelineId) params.set('pipeline_id', pipelineId);
    const response = await makeRequest(`/api/crm/deals/metrics?${params}`);
    return handleApiResponse<PipelineMetricsRecord>(response);
  },

  /** Get Kanban board data for a pipeline (deals grouped by stage) */
  getKanbanData: async (pipelineId: string): Promise<KanbanBoardData> => {
    const response = await makeRequest(`/api/crm/deals/kanban/${pipelineId}`);
    return handleApiResponse<KanbanBoardData>(response);
  },

  /** Get Kanban board for an org-scoped pipeline */
  getOrgKanbanData: async (orgId: string, pipelineId: string): Promise<KanbanBoardData> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/crm/pipelines/${pipelineId}/kanban`
    );
    return handleApiResponse<KanbanBoardData>(response);
  },

  /** List all deals for an organization */
  listOrgDeals: async (orgId: string): Promise<CrmDealRecord[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/crm/deals`);
    return handleApiResponse<CrmDealRecord[]>(response);
  },
};

// CRM Activities API
export interface CrmActivityRecord {
  id: string;
  organization_id: string;
  project_id: string | null;
  client_id: string | null;
  crm_contact_id?: string;
  crm_deal_id?: string;
  activity_type: string;
  subject?: string;
  description?: string;
  outcome?: string;
  email_message_id?: string;
  social_mention_id?: string;
  task_id?: string;
  performed_by_user?: string;
  performed_by_agent_id?: string;
  metadata?: string;
  duration_minutes?: number;
  activity_at: string;
  created_at: string;
}

export interface PipelineMetricsRecord {
  pipeline_id: string;
  total_deals: number;
  total_value: number;
  weighted_value: number;
  avg_deal_size: number;
  win_rate: number;
  deals_by_stage: Array<{
    stage_id: string;
    stage_name: string;
    count: number;
    total_value: number;
  }>;
  monthly_summary: Array<{
    month: string;
    new_deals: number;
    won_deals: number;
    lost_deals: number;
    total_value: number;
  }>;
}

export const crmActivitiesApi = {
  listActivities: async (options: {
    organization_id?: string;
    contact_id?: string;
    deal_id?: string;
    limit?: number;
  }): Promise<CrmActivityRecord[]> => {
    const params = new URLSearchParams();
    if (options.organization_id) params.set('organization_id', options.organization_id);
    if (options.contact_id) params.set('contact_id', options.contact_id);
    if (options.deal_id) params.set('deal_id', options.deal_id);
    if (options.limit) params.set('limit', options.limit.toString());
    const response = await makeRequest(`/api/crm/activities?${params}`);
    return handleApiResponse<CrmActivityRecord[]>(response);
  },

  createActivity: async (data: {
    organization_id: string;
    crm_contact_id?: string;
    crm_deal_id?: string;
    activity_type: string;
    subject?: string;
    description?: string;
    outcome?: string;
    performed_by_user?: string;
    duration_minutes?: number;
    metadata?: Record<string, unknown>;
  }): Promise<CrmActivityRecord> => {
    const response = await makeRequest('/api/crm/activities', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmActivityRecord>(response);
  },

  deleteActivity: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/activities/${id}`, { method: 'DELETE' });
    await handleApiResponse<void>(response);
  },
};

// ── Workflow Templates ──

export interface WorkflowTaskTemplate {
  title: string;
  description: string;
  position: number;
  task_type: 'agent' | 'human_review' | 'hybrid';
  agent_role?: string;
  requires_approval: boolean;
  priority: string;
  depends_on: number[];
  knowledge_inputs: string[];
  knowledge_outputs: string[];
  tags: string[];
}

export interface WorkflowPhaseTemplate {
  name: string;
  description: string;
  position: number;
  is_recurring: boolean;
  tasks: WorkflowTaskTemplate[];
}

export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  client_type: 'foundation_build' | 'managed_growth' | 'custom';
  is_recurring: boolean;
  phases: WorkflowPhaseTemplate[];
}

export interface DealConversionResult {
  project_id: string;
  project_name: string;
  boards_created: number;
  tasks_created: number;
  dependencies_created: number;
  template_used: string;
}

export interface ConvertDealRequest {
  template_id: string;
  project_name?: string;
  organization_id?: string;
  client_id?: string;
  git_repo_path?: string;
}

export const workflowTemplatesApi = {
  list: async (): Promise<WorkflowTemplate[]> => {
    const response = await makeRequest('/api/workflow-templates');
    return handleApiResponse<WorkflowTemplate[]>(response);
  },

  get: async (id: string): Promise<WorkflowTemplate> => {
    const response = await makeRequest(`/api/workflow-templates/${encodeURIComponent(id)}`);
    return handleApiResponse<WorkflowTemplate>(response);
  },

  convertDeal: async (dealId: string, data: ConvertDealRequest): Promise<DealConversionResult> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/convert`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<DealConversionResult>(response);
  },
};

// Aptos Blockchain Types
export interface AptosBalance {
  address: string;
  balance: number;
  balance_apt: number;
  sequence_number: number;
}

export interface AptosTransaction {
  version: string;
  hash: string;
  sender: string;
  sequence_number: string;
  timestamp: string;
  tx_type: string;
  success: boolean;
  gas_used: string;
  gas_unit_price: string;
  payload_function: string | null;
}

export interface FaucetResponse {
  success: boolean;
  message: string;
  tx_hashes: string[];
}

export interface SendTransactionRequest {
  sender_private_key: string;
  sender_address: string;
  recipient_address: string;
  amount_apt: number;
}

export interface SendTransactionResponse {
  success: boolean;
  tx_hash: string;
  message: string;
}

export interface EstimateGasResponse {
  gas_estimate: number;
  gas_unit_price: number;
  total_gas_apt: number;
}

// VIBE Token Types
export interface VibeBalance {
  address: string;
  balance: number;
  balance_vibe: number;
  equivalent_apt: number;
  usd_value: number;
}

export interface SendVibeRequest {
  sender_private_key: string;
  sender_address: string;
  recipient_address: string;
  amount_vibe: number;
}

export interface VibeTransferResponse {
  success: boolean;
  tx_hash: string;
  amount_vibe: number;
  message: string;
}

// Aptos Testnet API
export const aptosApi = {
  // Get account balance from Aptos testnet
  getBalance: async (address: string): Promise<AptosBalance> => {
    const response = await makeRequest(`/api/aptos/balance/${encodeURIComponent(address)}`);
    return handleApiResponse<AptosBalance>(response);
  },

  // Get recent transactions for an account
  getTransactions: async (address: string, limit?: number): Promise<AptosTransaction[]> => {
    const params = limit ? `?limit=${limit}` : '';
    const response = await makeRequest(`/api/aptos/transactions/${encodeURIComponent(address)}${params}`);
    return handleApiResponse<AptosTransaction[]>(response);
  },

  // Fund account from testnet faucet
  fundFromFaucet: async (address: string, amount?: number): Promise<FaucetResponse> => {
    const params = amount ? `?amount=${amount}` : '';
    const response = await makeRequest(`/api/aptos/faucet/${encodeURIComponent(address)}${params}`, {
      method: 'POST',
    });
    return handleApiResponse<FaucetResponse>(response);
  },

  // Check if account exists on chain
  accountExists: async (address: string): Promise<boolean> => {
    const response = await makeRequest(`/api/aptos/exists/${encodeURIComponent(address)}`);
    return handleApiResponse<boolean>(response);
  },

  // Send APT to another address
  sendApt: async (request: SendTransactionRequest): Promise<SendTransactionResponse> => {
    const response = await makeRequest('/api/aptos/send', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return handleApiResponse<SendTransactionResponse>(response);
  },

  // Estimate gas for a transfer
  estimateGas: async (address: string): Promise<EstimateGasResponse> => {
    const response = await makeRequest(`/api/aptos/estimate-gas/${encodeURIComponent(address)}`);
    return handleApiResponse<EstimateGasResponse>(response);
  },

  // VIBE Token Methods

  // Get VIBE balance for an address
  getVibeBalance: async (address: string): Promise<VibeBalance> => {
    const response = await makeRequest(`/api/vibe/balance/${encodeURIComponent(address)}`);
    return handleApiResponse<VibeBalance>(response);
  },

  // Send VIBE tokens to another address
  sendVibe: async (request: SendVibeRequest): Promise<VibeTransferResponse> => {
    const response = await makeRequest('/api/vibe/send', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return handleApiResponse<VibeTransferResponse>(response);
  },
};

// ============================================
// VIBE Token Economy API
// ============================================

export interface VibeDepositRecord {
  id: string;
  project_id: string;
  tx_hash: string;
  sender_address: string;
  amount_vibe: number;
  status: string;
  payment_method: string;
  credited_at: string | null;
}

export const vibeApi = {
  getConfig: async (): Promise<{ revenue_address: string; network: string; vibe_token_address: string }> => {
    const res = await fetch('/api/vibe/config');
    const data = await res.json();
    return data.data as { revenue_address: string; network: string; vibe_token_address: string };
  },

  verifyDeposit: async (projectId: string, txHash: string, amountVibe: number): Promise<VibeDepositRecord> => {
    const res = await fetch('/api/vibe/deposit/verify', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ project_id: projectId, tx_hash: txHash, amount_vibe: amountVibe }),
    });
    const data = await res.json();
    if (!data.success) throw new Error(data.error_data || 'Deposit verification failed');
    return data.data as VibeDepositRecord;
  },
};

// ============================================
// Model Pricing / Billing Rates API
// ============================================

export interface ModelPricing {
  id: string;
  model: string;
  provider: string;
  input_cost_per_million: number;
  output_cost_per_million: number;
  multiplier: number;
  effective_from: string;
  created_at: string;
}

export interface CostEstimate {
  model: string;
  provider: string;
  input_tokens: number;
  output_tokens: number;
  cost_cents: number;
  cost_vibe: number;
  cost_usd: number;
}

export interface UpsertModelPricing {
  model: string;
  provider: string;
  input_cost_per_million: number;
  output_cost_per_million: number;
  multiplier?: number;
}

export const modelPricingApi = {
  // List all model pricing entries
  list: async (): Promise<ModelPricing[]> => {
    const response = await makeRequest('/api/model-pricing');
    if (!response.ok) {
      throw new ApiError('Failed to load model pricing', response.status, response);
    }
    return response.json();
  },

  // Get pricing for a specific model
  get: async (model: string, provider: string): Promise<ModelPricing> => {
    const response = await makeRequest(`/api/model-pricing/${encodeURIComponent(model)}/${encodeURIComponent(provider)}`);
    if (!response.ok) {
      throw new ApiError('Failed to load model pricing', response.status, response);
    }
    return response.json();
  },

  // Estimate cost for token usage
  estimate: async (model: string, inputTokens: number, outputTokens: number, provider?: string): Promise<CostEstimate> => {
    const params = new URLSearchParams({
      model,
      input_tokens: inputTokens.toString(),
      output_tokens: outputTokens.toString(),
    });
    if (provider) params.set('provider', provider);
    const response = await makeRequest(`/api/model-pricing/estimate?${params.toString()}`);
    if (!response.ok) {
      throw new ApiError('Failed to estimate cost', response.status, response);
    }
    return response.json();
  },

  // Create or update model pricing
  upsert: async (data: UpsertModelPricing): Promise<ModelPricing> => {
    const response = await makeRequest('/api/model-pricing', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      throw new ApiError('Failed to save model pricing', response.status, response);
    }
    return response.json();
  },

  // Delete model pricing
  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/model-pricing/${id}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      throw new ApiError('Failed to delete model pricing', response.status, response);
    }
  },
};

// =====================
// Pulse Engine API
// =====================
export const pulseApi = {
  // Dashboard stats
  getStats: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/stats`);
    return handleApiResponse<{ total_content: number; total_sources: number; active_sources: number; unacknowledged_alerts: number }>(response);
  },

  // Sources
  getSources: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources`);
    return handleApiResponse<any[]>(response);
  },
  createSource: async (projectId: string, data: { source_id: string; source_type: string; name: string; url: string; config?: any; enabled?: boolean; collection_interval_secs?: number; category?: string }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources`, {
      method: 'POST',
      body: JSON.stringify({ ...data, project_id: projectId }),
    });
    return handleApiResponse<any>(response);
  },
  updateSource: async (projectId: string, sourceId: string, data: { name?: string; url?: string; config?: any; enabled?: boolean; collection_interval_secs?: number; category?: string }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources/${sourceId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },
  deleteSource: async (projectId: string, sourceId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources/${sourceId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<any>(response);
  },

  // Content
  getContent: async (projectId: string, params?: { keyword?: string; source_id?: string; status?: string; limit?: number }) => {
    const searchParams = new URLSearchParams();
    if (params?.keyword) searchParams.set('keyword', params.keyword);
    if (params?.source_id) searchParams.set('source_id', params.source_id);
    if (params?.status) searchParams.set('status', params.status);
    if (params?.limit) searchParams.set('limit', String(params.limit));
    const qs = searchParams.toString();
    const response = await makeRequest(`/api/pulse/projects/${projectId}/content${qs ? `?${qs}` : ''}`);
    return handleApiResponse<{ items: any[]; count: number }>(response);
  },
  getLatestContent: async (projectId: string, limit = 20) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/content/latest?limit=${limit}`);
    return handleApiResponse<{ items: any[]; count: number }>(response);
  },
  contentAction: async (projectId: string, contentId: string, action: { action: string; task_title?: string; task_description?: string; crm_contact_id?: string }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/content/${contentId}/action`, {
      method: 'POST',
      body: JSON.stringify(action),
    });
    return handleApiResponse<any>(response);
  },

  // Alerts
  getAlerts: async (projectId: string, limit = 50) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alerts?limit=${limit}`);
    return handleApiResponse<any[]>(response);
  },
  getAlertRules: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules`);
    return handleApiResponse<any[]>(response);
  },
  createAlertRule: async (projectId: string, data: { name: string; conditions: any; actions: any; priority?: string; enabled?: boolean }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules`, {
      method: 'POST',
      body: JSON.stringify({ ...data, project_id: projectId }),
    });
    return handleApiResponse<any>(response);
  },
  updateAlertRule: async (projectId: string, ruleId: string, data: { name?: string; conditions?: any; actions?: any; priority?: string; enabled?: boolean }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules/${ruleId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },
  deleteAlertRule: async (projectId: string, ruleId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules/${ruleId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<any>(response);
  },

  // Collection
  triggerCollection: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/collect`, {
      method: 'POST',
    });
    return handleApiResponse<any>(response);
  },
  getRuns: async (projectId: string, limit = 50) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/runs?limit=${limit}`);
    return handleApiResponse<any[]>(response);
  },

  // Tracking config
  getTrackingConfig: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/tracking`);
    return handleApiResponse<any>(response);
  },
  updateTrackingConfig: async (projectId: string, data: { keywords?: any; entities?: any; llm_enabled?: boolean; llm_model?: string; notification_config?: any }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/tracking`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },

  // Engine status
  getEngineStatus: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/engine/status`);
    return handleApiResponse<any>(response);
  },
};

// ============================================================================
// Sidebar Tree Types
// ============================================================================

export interface SidebarProject {
  id: string;
  name: string;
  is_container: boolean;
  children: SidebarProject[];
  health_status?: string;
  active_issues_count?: number;
  knowledge_completeness?: number;
  last_activity_at?: string;
}

export interface SidebarClient {
  id: string;
  name: string;
  slug: string;
  health_status?: string;
  active_issues_count?: number;
  knowledge_completeness?: number;
  last_activity_at?: string;
  crm_person_id?: string;
  crm_confidence?: number;
  projects: SidebarProject[];
}

export interface SidebarSharedBoard {
  board_id: string;
  board_name: string;
  project_id: string;
  project_name: string;
  permission: string;
  share_type: string;
}

export interface SidebarSharedBoardGroup {
  source_org_id: string;
  source_org_name: string;
  share_type: string;
  boards: SidebarSharedBoard[];
}

export interface SidebarOrg {
  id: string;
  name: string;
  slug: string;
  role: string;
  health_status?: string;
  active_issues_count?: number;
  knowledge_completeness?: number;
  last_activity_at?: string;
  internal_projects: SidebarProject[];
  clients: SidebarClient[];
  shared_boards: SidebarSharedBoardGroup[];
}

export interface SidebarTree {
  owned_orgs: SidebarOrg[];
  member_orgs: SidebarOrg[];
}

export interface OrganizationData {
  id: string;
  name: string;
  slug: string;
  description?: string;
  avatar_url?: string;
  owner_id: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  address?: string;
}

export interface ClientData {
  id: string;
  organization_id: string;
  name: string;
  slug: string;
  description?: string;
  logo_url?: string;
  website?: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Organizations & Clients API
// ============================================================================

export const organizationsApi = {
  // Sidebar tree
  getSidebarTree: async (): Promise<SidebarTree> => {
    const response = await makeRequest('/api/sidebar/tree');
    return handleApiResponse<SidebarTree>(response);
  },

  // Organizations
  getAll: async (): Promise<OrganizationData[]> => {
    const response = await makeRequest('/api/organizations');
    return handleApiResponse<OrganizationData[]>(response);
  },

  getById: async (id: string): Promise<OrganizationData> => {
    const response = await makeRequest(`/api/organizations/${id}`);
    return handleApiResponse<OrganizationData>(response);
  },

  create: async (data: { name: string; slug: string; description?: string }): Promise<OrganizationData> => {
    const response = await makeRequest('/api/organizations', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrganizationData>(response);
  },

  update: async (id: string, data: { name?: string; slug?: string; description?: string }): Promise<OrganizationData> => {
    const response = await makeRequest(`/api/organizations/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrganizationData>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  activate: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${id}/activate`, {
      method: 'PATCH',
    });
    return handleApiResponse<void>(response);
  },

  deactivate: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${id}/deactivate`, {
      method: 'PATCH',
    });
    return handleApiResponse<void>(response);
  },

  // Members
  getMembers: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members`);
    return handleApiResponse<any[]>(response);
  },

  addMember: async (orgId: string, userId: string, role?: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members`, {
      method: 'POST',
      body: JSON.stringify({ user_id: userId, role }),
    });
    return handleApiResponse<any>(response);
  },

  removeMember: async (orgId: string, userId: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  changeMemberRole: async (orgId: string, userId: string, role: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/role`, {
      method: 'PUT',
      body: JSON.stringify({ role }),
    });
    return handleApiResponse<any>(response);
  },

  // Member assignments
  getMemberAssignments: async (orgId: string, userId: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assignments`);
    return handleApiResponse<any>(response);
  },

  assignMember: async (orgId: string, userId: string, type: string, targetId: string, role?: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assign`, {
      method: 'POST',
      body: JSON.stringify({ type, target_id: targetId, role }),
    });
    return handleApiResponse<any>(response);
  },

  watchTaskForMember: async (orgId: string, userId: string, taskId: string): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/watch`, {
      method: 'POST',
      body: JSON.stringify({ task_id: taskId }),
    });
    return handleApiResponse<any>(response);
  },

  unassignProject: async (orgId: string, userId: string, projectId: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assignments/project/${projectId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  unassignClient: async (orgId: string, userId: string, clientId: string): Promise<void> => {
    const response = await makeRequest(`/api/organizations/${orgId}/members/${userId}/assignments/client/${clientId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Org invitations
  createInvitation: async (orgId: string, role?: string, maxUses?: number, expiresInHours?: number): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/invitations`, {
      method: 'POST',
      body: JSON.stringify({ role, max_uses: maxUses, expires_in_hours: expiresInHours }),
    });
    return handleApiResponse<any>(response);
  },

  listInvitations: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/invitations`);
    return handleApiResponse<any[]>(response);
  },

  // Clients
  getClients: async (orgId: string): Promise<ClientData[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/clients`);
    return handleApiResponse<ClientData[]>(response);
  },

  createClient: async (orgId: string, data: { name: string; slug: string; description?: string; website?: string }): Promise<ClientData> => {
    const response = await makeRequest(`/api/organizations/${orgId}/clients`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ClientData>(response);
  },

  updateClient: async (clientId: string, data: { name?: string; slug?: string; description?: string; website?: string }): Promise<ClientData> => {
    const response = await makeRequest(`/api/clients/${clientId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ClientData>(response);
  },

  deleteClient: async (clientId: string): Promise<void> => {
    const response = await makeRequest(`/api/clients/${clientId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Board Shares
  getBoardShares: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/board-shares`);
    return handleApiResponse<any[]>(response);
  },

  getSharedBoards: async (orgId: string): Promise<any[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/shared-boards`);
    return handleApiResponse<any[]>(response);
  },

  createBoardShare: async (orgId: string, data: { board_id: string; target_organization_id: string; permission?: string; share_type?: string }): Promise<any> => {
    const response = await makeRequest(`/api/organizations/${orgId}/board-shares`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },

  updateBoardShare: async (shareId: string, data: { permission?: string; share_type?: string; is_active?: boolean }): Promise<any> => {
    const response = await makeRequest(`/api/board-shares/${shareId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<any>(response);
  },

  deleteBoardShare: async (shareId: string): Promise<void> => {
    const response = await makeRequest(`/api/board-shares/${shareId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Person-org junction (for context badges)
  listPersonContacts: async (orgId: string): Promise<PersonOrgContact[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/person-contacts`);
    return handleApiResponse<PersonOrgContact[]>(response);
  },
  addPersonContact: async (orgId: string, data: { person_id: string; context?: string }): Promise<PersonOrgContact> => {
    const response = await makeRequest(`/api/organizations/${orgId}/person-contacts`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonOrgContact>(response);
  },

  // Brand profile
  getBrandProfile: async (orgId: string): Promise<OrgBrandProfile | null> => {
    const response = await makeRequest(`/api/organizations/${orgId}/brand-profile`);
    return handleApiResponse<OrgBrandProfile | null>(response);
  },
  upsertBrandProfile: async (orgId: string, data: Partial<OrgBrandProfile>): Promise<OrgBrandProfile> => {
    const response = await makeRequest(`/api/organizations/${orgId}/brand-profile`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrgBrandProfile>(response);
  },
  triggerBrandResearch: async (orgId: string): Promise<{ orgId: string; status: string; message: string }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/brand-research`, { method: 'POST' });
    return handleApiResponse(r);
  },
  seedBrandProject: async (orgId: string) => {
    const r = await fetch(`/api/organizations/${orgId}/seed-brand-project`, {
      method: 'POST',
      credentials: 'include',
    });
    return r.json();
  },
  getBrandResearchStatus: async (orgId: string): Promise<{ orgId: string; status: string; summary?: string; ranAt?: string }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/brand-research/status`);
    return handleApiResponse(r);
  },
  generateIntakeToken: async (orgId: string): Promise<{ token: string; url: string; expiresAt: string }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/intake-token`, { method: 'POST' });
    return handleApiResponse(r);
  },
  getKnowledge: async (orgId: string): Promise<{ knowledge_entries: OrgKnowledgeSource[]; stats: { knowledge_entry_count: number; data_source_count: number; avg_coverage: number } }> => {
    const r = await makeRequest(`/api/organizations/${orgId}/knowledge`);
    return handleApiResponse(r);
  },
};

// ============================================================================
// Brand Guide Types & Intake API
// ============================================================================

export interface OrgBrandProfile {
  id: string;
  organizationId: string;
  tagline?: string | null;
  primaryColor: string;
  secondaryColor: string;
  accentColor?: string | null;
  typographyHeading?: string | null;
  typographyBody?: string | null;
  logoUrl?: string | null;
  industry?: string | null;
  marketPosition?: string | null;
  uniqueValueProposition?: string | null;
  missionStatement?: string | null;
  visionStatement?: string | null;
  brandValues?: string | null;
  brandVoice?: string | null;
  brandArchetype?: string | null;
  targetAudience?: string | null;
  icpDescription?: string | null;
  icpCompanySize?: string | null;
  icpIndustries?: string | null;
  competitorBrands?: string | null;
  differentiators?: string | null;
  contentPillars?: string | null;
  contentTone?: string | null;
  websiteUrl?: string | null;
  socialInstagram?: string | null;
  socialTwitter?: string | null;
  socialLinkedin?: string | null;
  socialFacebook?: string | null;
  socialYoutube?: string | null;
  socialTiktok?: string | null;
  researchStatus: string;
  researchRanAt?: string | null;
  researchSummary?: string | null;
  moodBoardUrls?: string | null;
  clearbitLogoUrl?: string | null;
  brandPhotographyNotes?: string | null;
  researchIterations?: number;
  researchDepth?: number;
  founderName?: string | null;
  foundingYear?: string | null;
  keyClients?: string | null;
  estimatedTeamSize?: string | null;
  techStack?: string | null;
  geographicFocus?: string | null;
  fundingStage?: string | null;
  contentStrategyNotes?: string | null;
  awardsAndRecognition?: string | null;
  brandGapNotes?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface OrgKnowledgeSource {
  id: string;
  source_type: string;
  source_title: string;
  source_summary?: string | null;
  coverage_score: number;
  is_active: boolean;
  is_stale: boolean;
  project_id?: string | null;
  owner_type?: string | null;
  created_at: string;
  updated_at: string;
}

export const intakeApi = {
  getContext: async (token: string): Promise<{ orgName: string; orgId: string; existing: Record<string, string | null> }> => {
    const r = await makeRequest(`/api/intake/${token}`);
    return handleApiResponse(r);
  },
  submit: async (token: string, data: Record<string, string>): Promise<{ message: string }> => {
    const r = await makeRequest(`/api/intake/${token}`, { method: 'POST', body: JSON.stringify(data) });
    return handleApiResponse(r);
  },
};

// ============================================================================
// Project Folders API
// ============================================================================

export interface ProjectFolderData {
  id: string;
  organization_id: string;
  client_id?: string;
  name: string;
  sort_order: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Knowledge API
// ============================================================================

export interface ProjectKnowledgeSource {
  id: string;
  project_id: string;
  source_type: string;
  source_id: string;
  source_title: string;
  source_summary?: string;
  coverage_score: number;
  is_active: boolean;
  is_stale: boolean;
  auto_registered: boolean;
  last_refreshed_at: string;
  created_at: string;
  updated_at: string;
}

export interface ProjectKnowledgeCompleteness {
  project_id: string;
  total_sources: number;
  fresh_sources: number;
  avg_coverage: number;
  type_count: number;
  knowledge_completeness: number;
}

export interface ProjectKnowledgeResponse {
  project_id: string;
  completeness?: ProjectKnowledgeCompleteness;
  total_sources: number;
  stale_count: number;
  sources_by_type: Record<string, ProjectKnowledgeSource[]>;
}

export interface CreateKnowledgeSourceRequest {
  source_type: string;
  source_title: string;
  source_summary?: string;
  coverage_score?: number;
}

export const knowledgeApi = {
  getProjectKnowledge: async (projectId: string): Promise<ProjectKnowledgeResponse> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge`);
    return handleApiResponse<ProjectKnowledgeResponse>(response);
  },

  createSource: async (projectId: string, data: CreateKnowledgeSourceRequest): Promise<ProjectKnowledgeSource> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProjectKnowledgeSource>(response);
  },

  refreshSource: async (projectId: string, sourceId: string): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge/${sourceId}/refresh`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  markStale: async (projectId: string, sourceId: string): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/knowledge/${sourceId}/stale`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },
};

// projectFoldersApi removed — projects now use parent_project_id nesting via projectsApi.setParent()

// ============================================================================
// Entity Conversion API
// ============================================================================

export type EntityType = 'organization' | 'client' | 'project';

export interface ConvertEntityRequest {
  source_type: EntityType;
  source_id: string;
  target_type: EntityType;
  target_parent_id?: string;
}

export interface ConvertEntityResponse {
  new_id: string;
  new_type: EntityType;
}

export const entityConversionApi = {
  convert: async (data: ConvertEntityRequest): Promise<ConvertEntityResponse> => {
    const response = await makeRequest('/api/entities/convert', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ConvertEntityResponse>(response);
  },
};

// ============================================================================
// AGENT EXECUTION CONFIG & RALPH
// ============================================================================

export type ExecutionMode = 'standard' | 'ralph' | 'parallel' | 'pipeline';
export type RalphLoopStatus = 'initializing' | 'running' | 'validating' | 'complete' | 'maxreached' | 'failed' | 'cancelled';

export interface AgentExecutionProfile {
  id: string;
  name: string;
  description: string | null;
  execution_mode: ExecutionMode;
  max_iterations: number | null;
  completion_promise: string | null;
  exit_signal_key: string | null;
  backpressure_commands: string | null;
  iteration_delay_ms: number | null;
  iteration_timeout_ms: number | null;
  total_timeout_ms: number | null;
  preserve_session: boolean | null;
}

export interface AgentExecutionConfig {
  id: string;
  agent_id: string;
  execution_profile_id: string | null;
  execution_mode_override: ExecutionMode | null;
  max_iterations_override: number | null;
  backpressure_commands_override: string | null;
  system_prompt_prefix: string | null;
  system_prompt_suffix: string | null;
  auto_commit_on_success: boolean | null;
  auto_create_pr_on_complete: boolean | null;
  require_tests_pass: boolean | null;
  is_active: boolean | null;
  created_at: string;
  updated_at: string;
  type?: 'Found';
}

export interface CreateAgentExecutionConfig {
  execution_profile_id?: string | null;
  execution_mode_override?: ExecutionMode | null;
  max_iterations_override?: number | null;
  backpressure_commands_override?: string;
  system_prompt_prefix?: string;
  system_prompt_suffix?: string;
  auto_commit_on_success?: boolean;
  auto_create_pr_on_complete?: boolean;
  require_tests_pass?: boolean;
}

export interface UpdateAgentExecutionConfig extends CreateAgentExecutionConfig {}

export interface RalphLoopState {
  id: string;
  task_attempt_id: string;
  agent_id: string | null;
  current_iteration: number;
  max_iterations: number;
  session_id: string | null;
  status: RalphLoopStatus;
  completion_promise: string | null;
  completion_detected_at: string | null;
  final_validation_passed: boolean | null;
  total_tokens_used: number | null;
  total_cost_cents: number | null;
  started_at: string;
  completed_at: string | null;
  last_iteration_at: string | null;
  last_error: string | null;
  consecutive_failures: number | null;
  type?: 'Found';
}

export interface RalphIteration {
  id: string;
  ralph_loop_id: string;
  execution_process_id: string | null;
  iteration_number: number;
  status: string;
  completion_signal_found: boolean | null;
  exit_signal_found: boolean | null;
  all_backpressure_passed: boolean | null;
  tokens_used: number | null;
  cost_cents: number | null;
  duration_ms: number | null;
  started_at: string;
  completed_at: string | null;
  output_summary: string | null;
  files_modified: number | null;
  commits_made: number | null;
}

export const agentExecutionConfigApi = {
  listProfiles: async (): Promise<AgentExecutionProfile[]> => {
    const response = await makeRequest('/api/execution-profiles');
    return handleApiResponse<AgentExecutionProfile[]>(response);
  },
  getAgentConfig: async (agentId: string): Promise<AgentExecutionConfig | { type: 'NotFound' }> => {
    const response = await makeRequest(`/api/agents/${agentId}/execution-config`);
    return handleApiResponse<AgentExecutionConfig | { type: 'NotFound' }>(response);
  },
  createAgentConfig: async (agentId: string, data: CreateAgentExecutionConfig): Promise<AgentExecutionConfig> => {
    const response = await makeRequest(`/api/agents/${agentId}/execution-config`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentExecutionConfig>(response);
  },
  updateAgentConfig: async (agentId: string, data: UpdateAgentExecutionConfig): Promise<AgentExecutionConfig> => {
    const response = await makeRequest(`/api/agents/${agentId}/execution-config`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<AgentExecutionConfig>(response);
  },
};

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

// ============================================================================
// EMAIL MESSAGES
// ============================================================================

export interface EmailMessageRecord {
  id: string;
  email_account_id: string;
  project_id: string;
  provider_message_id: string;
  thread_id: string | null;
  from_address: string;
  from_name: string | null;
  to_addresses: string;
  subject: string | null;
  body_text: string | null;
  body_html: string | null;
  snippet: string | null;
  has_attachments: number;
  is_read: number;
  is_starred: number;
  is_draft: number;
  is_sent: number;
  is_archived: number;
  is_trash: number;
  sentiment: string | null;
  priority: string | null;
  needs_response: number;
  received_at: string;
  created_at: string;
  updated_at: string;
}

export interface EmailInboxStats {
  total: number;
  unread: number;
  starred: number;
  needs_response: number;
  by_account: Array<{ account_id: string; email_address: string; total: number; unread: number }>;
}

export const emailMessagesApi = {
  listMessages: async (params: {
    project_id: string;
    email_account_id?: string;
    is_read?: boolean;
    is_starred?: boolean;
    needs_response?: boolean;
    limit?: number;
  }): Promise<EmailMessageRecord[]> => {
    const qs = new URLSearchParams({ project_id: params.project_id });
    if (params.email_account_id) qs.set('email_account_id', params.email_account_id);
    if (params.is_read !== undefined) qs.set('is_read', String(params.is_read));
    if (params.is_starred !== undefined) qs.set('is_starred', String(params.is_starred));
    if (params.needs_response !== undefined) qs.set('needs_response', String(params.needs_response));
    if (params.limit) qs.set('limit', String(params.limit));
    const response = await makeRequest(`/api/email/messages?${qs}`);
    return handleApiResponse<EmailMessageRecord[]>(response);
  },
  getInboxStats: async (projectId: string): Promise<EmailInboxStats> => {
    const response = await makeRequest(`/api/email/messages/stats/${projectId}`);
    return handleApiResponse<EmailInboxStats>(response);
  },
  markAsRead: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/read`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
  toggleStar: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/star`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
  moveToTrash: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/trash`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
};

// ============================================================
// Universal Persons API
// ============================================================

export interface PersonRecord {
  id: string;
  full_name: string;
  email?: string;
  phone?: string;
  avatar_url?: string;
  person_type: string;
  financial_role: string;
  client_profile?: string;
  business_stage?: string;
  lifecycle_stage: string;
  lead_score: number;
  company_name?: string;
  job_title?: string;
  website?: string;
  user_id?: string;
  crm_contact_id?: string;
  organization_id?: string;
  intelligence_summary?: string;
  intelligence_raw?: string;
  intelligence_last_run_at?: string;
  intelligence_confidence: number;
  intelligence_status?: 'idle' | 'queued' | 'running' | 'done' | 'failed';
  intelligence_agent?: string;
  research_pass_count?: number;
  research_depth?: 'shallow' | 'moderate' | 'deep';
  notes?: string;
  tags: string;
  custom_fields: string;
  /** How this person first engaged: 'email'|'instagram'|'whatsapp'|'linkedin'|'twitter'|'sms'|'phone'|'in_person' */
  onboarding_channel?: string;
  /** Preferred outbound contact channel */
  preferred_contact?: string;
  created_at: string;
  updated_at: string;
}

export interface PersonSocialProfile {
  id: string;
  person_id: string;
  platform: string;
  handle?: string;
  profile_url?: string;
  follower_count?: number;
  following_count?: number;
  bio?: string;
  verified: number;
  last_synced_at?: string;
  created_at: string;
  updated_at: string;
}

export interface PersonCompanyRole {
  id: string;
  person_id: string;
  company_id: string;
  role: string;
  title?: string;
  is_primary: number;
  start_date?: string;
  end_date?: string;
  notes?: string;
  created_at: string;
  company_name?: string;
  company_slug?: string;
}

export interface PersonOrgContact {
  id: string;
  person_id: string;
  organization_id: string;
  context: string;
  notes?: string;
  added_at: string;
  org_name?: string;
}

export interface CompanyContactMethod {
  id: string;
  company_id: string;
  method_type: string;
  label?: string;
  value: string;
  is_primary: number;
  created_at: string;
}

export interface PersonWithSocials extends PersonRecord {
  social_profiles: PersonSocialProfile[];
  company_roles: PersonCompanyRole[];
  org_contacts: PersonOrgContact[];
}

export interface InvoiceRecord {
  id: string;
  invoice_number: string;
  person_id?: string;
  organization_id?: string;
  project_id?: string;
  invoice_type: string;
  status: string;
  amount_usd: number;
  amount_vibe: number;
  currency: string;
  title?: string;
  description?: string;
  line_items: string;
  issue_date?: string;
  due_date?: string;
  paid_at?: string;
  payment_method?: string;
  payment_reference?: string;
  notes?: string;
  created_at: string;
  updated_at: string;
}

export interface CreatePersonInput {
  full_name: string;
  email?: string;
  phone?: string;
  person_type?: string;
  financial_role?: string;
  client_profile?: string;
  business_stage?: string;
  lifecycle_stage?: string;
  company_name?: string;
  job_title?: string;
  website?: string;
  notes?: string;
  tags?: string[];
}

export interface UpdatePersonInput {
  full_name?: string;
  email?: string;
  phone?: string;
  person_type?: string;
  financial_role?: string;
  client_profile?: string;
  business_stage?: string;
  lifecycle_stage?: string;
  lead_score?: number;
  company_name?: string;
  job_title?: string;
  website?: string;
  notes?: string;
  tags?: string[];
  intelligence_summary?: string;
  onboarding_channel?: string;
  preferred_contact?: string;
}

export const personsApi = {
  list: async (params?: {
    person_type?: string;
    financial_role?: string;
    lifecycle_stage?: string;
    organization_id?: string;
    q?: string;
    limit?: number;
    offset?: number;
  }): Promise<PersonRecord[]> => {
    const qs = new URLSearchParams();
    if (params?.person_type) qs.set('person_type', params.person_type);
    if (params?.financial_role) qs.set('financial_role', params.financial_role);
    if (params?.lifecycle_stage) qs.set('lifecycle_stage', params.lifecycle_stage);
    if (params?.organization_id) qs.set('organization_id', params.organization_id);
    if (params?.q) qs.set('q', params.q);
    if (params?.limit) qs.set('limit', String(params.limit));
    if (params?.offset) qs.set('offset', String(params.offset));
    const response = await makeRequest(`/api/persons?${qs}`);
    return handleApiResponse<PersonRecord[]>(response);
  },

  get: async (id: string): Promise<PersonWithSocials> => {
    const response = await makeRequest(`/api/persons/${id}`);
    return handleApiResponse<PersonWithSocials>(response);
  },

  create: async (data: CreatePersonInput): Promise<PersonRecord> => {
    const response = await makeRequest('/api/persons', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonRecord>(response);
  },

  update: async (id: string, data: UpdatePersonInput): Promise<PersonRecord> => {
    const response = await makeRequest(`/api/persons/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  listSocialProfiles: async (id: string): Promise<PersonSocialProfile[]> => {
    const response = await makeRequest(`/api/persons/${id}/social-profiles`);
    return handleApiResponse<PersonSocialProfile[]>(response);
  },

  upsertSocialProfile: async (
    id: string,
    data: {
      platform: string;
      handle?: string;
      profile_url?: string;
      follower_count?: number;
      bio?: string;
      verified?: boolean;
    }
  ): Promise<PersonSocialProfile> => {
    const response = await makeRequest(`/api/persons/${id}/social-profiles`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonSocialProfile>(response);
  },

  deleteSocialProfile: async (id: string, platform: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}/social-profiles/${platform}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  listInvoices: async (id: string): Promise<InvoiceRecord[]> => {
    const response = await makeRequest(`/api/persons/${id}/invoices`);
    return handleApiResponse<InvoiceRecord[]>(response);
  },

  // Company affiliations
  listCompanies: async (id: string): Promise<PersonCompanyRole[]> => {
    const response = await makeRequest(`/api/persons/${id}/companies`);
    return handleApiResponse<PersonCompanyRole[]>(response);
  },
  addCompany: async (id: string, data: { company_id: string; role?: string; title?: string; is_primary?: boolean }): Promise<PersonCompanyRole> => {
    const response = await makeRequest(`/api/persons/${id}/companies`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonCompanyRole>(response);
  },
  updateCompanyRole: async (id: string, company_id: string, data: { role?: string; title?: string; is_primary?: boolean }): Promise<PersonCompanyRole> => {
    const response = await makeRequest(`/api/persons/${id}/companies/${company_id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonCompanyRole>(response);
  },
  removeCompany: async (id: string, company_id: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}/companies/${company_id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  // Org affiliations
  listOrgs: async (id: string): Promise<PersonOrgContact[]> => {
    const response = await makeRequest(`/api/persons/${id}/organizations`);
    return handleApiResponse<PersonOrgContact[]>(response);
  },
  addOrg: async (id: string, data: { organization_id: string; context?: string }): Promise<PersonOrgContact> => {
    const response = await makeRequest(`/api/persons/${id}/organizations`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonOrgContact>(response);
  },
  removeOrg: async (id: string, org_id: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}/organizations/${org_id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ── Proposals ─────────────────────────────────────────────────────────────────

export type ProposalStatus =
  | 'drafted'
  | 'pending_approval'
  | 'approved'
  | 'meeting_scheduled'
  | 'sent'
  | 'seen'
  | 'verbal'
  | 'contract_signed'
  | 'declined'
  | 'deferred';

export type DealType = 'one-off' | 'retainer' | 'hybrid';

export interface ProposalRecord {
  id: string;
  lead_id?: string;
  organization_id?: string;
  owner_id?: string;
  project_id?: string;
  company_id?: string;
  contact_ids: string;
  status: ProposalStatus;
  title: string;
  description: string;
  quote_amount_vibe: number;
  deal_type: DealType;
  sent_at?: string;
  seen_at?: string;
  verbal_at?: string;
  signed_at?: string;
  declined_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateProposalInput {
  title: string;
  lead_id?: string;
  organization_id?: string;
  owner_id?: string;
  project_id?: string;
  company_id?: string;
  description?: string;
  quote_amount_vibe?: number;
  deal_type?: DealType;
}

export interface UpdateProposalInput {
  title?: string;
  description?: string;
  quote_amount_vibe?: number;
  deal_type?: DealType;
  lead_id?: string;
  project_id?: string;
  owner_id?: string;
}

export const proposalsApi = {
  list: async (params?: {
    status?: string;
    lead_id?: string;
    project_id?: string;
    owner_id?: string;
    organization_id?: string;
    limit?: number;
  }): Promise<ProposalRecord[]> => {
    const qs = params ? '?' + new URLSearchParams(
      Object.entries(params)
        .filter(([, v]) => v != null)
        .map(([k, v]) => [k, String(v)])
    ) : '';
    const response = await makeRequest(`/api/proposals${qs}`);
    return handleApiResponse<ProposalRecord[]>(response);
  },

  get: async (id: string): Promise<ProposalRecord> => {
    const response = await makeRequest(`/api/proposals/${id}`);
    return handleApiResponse<ProposalRecord>(response);
  },

  create: async (data: CreateProposalInput): Promise<ProposalRecord> => {
    const response = await makeRequest('/api/proposals', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProposalRecord>(response);
  },

  update: async (id: string, data: UpdateProposalInput): Promise<ProposalRecord> => {
    const response = await makeRequest(`/api/proposals/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProposalRecord>(response);
  },

  moveStatus: async (id: string, status: ProposalStatus): Promise<ProposalRecord> => {
    const response = await makeRequest(`/api/proposals/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<ProposalRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/proposals/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  scheduleMeeting: async (
    id: string,
    data: {
      scheduled_at: string;
      duration_min?: number;
      location?: string;
      agenda?: string;
      channel?: string;
      invitees: Array<{ person_id: string; channel?: string; channel_address?: string }>;
    }
  ): Promise<{ meeting: ScheduledMeetingRecord; dispatched: InviteDispatchResult[] }> => {
    const response = await makeRequest(`/api/proposals/${id}/schedule-meeting`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse(response);
  },

  listScheduledMeetings: async (id: string): Promise<ScheduledMeetingRecord[]> => {
    const response = await makeRequest(`/api/proposals/${id}/scheduled-meetings`);
    return handleApiResponse(response);
  },
};

// ── Deliverables ──────────────────────────────────────────────────────────────

export type DeliverableType = 'video' | 'audio' | 'graphic' | 'copy' | 'code' | 'document' | 'other';
export type DeliverableStatus =
  | 'working'
  | 'internal_review'
  | 'client_review'
  | 'revision'
  | 'client_revision'
  | 'done';

export interface DeliverableRecord {
  id: string;
  project_id: string;
  proposal_id?: string;
  deliverable_type: DeliverableType;
  title: string;
  description: string;
  status: DeliverableStatus;
  revision_rounds_allowed: number;
  revision_rounds_used: number;
  working_file_url?: string;
  final_link?: string;
  due_date?: string;
  delivered_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateDeliverableInput {
  project_id: string;
  proposal_id?: string;
  deliverable_type?: DeliverableType;
  title: string;
  description?: string;
  revision_rounds_allowed?: number;
  working_file_url?: string;
  due_date?: string;
}

export const deliverablesApi = {
  listForProject: async (projectId: string): Promise<DeliverableRecord[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/deliverables`);
    return handleApiResponse<DeliverableRecord[]>(response);
  },

  create: async (data: CreateDeliverableInput): Promise<DeliverableRecord> => {
    const response = await makeRequest(`/api/projects/${data.project_id}/deliverables`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<DeliverableRecord>(response);
  },

  update: async (id: string, data: Partial<CreateDeliverableInput> & { final_link?: string }): Promise<DeliverableRecord> => {
    const response = await makeRequest(`/api/deliverables/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<DeliverableRecord>(response);
  },

  moveStatus: async (id: string, status: DeliverableStatus): Promise<DeliverableRecord> => {
    const response = await makeRequest(`/api/deliverables/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<DeliverableRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/deliverables/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ── Command Center ────────────────────────────────────────────────────────────

export interface CommandCenterSnapshot {
  overdue_tasks: Array<{
    id: string; title: string; project_name: string; due_date: string; assignee_name?: string;
  }>;
  deliverables_due_this_week: Array<{
    id: string; title: string; deliverable_type: string; project_name: string;
    status: string; due_date: string;
  }>;
  waiting_on_client_projects: Array<{
    id: string; name: string; client_name?: string; updated_at: string;
  }>;
  follow_up_required: Array<{
    id: string; full_name: string; email?: string;
    follow_up_attempts: number; lifecycle_stage: string;
  }>;
  proposals_awaiting_approval: Array<{
    id: string; title: string; lead_name?: string;
    quote_amount_vibe: number; created_at: string;
  }>;
  closed_unpaid_projects: Array<{
    id: string; name: string; client_name?: string; updated_at: string;
  }>;
}

export const commandCenterApi = {
  get: async (orgId?: string): Promise<CommandCenterSnapshot> => {
    const qs = orgId ? `?org_id=${orgId}` : '';
    const response = await makeRequest(`/api/command-center${qs}`);
    return handleApiResponse<CommandCenterSnapshot>(response);
  },
};

// ── Invoices ──────────────────────────────────────────────────────────────────

export interface CreateInvoiceInput {
  person_id?: string;
  organization_id?: string;
  project_id?: string;
  invoice_type?: 'ar' | 'ap';
  title?: string;
  description?: string;
  amount_usd?: number;
  amount_vibe?: number;
  currency?: string;
  issue_date?: string;
  due_date?: string;
  notes?: string;
}

export const invoicesApi = {
  list: async (params?: { invoice_type?: string; status?: string; person_id?: string; project_id?: string; limit?: number }): Promise<InvoiceRecord[]> => {
    const qs = params ? '?' + new URLSearchParams(Object.fromEntries(Object.entries(params).filter(([, v]) => v != null).map(([k, v]) => [k, String(v)]))).toString() : '';
    const response = await makeRequest(`/api/invoices${qs}`);
    return handleApiResponse<InvoiceRecord[]>(response);
  },

  create: async (data: CreateInvoiceInput): Promise<InvoiceRecord> => {
    const response = await makeRequest('/api/invoices', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<InvoiceRecord>(response);
  },

  get: async (id: string): Promise<InvoiceRecord> => {
    const response = await makeRequest(`/api/invoices/${id}`);
    return handleApiResponse<InvoiceRecord>(response);
  },

  update: async (id: string, data: Partial<CreateInvoiceInput>): Promise<InvoiceRecord> => {
    const response = await makeRequest(`/api/invoices/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<InvoiceRecord>(response);
  },

  moveStatus: async (id: string, status: string): Promise<InvoiceRecord> => {
    const response = await makeRequest(`/api/invoices/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<InvoiceRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/invoices/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ── Intelligence ──────────────────────────────────────────────────────────────

export interface IntelligenceStatus {
  person_id: string;
  status: 'idle' | 'queued' | 'running' | 'done' | 'failed';
  summary?: string;
  confidence: number;
  agent?: string;
  last_run_at?: string;
}

// ============================================================
// Companies API (knowledge-graph company entities)
// ============================================================

export interface CompanyRecord {
  id: string;
  name: string;
  slug?: string | null;
  website?: string | null;
  industry?: string | null;
  description?: string | null;
  logo_url?: string | null;
  cover_image_url?: string | null;
  headquarters?: string | null;
  address?: string | null;
  city?: string | null;
  country?: string | null;
  phone?: string | null;
  email?: string | null;
  whatsapp?: string | null;
  instagram_handle?: string | null;
  linkedin_url?: string | null;
  twitter_handle?: string | null;
  facebook_url?: string | null;
  founded_year?: number | null;
  employee_count?: string | null;
  tags?: string | null;
  business_hours?: string | null;
  notes?: string | null;
  gmb_rating?: number | null;
  gmb_review_count?: number | null;
  gmb_place_id?: string | null;
  intelligence_summary?: string | null;
  intelligence_raw?: string | null;
  intelligence_status: string;
  intelligence_last_run_at?: string | null;
  intelligence_confidence?: number | null;
  intelligence_agent?: string | null;
  organization_id?: string | null;
  created_by_org_id?: string | null;
  created_at: string;
  updated_at: string;
}

export const companiesApi = {
  list: async (params?: { limit?: number; has_platform_org?: boolean; created_by_org_id?: string }): Promise<CompanyRecord[]> => {
    const qs = new URLSearchParams();
    if (params?.limit != null) qs.set('limit', String(params.limit));
    if (params?.has_platform_org != null) qs.set('has_platform_org', String(params.has_platform_org));
    if (params?.created_by_org_id != null) qs.set('created_by_org_id', params.created_by_org_id);
    const response = await makeRequest(`/api/companies?${qs.toString()}`);
    return handleApiResponse<CompanyRecord[]>(response);
  },

  get: async (id: string): Promise<CompanyRecord> => {
    const response = await makeRequest(`/api/companies/${id}`);
    return handleApiResponse<CompanyRecord>(response);
  },

  create: async (data: { name: string; website?: string; industry?: string; description?: string; headquarters?: string; created_by_org_id?: string }): Promise<CompanyRecord> => {
    const response = await makeRequest('/api/companies', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CompanyRecord>(response);
  },

  update: async (id: string, data: Partial<CompanyRecord>): Promise<CompanyRecord> => {
    const response = await makeRequest(`/api/companies/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CompanyRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/companies/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  listProposals: async (id: string): Promise<ProposalRecord[]> => {
    const response = await makeRequest(`/api/companies/${id}/proposals`);
    return handleApiResponse<ProposalRecord[]>(response);
  },

  listPersons: async (id: string): Promise<PersonRecord[]> => {
    const response = await makeRequest(`/api/companies/${id}/persons`);
    return handleApiResponse<PersonRecord[]>(response);
  },

  getIntelligenceStatus: async (id: string): Promise<{ status: string; summary?: string; confidence: number; agent?: string; last_run_at?: string }> => {
    const company = await companiesApi.get(id);
    return {
      status: company.intelligence_status,
      summary: company.intelligence_summary ?? undefined,
      confidence: company.intelligence_confidence ?? 0,
      agent: company.intelligence_agent ?? undefined,
      last_run_at: company.intelligence_last_run_at ?? undefined,
    };
  },

  research: async (id: string): Promise<{ status: string; message: string }> => {
    const response = await makeRequest(`/api/companies/${id}/research`, {
      method: 'POST',
      body: JSON.stringify({}),
    });
    return handleApiResponse<{ status: string; message: string }>(response);
  },

  exportAnalysis: async (id: string, companyName?: string): Promise<void> => {
    const response = await makeRequest(`/api/companies/${id}/export-analysis`);
    if (!response.ok) throw new Error('Export failed');
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = companyName ? `PCG_Analysis_${companyName.replace(/\s+/g, '_')}.md` : 'PCG_Analysis.md';
    a.click();
    URL.revokeObjectURL(url);
  },

  listContactMethods: async (id: string): Promise<CompanyContactMethod[]> => {
    const response = await makeRequest(`/api/companies/${id}/contact-methods`);
    return handleApiResponse<CompanyContactMethod[]>(response);
  },
  addContactMethod: async (id: string, data: { method_type: string; label?: string; value: string; is_primary?: boolean }): Promise<CompanyContactMethod> => {
    const response = await makeRequest(`/api/companies/${id}/contact-methods`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CompanyContactMethod>(response);
  },
  removeContactMethod: async (id: string, method_id: string): Promise<void> => {
    const response = await makeRequest(`/api/companies/${id}/contact-methods/${method_id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ── Scheduled Meeting types ───────────────────────────────────────────────────

export interface ScheduledMeetingInviteeRecord {
  id: string;
  scheduled_meeting_id: string;
  person_id: string;
  channel: string;
  channel_address: string;
  status: string;
  sent_at?: string;
  created_at: string;
}

export interface ScheduledMeetingRecord {
  id: string;
  proposal_id: string;
  scheduled_at: string;
  duration_min: number;
  location?: string;
  agenda?: string;
  channel: string;
  invite_status: string;
  invite_sent_at?: string;
  notes?: string;
  invitees: ScheduledMeetingInviteeRecord[];
  created_at: string;
  updated_at: string;
}

export interface InviteDispatchResult {
  person_id: string;
  channel: string;
  status: string;
  message: string;
}

export const meetingsApi = {
  publish: async (
    sessionId: string,
    data: {
      project_id: string;
      company_id?: string;
      proposal_id?: string;
      attendee_person_ids?: string[];
      source_title?: string;
    }
  ): Promise<{ session_id: string; knowledge_source_id: string; message: string }> => {
    const response = await makeRequest(`/api/topsi/meeting/${sessionId}/publish`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse(response);
  },
};

// ── Discord Voice ─────────────────────────────────────────────────────────────

export interface DiscordSessionSummary {
  meeting_session_id: string;
  guild_id: string;
  channel_id: string;
  channel_name: string;
  project_id: string;
  agent: string;
  started_at: string;
  elapsed_seconds: number;
  segment_count: number;
  participant_count: number;
}

export interface DiscordSegment {
  id: string;
  segment_index: number;
  speaker_label?: string;
  text: string;
  confidence?: number;
  start_time_ms: number;
  end_time_ms: number;
  is_topsi_addressed: boolean;
  metadata?: string;
  created_at: string;
}

export interface DiscordTranscript {
  meeting_session_id: string;
  segment_count: number;
  segments: DiscordSegment[];
}

export const discordApi = {
  activeSessions: async (): Promise<DiscordSessionSummary[]> => {
    const response = await makeRequest('/api/discord/sessions');
    return handleApiResponse<DiscordSessionSummary[]>(response);
  },

  archivedSessions: async (params?: { limit?: number; offset?: number }): Promise<any[]> => {
    const qs = params ? '?' + new URLSearchParams(Object.fromEntries(Object.entries(params).filter(([, v]) => v != null).map(([k, v]) => [k, String(v)]))).toString() : '';
    const response = await makeRequest(`/api/discord/archive${qs}`);
    return handleApiResponse<any[]>(response);
  },

  getTranscript: async (sessionId: string): Promise<DiscordTranscript> => {
    const response = await makeRequest(`/api/discord/sessions/${sessionId}/transcript`);
    return handleApiResponse<DiscordTranscript>(response);
  },

  leave: async (guildId: string): Promise<{ success: boolean; meeting_session_id: string }> => {
    const response = await makeRequest('/api/discord/leave', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ guild_id: guildId }),
    });
    return handleApiResponse(response);
  },
};

export const intelligenceApi = {
  triggerResearch: async (personId: string, opts?: { project_id?: string; agent_preference?: string }): Promise<{ person_id: string; status: string; message: string }> => {
    const response = await makeRequest(`/api/persons/${personId}/research`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(opts ?? {}),
    });
    return handleApiResponse(response);
  },

  getStatus: async (personId: string): Promise<IntelligenceStatus> => {
    const response = await makeRequest(`/api/persons/${personId}/intelligence-status`);
    return handleApiResponse<IntelligenceStatus>(response);
  },

  listResearchPasses: async (personId: string): Promise<ResearchPass[]> => {
    const response = await makeRequest(`/api/persons/${personId}/research-passes`);
    return handleApiResponse<ResearchPass[]>(response);
  },

  triggerNextPass: async (personId: string, opts?: { focus?: string; project_id?: string }): Promise<{ pass_id: string; pass_number: number; focus: string; status: string }> => {
    const response = await makeRequest(`/api/persons/${personId}/research-passes/next`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(opts ?? {}),
    });
    return handleApiResponse(response);
  },

  listPersonReports: async (personId: string): Promise<BusinessReportRecord[]> => {
    const response = await makeRequest(`/api/persons/${personId}/reports`);
    return handleApiResponse<BusinessReportRecord[]>(response);
  },
};

export const reportsApi = {
  list: async (): Promise<BusinessReportRecord[]> => {
    const response = await makeRequest('/api/business-reports');
    return handleApiResponse<BusinessReportRecord[]>(response);
  },

  get: async (id: string): Promise<BusinessReportRecord> => {
    const response = await makeRequest(`/api/business-reports/${id}`);
    return handleApiResponse<BusinessReportRecord>(response);
  },

  patch: async (id: string, data: Partial<BusinessReportRecord>): Promise<BusinessReportRecord> => {
    const response = await makeRequest(`/api/business-reports/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<BusinessReportRecord>(response);
  },

  generate: async (personId: string, reportType?: string): Promise<{ status: string; person_id: string }> => {
    const response = await makeRequest('/api/business-reports/generate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ person_id: personId, report_type: reportType }),
    });
    return handleApiResponse(response);
  },

  approve: async (id: string): Promise<{ report: BusinessReportRecord; deal: unknown; proposal: unknown }> => {
    const response = await makeRequest(`/api/business-reports/${id}/approve`, {
      method: 'POST',
    });
    return handleApiResponse(response);
  },

  requestRevision: async (id: string, notes: string): Promise<BusinessReportRecord> => {
    const response = await makeRequest(`/api/business-reports/${id}/request-revision`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ notes }),
    });
    return handleApiResponse<BusinessReportRecord>(response);
  },
};

export interface ResearchPass {
  id: string;
  person_id: string;
  pass_number: number;
  research_focus: string;
  status: string;
  summary?: string;
  key_findings: string; // JSON
  confidence_delta: number;
  agent_used?: string;
  created_at: string;
  completed_at?: string;
  error?: string;
}

export interface BusinessReportRecord {
  id: string;
  person_id?: string;
  company_id?: string;
  report_type: string;
  title: string;
  status: string;
  executive_summary?: string;
  company_overview?: string;
  pain_points: string; // JSON [{point, severity}]
  opportunities: string; // JSON [{title, description, priority, estimated_value}]
  recommended_services: string; // JSON [{name, rationale, timeline}]
  next_steps: string; // JSON [{action, owner, deadline}]
  // Enhanced analytics sections
  individual_profiles: string; // JSON [{name, role, company, linkedin, summary, key_insights}]
  market_analysis?: string;
  competitor_analysis: string; // JSON [{name, website, strengths, weaknesses, threat_level}]
  target_clients?: string;
  brand_positioning?: string;
  digital_presence?: string;
  sources: string; // JSON [{title, url, excerpt}]
  full_report_md?: string;
  // CRM deal linkage + human review checkpoint
  crm_deal_id?: string;
  review_status: string; // 'pending_review' | 'approved' | 'rejected'
  reviewed_by?: string;
  reviewed_at?: string;
  review_notes?: string;
  created_at: string;
  updated_at: string;
}

// ============================================================================
// Data Sources API
// ============================================================================

export interface DataSourceRecord {
  id: string;
  organization_id?: string;
  project_id?: string;
  created_by?: string;
  title: string;
  description?: string;
  data_type: string;
  /** "file", "text", or "integration" */
  source_type: string;
  file_type?: string;
  /** Raw text content (for source_type = "text") */
  content?: string;
  file_name?: string;
  file_path?: string;
  file_size_bytes?: number;
  file_hash?: string;
  metadata: string; // JSON string
  status: string;
  processing_error?: string;
  /** Slash-delimited folder path, e.g. "Meetings/Google Meet" */
  folder: string;
  created_at: string;
  updated_at: string;
  archived_at?: string;
}

export interface CreateDataSourceRequest {
  organization_id?: string;
  project_id?: string;
  title: string;
  description?: string;
  data_type: string;
  /** "file", "text", or "integration" */
  source_type?: string;
  file_type?: string;
  /** Raw text content (for source_type = "text") */
  content?: string;
  metadata?: Record<string, unknown>;
  folder?: string;
}

export interface UpdateDataSourceRequest {
  title?: string;
  description?: string;
  data_type?: string;
  source_type?: string;
  content?: string;
  metadata?: string;
  status?: string;
  processing_error?: string;
  folder?: string;
}

export const SOURCE_TYPE_OPTIONS = [
  { value: 'text', label: 'Text (copy/paste)' },
  { value: 'file', label: 'File Upload' },
  { value: 'integration', label: 'Integration' },
] as const;

export const DATA_TYPE_OPTIONS = [
  { value: 'conversation', label: 'Conversation' },
  { value: 'document', label: 'Document' },
  { value: 'transcript', label: 'Transcript' },
  { value: 'report', label: 'Report' },
  { value: 'dataset', label: 'Dataset' },
  { value: 'media', label: 'Media' },
  { value: 'other', label: 'Other' },
] as const;

export const dataSourcesApi = {
  listByOrganization: async (orgId: string): Promise<DataSourceRecord[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/data-sources`);
    return handleApiResponse<DataSourceRecord[]>(response);
  },

  listByProject: async (projectId: string): Promise<DataSourceRecord[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/data-sources`);
    return handleApiResponse<DataSourceRecord[]>(response);
  },

  get: async (id: string): Promise<DataSourceRecord> => {
    const response = await makeRequest(`/api/data-sources/${id}`);
    return handleApiResponse<DataSourceRecord>(response);
  },

  create: async (data: CreateDataSourceRequest): Promise<DataSourceRecord> => {
    const response = await makeRequest('/api/data-sources', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<DataSourceRecord>(response);
  },

  upload: async (formData: FormData): Promise<DataSourceRecord> => {
    const response = await fetch(resolveApiUrl('/api/data-sources/upload'), {
      method: 'POST',
      body: formData,
      credentials: 'include',
    });
    if (!response.ok) {
      const errorText = await response.text();
      throw new ApiError(`Failed to upload data source: ${errorText}`, response.status, response);
    }
    const result = await response.json();
    return result.data as DataSourceRecord;
  },

  update: async (id: string, data: UpdateDataSourceRequest): Promise<DataSourceRecord> => {
    const response = await makeRequest(`/api/data-sources/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<DataSourceRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/data-sources/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  getMetadataTemplate: async (dataType: string): Promise<Record<string, unknown>> => {
    const response = await makeRequest(`/api/data-sources/metadata-template/${dataType}`);
    return handleApiResponse<Record<string, unknown>>(response);
  },

  getWorkflows: async (dataSourceId: string) => {
    const response = await makeRequest(`/api/data-sources/${dataSourceId}/workflows`);
    return handleApiResponse<any>(response);
  },

  runWorkflow: async (dataSourceId: string, workflowId: string, model?: string, force?: boolean) => {
    const response = await makeRequest(`/api/data-sources/${dataSourceId}/workflows/${workflowId}/run`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ model, force }),
    });
    return handleApiResponse<any>(response);
  },

  getArtifacts: async (dataSourceId: string) => {
    const response = await makeRequest(`/api/data-sources/${dataSourceId}/artifacts`);
    return handleApiResponse<any[]>(response);
  },

  download: async (id: string): Promise<Blob> => {
    const response = await fetch(resolveApiUrl(`/api/data-sources/${id}/download`), {
      credentials: 'include',
    });
    if (!response.ok) {
      throw new ApiError(`Download failed: ${response.statusText}`, response.status, response);
    }
    return response.blob();
  },

  downloadUrl: (id: string): string => resolveApiUrl(`/api/data-sources/${id}/download`),
};

// ── Workflow types ──────────────────────────────────────────────────────────

export interface WorkflowNodePosition {
  x: number;
  y: number;
}

export interface WorkflowNode {
  id: string;
  name: string;
  type: string;
  parameters: Record<string, any>;
  position: WorkflowNodePosition;
}

export interface WorkflowConnection {
  source: string;
  target: string;
  source_output?: number;
  target_input?: number;
}

export interface WorkflowDefinition {
  id: string;
  name: string;
  description?: string;
  nodes: WorkflowNode[];
  connections: WorkflowConnection[];
  is_system: boolean;
  owner_type: string;  // "system", "organization", "user"
  owner_id?: string;
  default_model?: string;
}

export interface CreateWorkflowRequest {
  id: string;
  name: string;
  description?: string;
  nodes: WorkflowNode[];
  connections: WorkflowConnection[];
  owner_type?: string;
  owner_id?: string;
  default_model?: string;
}

export interface PreviewNodeResult {
  node_id: string;
  node_name: string;
  node_type: string;
  output: string;
  usage?: {
    model_used?: string;
    provider?: string;
    input_tokens?: number;
    output_tokens?: number;
    estimated_cost_micros?: number;
  };
}

export interface UpdateWorkflowRequest {
  name?: string;
  description?: string;
  nodes?: WorkflowNode[];
  connections?: WorkflowConnection[];
  default_model?: string;
}

export interface AvailableModel {
  id: string;
  label: string;
  is_default: boolean;
  provider: string;
  cost_per_million_input: number;
  cost_per_million_output: number;
}

export const workflowsApi = {
  listDefinitions: async (): Promise<WorkflowDefinition[]> => {
    const response = await makeRequest('/api/workflows/definitions');
    return handleApiResponse<WorkflowDefinition[]>(response);
  },

  createDefinition: async (data: CreateWorkflowRequest): Promise<WorkflowDefinition> => {
    const response = await makeRequest('/api/workflows/definitions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowDefinition>(response);
  },

  updateDefinition: async (id: string, data: UpdateWorkflowRequest): Promise<WorkflowDefinition> => {
    const response = await makeRequest(`/api/workflows/definitions/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowDefinition>(response);
  },

  deleteDefinition: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/workflows/definitions/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  previewWorkflow: async (data: { nodes: WorkflowNode[]; connections: WorkflowConnection[]; content?: string }): Promise<PreviewNodeResult[]> => {
    const response = await makeRequest('/api/workflows/preview', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PreviewNodeResult[]>(response);
  },

  listRecentArtifacts: async (organizationId?: string) => {
    const params = organizationId ? `?organization_id=${organizationId}` : '';
    const response = await makeRequest(`/api/artifacts/recent${params}`);
    return handleApiResponse<ExecutionArtifact[]>(response);
  },

  listAvailableModels: async (): Promise<AvailableModel[]> => {
    const response = await makeRequest('/api/workflows/models');
    return handleApiResponse<AvailableModel[]>(response);
  },

  listRecentRuns: async (params?: { workflow_id?: string; organization_id?: string; limit?: number }): Promise<WorkflowRun[]> => {
    const searchParams = new URLSearchParams();
    if (params?.workflow_id) searchParams.set('workflow_id', params.workflow_id);
    if (params?.organization_id) searchParams.set('organization_id', params.organization_id);
    if (params?.limit) searchParams.set('limit', String(params.limit));
    const qs = searchParams.toString();
    const response = await makeRequest(`/api/workflows/runs/recent${qs ? `?${qs}` : ''}`);
    return handleApiResponse<WorkflowRun[]>(response);
  },

  getRunById: async (id: string): Promise<WorkflowRun> => {
    const response = await makeRequest(`/api/workflows/runs/${id}`);
    return handleApiResponse<WorkflowRun>(response);
  },

  getRunStats: async (id: string): Promise<WorkflowRunStats> => {
    const response = await makeRequest(`/api/workflows/runs/${id}/stats`);
    return handleApiResponse<WorkflowRunStats>(response);
  },
};

// ── Workflow Run types ──────────────────────────────────────────────────────

export interface WorkflowRun {
  id: string;
  workflow_id: string;
  workflow_name: string;
  data_source_id?: string;
  organization_id?: string;
  project_id?: string;
  model_used?: string;
  status: 'running' | 'completed' | 'failed';
  total_input_tokens?: number;
  total_output_tokens?: number;
  total_estimated_cost_micros?: number;
  total_records_staged?: number;
  total_records_approved?: number;
  total_records_rejected?: number;
  total_records_committed?: number;
  total_duplicates_found?: number;
  total_validation_errors?: number;
  node_count?: number;
  llm_node_count?: number;
  duration_ms?: number;
  content_hash?: string;
  started_at: string;
  completed_at?: string;
  created_at: string;
}

export interface WorkflowRunStats {
  run: WorkflowRun;
  live_counts: {
    total: number;
    approved: number;
    rejected: number;
    committed: number;
    duplicates: number;
    pending: number;
  };
  rates: {
    approval_rate: number;
    duplicate_rate: number;
  };
  cost_dollars: number;
}

// ── Workflow Trigger types ──────────────────────────────────────────────────

export interface WorkflowTrigger {
  id: string;
  workflow_id: string;
  name: string;
  enabled: boolean;
  trigger_type: 'data_source_created' | 'data_source_updated' | 'schedule';
  filter_data_source_types: string | null;  // JSON array
  filter_organization_id: string | null;
  filter_project_id: string | null;
  filter_tags: string | null;  // JSON array
  model_override: string | null;
  auto_approve: boolean;
  last_triggered_at: string | null;
  trigger_count: number;
  created_at: string;
  updated_at: string;
}

export interface CreateWorkflowTrigger {
  workflow_id: string;
  name: string;
  trigger_type?: string;
  filter_data_source_types?: string[];
  filter_organization_id?: string;
  filter_project_id?: string;
  filter_tags?: string[];
  model_override?: string;
  auto_approve?: boolean;
}

export interface UpdateWorkflowTrigger {
  name?: string;
  enabled?: boolean;
  trigger_type?: string;
  filter_data_source_types?: string[];
  filter_organization_id?: string;
  filter_project_id?: string;
  filter_tags?: string[];
  model_override?: string;
  auto_approve?: boolean;
}

export const triggersApi = {
  list: async (workflowId?: string): Promise<WorkflowTrigger[]> => {
    const params = workflowId ? `?workflow_id=${encodeURIComponent(workflowId)}` : '';
    const response = await makeRequest(`/api/workflows/triggers${params}`);
    return handleApiResponse<WorkflowTrigger[]>(response);
  },

  get: async (id: string): Promise<WorkflowTrigger> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}`);
    return handleApiResponse<WorkflowTrigger>(response);
  },

  create: async (data: CreateWorkflowTrigger): Promise<WorkflowTrigger> => {
    const response = await makeRequest('/api/workflows/triggers', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowTrigger>(response);
  },

  update: async (id: string, data: UpdateWorkflowTrigger): Promise<WorkflowTrigger> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowTrigger>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  toggle: async (id: string): Promise<WorkflowTrigger> => {
    const response = await makeRequest(`/api/workflows/triggers/${id}/toggle`, {
      method: 'POST',
    });
    return handleApiResponse<WorkflowTrigger>(response);
  },

  check: async (dataSourceId: string): Promise<WorkflowTrigger[]> => {
    const response = await makeRequest('/api/workflows/triggers/check', {
      method: 'POST',
      body: JSON.stringify({ data_source_id: dataSourceId }),
    });
    return handleApiResponse<WorkflowTrigger[]>(response);
  },
};

// ── Schema types and API ──────────────────────────────────────────────────────

export interface FieldDef {
  type: string;
  required: boolean;
  description: string;
  enum_values?: string[];
  format?: string;
}

export interface TargetSchema {
  target_type: string;
  description: string;
  fields: Record<string, FieldDef>;
}

export const schemasApi = {
  list: async (): Promise<{ target_type: string; description: string; icon: string }[]> => {
    const response = await makeRequest('/api/schemas');
    return response.json();
  },

  get: async (targetType: string): Promise<TargetSchema> => {
    const response = await makeRequest(`/api/schemas/${targetType}`);
    return response.json();
  },
};

export interface WorkflowStagingRecord {
  id: string;
  workflow_run_id: string;
  workflow_id: string;
  node_id: string;
  data_source_id: string | null;
  organization_id: string | null;
  project_id: string | null;
  target_type: 'crm_contact' | 'company' | 'crm_deal' | 'task';
  record_data: string;  // JSON string
  status: 'pending_review' | 'approved' | 'rejected' | 'committed' | 'error';
  duplicate_of_id: string | null;
  duplicate_of_type: string | null;
  confidence: number | null;
  error_message: string | null;
  reviewed_by: string | null;
  reviewed_at: string | null;
  committed_at: string | null;
  validation_errors: string | null;  // JSON array string of validation error messages, or null
  created_at: string;
  updated_at: string;
}

export interface CommitResult {
  id: string;
  target_type: string;
  created_id: string | null;
  error: string | null;
}

export interface BatchCommitResult {
  committed: number;
  errors: number;
  results: CommitResult[];
}

export const stagingApi = {
  listByRun: async (workflowRunId: string): Promise<WorkflowStagingRecord[]> => {
    const response = await makeRequest(`/api/workflow-staging?workflow_run_id=${workflowRunId}`);
    return handleApiResponse<WorkflowStagingRecord[]>(response);
  },

  listPending: async (organizationId: string): Promise<WorkflowStagingRecord[]> => {
    const response = await makeRequest(`/api/workflow-staging/pending?organization_id=${organizationId}`);
    return handleApiResponse<WorkflowStagingRecord[]>(response);
  },

  get: async (id: string): Promise<WorkflowStagingRecord> => {
    const response = await makeRequest(`/api/workflow-staging/${id}`);
    return handleApiResponse<WorkflowStagingRecord>(response);
  },

  update: async (id: string, data: { status?: string; record_data?: any }): Promise<WorkflowStagingRecord> => {
    const response = await makeRequest(`/api/workflow-staging/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<WorkflowStagingRecord>(response);
  },

  batchAction: async (ids: string[], action: 'approve' | 'reject'): Promise<void> => {
    const response = await makeRequest('/api/workflow-staging/batch', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ids, action }),
    });
    await handleApiResponse<void>(response);
  },

  commit: async (id: string): Promise<CommitResult> => {
    const response = await makeRequest(`/api/workflow-staging/${id}/commit`, {
      method: 'POST',
    });
    return handleApiResponse<CommitResult>(response);
  },

  batchCommit: async (workflowRunId: string): Promise<BatchCommitResult> => {
    const response = await makeRequest('/api/workflow-staging/batch-commit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow_run_id: workflowRunId }),
    });
    return handleApiResponse<BatchCommitResult>(response);
  },

  autoApprove: async (workflowRunId: string): Promise<{ affected: number }> => {
    const response = await makeRequest('/api/workflow-staging/auto-approve', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow_run_id: workflowRunId }),
    });
    return handleApiResponse<{ affected: number }>(response);
  },

  rejectDuplicates: async (workflowRunId: string): Promise<{ affected: number }> => {
    const response = await makeRequest('/api/workflow-staging/reject-duplicates', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ workflow_run_id: workflowRunId }),
    });
    return handleApiResponse<{ affected: number }>(response);
  },
};

// ── PCG Router / Provider Keys API ──────────────────────────────────────────

export interface ProviderKeyStatus {
  provider: string;
  has_key: boolean;
  model_count: number;
  enabled_count: number;
  env_var: string | null;
}

export interface PcgRouterModelInfo {
  id: string;
  name: string;
  model_id: string;
  provider: string;
  provider_base_url: string | null;
  api_key_env_var: string | null;
  priority: number;
  context_window: number | null;
  max_output_tokens: number | null;
  supports_tools: boolean;
  supports_vision: boolean;
  cost_per_million_input: number;
  cost_per_million_output: number;
  is_enabled: boolean;
  created_at: string;
  updated_at: string;
}

// PCG Router returns raw JSON (not wrapped in { success, data })
const handleRawJsonResponse = async <T>(response: Response): Promise<T> => {
  if (!response.ok) {
    const text = await response.text().catch(() => response.statusText);
    throw new ApiError(text || 'Request failed', response.status, response);
  }
  return response.json();
};

export const pcgRouterApi = {
  listModels: async (): Promise<PcgRouterModelInfo[]> => {
    const response = await makeRequest('/api/pcg-router/models');
    return handleRawJsonResponse<PcgRouterModelInfo[]>(response);
  },

  listProviderKeys: async (): Promise<ProviderKeyStatus[]> => {
    const response = await makeRequest('/api/pcg-router/provider-keys');
    return handleRawJsonResponse<ProviderKeyStatus[]>(response);
  },

  setProviderKey: async (provider: string, apiKey: string): Promise<{ provider: string; models_updated: number; has_key: boolean }> => {
    const response = await makeRequest('/api/pcg-router/provider-keys', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ provider, api_key: apiKey }),
    });
    return handleRawJsonResponse<{ provider: string; models_updated: number; has_key: boolean }>(response);
  },

  deleteProviderKey: async (provider: string): Promise<void> => {
    const response = await makeRequest(`/api/pcg-router/provider-keys/${provider}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      throw new ApiError('Failed to delete provider key', response.status, response);
    }
  },

  patchModel: async (id: string, data: Record<string, unknown>): Promise<PcgRouterModelInfo> => {
    const response = await makeRequest(`/api/pcg-router/models/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleRawJsonResponse<PcgRouterModelInfo>(response);
  },
};

// Axios-compatible client for pages that use apiClient.get/post/patch/delete
export const apiClient = {
  get: async <T = unknown>(url: string) => { const r = await makeRequest(`/api${url}`); const data = await r.json() as T; return { data }; },
  post: async <T = unknown>(url: string, body?: unknown) => { const r = await makeRequest(`/api${url}`, { method: 'POST', body: body ? JSON.stringify(body) : undefined }); const data = await r.json() as T; return { data }; },
  patch: async <T = unknown>(url: string, body?: unknown) => { const r = await makeRequest(`/api${url}`, { method: 'PATCH', body: body ? JSON.stringify(body) : undefined }); const data = await r.json() as T; return { data }; },
  delete: async <T = unknown>(url: string) => { const r = await makeRequest(`/api${url}`, { method: 'DELETE' }); const data = await r.json() as T; return { data }; },
};

export const authApi = {
  getInviteInfo: async (token: string): Promise<{ org_name: string; pending_owner_email?: string }> => {
    const response = await makeRequest(`/api/auth/invite-info?token=${encodeURIComponent(token)}`);
    return handleApiResponse(response);
  },
  register: async (data: { username: string; password: string; full_name: string; email?: string; invite_token: string }): Promise<void> => {
    const response = await makeRequest('/api/auth/register', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse(response);
  },
};

export interface MediaAsset {
  id: string;
  project_id: string;
  batch_id?: string;
  filename: string;
  file_path: string;
  file_size_bytes: number;
  mime_type: string;
  duration_seconds?: number;
  width?: number;
  height?: number;
  ai_description?: string;
  shot_type?: string;
  energy_level: number;
  motion_intensity: number;
  dominant_colors: string;
  scene_tags: string;
  ai_confidence: number;
  analysis_status: string;
  created_at: string;
  updated_at: string;
}

export const mediaApi = {
  list: async (projectId: string) => {
    const r = await makeRequest(`/api/projects/${projectId}/media`);
    return handleApiResponse<MediaAsset[]>(r);
  },
  search: async (projectId: string, q: string) => {
    const r = await makeRequest(`/api/projects/${projectId}/media/search?q=${encodeURIComponent(q)}`);
    return handleApiResponse<MediaAsset[]>(r);
  },
  upload: async (projectId: string, fd: FormData) => {
    const r = await makeRequest(`/api/projects/${projectId}/media`, {
      method: 'POST',
      headers: {},
      body: fd,
    } as RequestInit);
    return handleApiResponse<MediaAsset>(r);
  },
  delete: async (id: string) => {
    const r = await makeRequest(`/api/media/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(r);
  },
};

export interface ReviewComment { id: string; author_name?: string; content: string; timecode_seconds?: number; is_resolved: boolean; resolved_at?: string; created_at: string; }
export interface ReviewDeliverable { id: string; title: string; status: string; description?: string; working_file_url?: string; final_link?: string; }
export interface ReviewToken { id: string; view_count: number; expires_at?: string; }
export interface ReviewSourceFile { name: string; url: string; size_bytes: number; }
export interface ReviewData { token: ReviewToken; deliverable: ReviewDeliverable; comments: ReviewComment[]; source_files: ReviewSourceFile[]; }

export const reviewApi = {
  getData: async (token: string): Promise<ReviewData> => {
    const r = await makeRequest(`/api/review/${token}/data`);
    return handleApiResponse<ReviewData>(r);
  },
  addComment: async (token: string, data: { author_name?: string; author_email?: string; content: string; timecode_seconds?: number }): Promise<ReviewComment> => {
    const r = await makeRequest(`/api/review/${token}/comments`, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(data) });
    return handleApiResponse<ReviewComment>(r);
  },
  resolve: async (token: string, commentId: string): Promise<ReviewComment> => {
    const r = await makeRequest(`/api/review/${token}/comments/${commentId}/resolve`, { method: 'PATCH', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({}) });
    return handleApiResponse<ReviewComment>(r);
  },
};

// Token Usage API Types
export interface TokenUsageSummary {
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_cents: number | null;
  request_count: number;
}

export interface DailyTokenUsage {
  usage_date: string;
  project_id: string;
  model: string;
  provider: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_cents: number | null;
  request_count: number;
}

export interface TokenUsageByProject {
  project_id: string;
  project_name: string | null;
  total_tokens: number;
  request_count: number;
}

export interface TokenUsageByAgent {
  agent_id: string;
  agent_name: string | null;
  total_tokens: number;
  request_count: number;
}

export interface TokenUsageByProvider {
  provider: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_cents: number | null;
  request_count: number;
}

export interface TokenUsageByModel {
  model: string;
  provider: string;
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_cents: number | null;
  request_count: number;
}

export const tokenUsageApi = {
  getToday: async (): Promise<TokenUsageSummary> => {
    const r = await makeRequest('/api/token-usage/today');
    return handleApiResponse<TokenUsageSummary>(r);
  },
  getDaily: async (days: number = 7): Promise<DailyTokenUsage[]> => {
    const r = await makeRequest(`/api/token-usage/daily?days=${days}`);
    return handleApiResponse<DailyTokenUsage[]>(r);
  },
  getByProject: async (days: number = 7): Promise<TokenUsageByProject[]> => {
    const r = await makeRequest(`/api/token-usage/by-project?days=${days}`);
    return handleApiResponse<TokenUsageByProject[]>(r);
  },
  getByAgent: async (days: number = 7): Promise<TokenUsageByAgent[]> => {
    const r = await makeRequest(`/api/token-usage/by-agent?days=${days}`);
    return handleApiResponse<TokenUsageByAgent[]>(r);
  },
  getByProvider: async (days: number = 7): Promise<TokenUsageByProvider[]> => {
    const r = await makeRequest(`/api/token-usage/by-provider?days=${days}`);
    return handleApiResponse<TokenUsageByProvider[]>(r);
  },
  getByModel: async (days: number = 7): Promise<TokenUsageByModel[]> => {
    const r = await makeRequest(`/api/token-usage/by-model?days=${days}`);
    return handleApiResponse<TokenUsageByModel[]>(r);
  },
};

// ========================================
// System Settings API
// ========================================

export interface SystemSetting {
  key: string;
  value: string;
  updated_by: string | null;
  updated_at: string | null;
}

export const systemSettingsApi = {
  getAll: async (): Promise<SystemSetting[]> => {
    const r = await makeRequest('/api/system-settings');
    return handleApiResponse<SystemSetting[]>(r);
  },
  update: async (key: string, value: string): Promise<string> => {
    const r = await makeRequest(`/api/system-settings/${key}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ value }),
    });
    return handleApiResponse<string>(r);
  },
};
