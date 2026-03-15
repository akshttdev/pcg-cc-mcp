import type {
  Task,
  CreateTask,
  CreateAndStartTaskRequest,
  UpdateTask,
  TaskWithAttemptStatus,
  TaskAttempt,
  TaskRelationships,
  CreateTaskAttemptBody,
  CreateFollowUpAttempt,
  EditorType,
  BranchStatus,
  RebaseTaskAttemptRequest,
  GitOperationError,
  CreateGitHubPrRequest,
  GitHubServiceError,
  CommitInfo,
  FollowUpDraftResponse,
  UpdateFollowUpDraftRequest,
} from 'shared/types';
import { makeRequest, handleApiResponse, handleApiResponseAsResult, type TaskWithArchive, type Result } from './client';
import type { AssignedTask } from './projects';

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
