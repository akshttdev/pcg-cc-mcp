import { makeRequest, handleApiResponse } from './client';
import type {
  BrowserSession,
  BrowserSessionDetails,
  BrowserScreenshot,
  BrowserAction,
  BrowserAllowlist,
  BowserSummary,
  BrowserType,
  PatternType,
} from '@/hooks/useBowser';

export const bowserApi = {
  // Sessions
  listActiveSessions: (): Promise<BrowserSession[]> =>
    makeRequest('/api/bowser/sessions').then(handleApiResponse<BrowserSession[]>),

  getSession: (sessionId: string): Promise<BrowserSession> =>
    makeRequest(`/api/bowser/sessions/${sessionId}`).then(handleApiResponse<BrowserSession>),

  getSessionDetails: (sessionId: string): Promise<BrowserSessionDetails> =>
    makeRequest(`/api/bowser/sessions/${sessionId}/details`).then(handleApiResponse<BrowserSessionDetails>),

  startSession: (data: {
    execution_process_id: string;
    browser_type?: BrowserType;
    viewport_width?: number;
    viewport_height?: number;
    headless?: boolean;
  }): Promise<BrowserSession> =>
    makeRequest('/api/bowser/sessions', {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<BrowserSession>),

  closeSession: (sessionId: string): Promise<BrowserSession> =>
    makeRequest(`/api/bowser/sessions/${sessionId}/close`, {
      method: 'POST',
    }).then(handleApiResponse<BrowserSession>),

  // Screenshots
  listScreenshots: (sessionId: string): Promise<BrowserScreenshot[]> =>
    makeRequest(`/api/bowser/sessions/${sessionId}/screenshots`).then(handleApiResponse<BrowserScreenshot[]>),

  listScreenshotsWithDiffs: (sessionId: string): Promise<BrowserScreenshot[]> =>
    makeRequest(`/api/bowser/sessions/${sessionId}/screenshots/diffs`).then(handleApiResponse<BrowserScreenshot[]>),

  // Actions
  listActions: (sessionId: string): Promise<BrowserAction[]> =>
    makeRequest(`/api/bowser/sessions/${sessionId}/actions`).then(handleApiResponse<BrowserAction[]>),

  navigate: (sessionId: string, projectId: string, url: string): Promise<BrowserAction> =>
    makeRequest(`/api/bowser/sessions/${sessionId}/navigate`, {
      method: 'POST',
      body: JSON.stringify({ url, project_id: projectId }),
    }).then(handleApiResponse<BrowserAction>),

  // Allowlist
  listAllowlist: (projectId: string): Promise<BrowserAllowlist[]> =>
    makeRequest(`/api/bowser/projects/${projectId}/allowlist`).then(handleApiResponse<BrowserAllowlist[]>),

  checkUrl: (projectId: string, url: string): Promise<{ allowed: boolean; url: string }> =>
    makeRequest(`/api/bowser/projects/${projectId}/check-url`, {
      method: 'POST',
      body: JSON.stringify({ url }),
    }).then(handleApiResponse<{ allowed: boolean; url: string }>),

  addToAllowlist: (data: {
    project_id?: string;
    pattern: string;
    pattern_type?: PatternType;
    description?: string;
    is_global?: boolean;
  }): Promise<BrowserAllowlist> =>
    makeRequest('/api/bowser/allowlist', {
      method: 'POST',
      body: JSON.stringify(data),
    }).then(handleApiResponse<BrowserAllowlist>),

  removeFromAllowlist: (entryId: string): Promise<void> =>
    makeRequest(`/api/bowser/allowlist/${entryId}`, {
      method: 'DELETE',
    }).then(handleApiResponse<void>),

  // Summary
  getSummary: (): Promise<BowserSummary> =>
    makeRequest('/api/bowser/summary').then(handleApiResponse<BowserSummary>),
};
