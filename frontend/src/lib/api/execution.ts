import type {
  ExecutionProcess,
  ExecutionSummary,
} from 'shared/types';
import { makeRequest, handleApiResponse, ApiError } from './client';

// Execution Process Logs type
export interface ExecutionProcessLogs {
  execution_id: string;
  logs: string; // JSONL format
  byte_size: number;
  inserted_at: Date;
}

// Task Attempts API
export const taskAttemptsApi = {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  list: async (): Promise<Array<Record<string, any>>> => {
    const response = await makeRequest('/api/task-attempts');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return handleApiResponse<Array<Record<string, any>>>(response);
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

  // Update user role
  updateRole: async (userId: string, isAdmin: boolean): Promise<UserListItem> => {
    const response = await makeRequest(`/api/users/${userId}/role`, {
      method: 'PATCH',
      body: JSON.stringify({ is_admin: isAdmin }),
    });
    return handleApiResponse<UserListItem>(response);
  },

  // Suspend user
  suspend: async (userId: string): Promise<UserListItem> => {
    const response = await makeRequest(`/api/users/${userId}/suspend`, {
      method: 'PATCH',
    });
    return handleApiResponse<UserListItem>(response);
  },

  // Activate user
  activate: async (userId: string): Promise<UserListItem> => {
    const response = await makeRequest(`/api/users/${userId}/activate`, {
      method: 'PATCH',
    });
    return handleApiResponse<UserListItem>(response);
  },

  // Create user
  create: async (userData: {
    username: string;
    password: string;
    email?: string;
    full_name: string;
    is_admin: boolean;
  }): Promise<{ message: string; user_id: string; username: string }> => {
    const response = await makeRequest('/api/users/create', {
      method: 'POST',
      body: JSON.stringify(userData),
    });
    return handleApiResponse<{ message: string; user_id: string; username: string }>(response);
  },
};
