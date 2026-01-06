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

// Re-export types for convenience
export type { RepositoryInfo } from 'shared/types';
export type {
  FollowUpDraftResponse,
  UpdateFollowUpDraftRequest,
} from 'shared/types';
export type { ProjectBoard, ProjectBoardType } from 'shared/types';
export type { BrandProfile, UpsertBrandProfile } from 'shared/types';

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

const makeRequest = async (url: string, options: RequestInit = {}) => {
  const headers = {
    'Content-Type': 'application/json',
    ...(options.headers || {}),
  };

  const isFileProtocol =
    typeof window !== 'undefined' && window.location.protocol === 'file:';

  if (isFileProtocol) {
    throw new Error(
      'The PCG CC dashboard assets were opened directly from disk. Please start the PCG CC server (`node npx-cli/bin/cli.js`) and access it via the URL printed in the terminal.'
    );
  }

  try {
    return await fetch(url, {
      ...options,
      headers,
    });
  } catch (error) {
    if (isFileProtocol) {
      throw new Error(
        'Unable to reach the PCG CC server because the dashboard is running from the filesystem. Launch the server via `node npx-cli/bin/cli.js` and reopen the app from the provided http:// address.'
      );
    }

    if (typeof window !== 'undefined' && error instanceof TypeError) {
      throw new Error(
        `Failed to reach the PCG CC server at ${window.location.origin}. Make sure the server process is running and reachable.`
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
};

// Task Management APIs
export const tasksApi = {
  getAll: async (projectId: string): Promise<TaskWithAttemptStatus[]> => {
    const response = await makeRequest(`/api/tasks?project_id=${projectId}`);
    return handleApiResponse<TaskWithAttemptStatus[]>(response);
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

// Agent Registry APIs
export const agentsApi = {
  // List all agents
  list: async (): Promise<AgentWithParsedFields[]> => {
    const response = await makeRequest('/api/agents');
    return handleApiResponse<AgentWithParsedFields[]>(response);
  },

  // List active agents only
  listActive: async (): Promise<AgentWithParsedFields[]> => {
    const response = await makeRequest('/api/agents/active');
    return handleApiResponse<AgentWithParsedFields[]>(response);
  },

  // Get agent by ID
  getById: async (agentId: string): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}`);
    return handleApiResponse<AgentWithParsedFields>(response);
  },

  // Get agent by name
  getByName: async (name: string): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/by-name/${encodeURIComponent(name)}`);
    return handleApiResponse<AgentWithParsedFields>(response);
  },

  // Create a new agent
  create: async (agent: CreateAgent): Promise<AgentWithParsedFields> => {
    const response = await makeRequest('/api/agents', {
      method: 'POST',
      body: JSON.stringify(agent),
    });
    return handleApiResponse<AgentWithParsedFields>(response);
  },

  // Update an agent
  update: async (agentId: string, agent: UpdateAgent): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}`, {
      method: 'PUT',
      body: JSON.stringify(agent),
    });
    return handleApiResponse<AgentWithParsedFields>(response);
  },

  // Delete an agent
  delete: async (agentId: string): Promise<void> => {
    const response = await makeRequest(`/api/agents/${agentId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Seed core agents (Nora, Maci, Editron)
  seedCoreAgents: async (): Promise<AgentWithParsedFields[]> => {
    const response = await makeRequest('/api/agents/seed', {
      method: 'POST',
    });
    return handleApiResponse<AgentWithParsedFields[]>(response);
  },

  // Update agent status
  updateStatus: async (agentId: string, status: AgentStatus): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}/status`, {
      method: 'PUT',
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<AgentWithParsedFields>(response);
  },

  // Assign wallet to agent
  assignWallet: async (agentId: string, walletAddress: string): Promise<AgentWithParsedFields> => {
    const response = await makeRequest(`/api/agents/${agentId}/wallet`, {
      method: 'PUT',
      body: JSON.stringify({ wallet_address: walletAddress }),
    });
    return handleApiResponse<AgentWithParsedFields>(response);
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

    const response = await fetch('/api/images/upload', {
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
    return new EventSource(`/api/agent-flows/${flowId}/events/stream`);
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
  listAccounts: async (projectId?: string, provider?: string): Promise<EmailAccountRecord[]> => {
    const searchParams = new URLSearchParams();
    if (projectId) searchParams.set('project_id', projectId);
    if (provider) searchParams.set('provider', provider);
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
    projectId: string,
    provider: string,
    redirectUri: string
  ): Promise<OAuthUrlResponse> => {
    const response = await makeRequest('/api/email/oauth/initiate', {
      method: 'POST',
      body: JSON.stringify({
        project_id: projectId,
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
  project_id: string;
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
  project_id: string;
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

export const crmApi = {
  listContacts: async (
    projectId: string,
    options?: { lifecycleStage?: string; limit?: number }
  ): Promise<CrmContactRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('project_id', projectId);
    if (options?.lifecycleStage) searchParams.set('lifecycle_stage', options.lifecycleStage);
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    const response = await makeRequest(`/api/crm/contacts?${searchParams.toString()}`);
    return handleApiResponse<CrmContactRecord[]>(response);
  },

  searchContacts: async (
    projectId: string,
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
    searchParams.set('project_id', projectId);
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

  getContactStats: async (projectId: string): Promise<CrmContactStats> => {
    const response = await makeRequest(`/api/crm/contacts/stats/${projectId}`);
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
