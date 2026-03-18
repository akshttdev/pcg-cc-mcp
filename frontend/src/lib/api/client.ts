// Import all necessary types from shared types

import {
  ApiResponse,
} from 'shared/types';

// Re-export types for convenience
export type { RepositoryInfo } from 'shared/types';
export type {
  FollowUpDraftResponse,
  UpdateFollowUpDraftRequest,
} from 'shared/types';
export type { ProjectBoard, ProjectBoardType } from 'shared/types';
export type { BrandProfile, UpsertBrandProfile } from 'shared/types';
export type { AgentChatRequest, AgentChatResponse, ConversationSummary } from 'shared/types';

// Extend TaskWithAttemptStatus with archived_at (frontend feature, not yet in DB/backend)
import type { TaskWithAttemptStatus } from 'shared/types';
export type TaskWithArchive = TaskWithAttemptStatus & { archived_at?: string | null };

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

export class ApiError<E = unknown> extends Error {
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

export const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;
const getOrchaPort = (): number =>
  (window as unknown as Record<string, number>).__ORCHA_BACKEND_PORT__ || 58297;
export const API_BASE = isTauri
  ? `http://localhost:${getOrchaPort()}`
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
    const port = getOrchaPort();
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
export const handleApiResponseAsResult = async <T, E>(
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

export const handleApiResponse = async <T, E = T>(response: Response): Promise<T> => {
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
