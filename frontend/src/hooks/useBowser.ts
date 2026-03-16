import { useQuery } from '@tanstack/react-query';
import { bowserApi } from '@/lib/api';
import { bowserKeys } from '@/lib/query-keys';
import { useMutationWithToast } from './useMutationWithToast';

// ========== Types ==========

export type BrowserType = 'chromium' | 'firefox' | 'webkit';
export type SessionStatus = 'starting' | 'active' | 'idle' | 'closed' | 'error';
export type ActionType = 'navigate' | 'click' | 'type' | 'scroll' | 'screenshot' | 'wait' | 'select' | 'hover' | 'press_key' | 'evaluate' | 'upload' | 'download';
export type ActionResult = 'success' | 'failed' | 'blocked' | 'timeout';
export type PatternType = 'glob' | 'regex' | 'exact';

export interface BrowserSession {
  id: string;
  execution_process_id: string;
  browser_type: BrowserType;
  viewport_width: number;
  viewport_height: number;
  headless: boolean;
  status: SessionStatus;
  current_url: string | null;
  error_message: string | null;
  started_at: string;
  closed_at: string | null;
}

export interface BrowserScreenshot {
  id: string;
  browser_session_id: string;
  url: string;
  page_title: string | null;
  screenshot_path: string;
  thumbnail_path: string | null;
  baseline_screenshot_id: string | null;
  diff_path: string | null;
  diff_percentage: number | null;
  viewport_width: number;
  viewport_height: number;
  full_page: boolean;
  metadata: string | null;
  created_at: string;
}

export interface BrowserAction {
  id: string;
  browser_session_id: string;
  action_type: ActionType;
  target_selector: string | null;
  action_data: string | null;
  result: ActionResult | null;
  error_message: string | null;
  duration_ms: number | null;
  screenshot_id: string | null;
  created_at: string;
}

export interface BrowserAllowlist {
  id: string;
  project_id: string | null;
  pattern: string;
  pattern_type: PatternType;
  description: string | null;
  is_global: boolean;
  created_by: string | null;
  created_at: string;
}

export interface BrowserSessionDetails {
  session: BrowserSession;
  screenshots: BrowserScreenshot[];
  actions: BrowserAction[];
  action_counts: [string, number][];
}

export interface BowserSummary {
  active_sessions: number;
  total_screenshots: number;
  total_actions: number;
  blocked_actions: number;
}

// ========== Query Hooks ==========

export function useBowserSummary() {
  return useQuery({
    queryKey: bowserKeys.summary(),
    queryFn: bowserApi.getSummary,
    refetchInterval: 5000,
  });
}

export function useActiveSessions() {
  return useQuery({
    queryKey: bowserKeys.sessionsActive(),
    queryFn: bowserApi.listActiveSessions,
    refetchInterval: 3000,
  });
}

export function useSession(sessionId: string | undefined) {
  return useQuery({
    queryKey: bowserKeys.session(sessionId!),
    queryFn: () => bowserApi.getSession(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 2000,
  });
}

export function useSessionDetails(sessionId: string | undefined) {
  return useQuery({
    queryKey: bowserKeys.sessionDetails(sessionId!),
    queryFn: () => bowserApi.getSessionDetails(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 2000,
  });
}

export function useScreenshots(sessionId: string | undefined) {
  return useQuery({
    queryKey: bowserKeys.screenshots(sessionId!),
    queryFn: () => bowserApi.listScreenshots(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 3000,
  });
}

export function useScreenshotsWithDiffs(sessionId: string | undefined) {
  return useQuery({
    queryKey: bowserKeys.screenshotsDiffs(sessionId!),
    queryFn: () => bowserApi.listScreenshotsWithDiffs(sessionId!),
    enabled: !!sessionId,
  });
}

export function useActions(sessionId: string | undefined) {
  return useQuery({
    queryKey: bowserKeys.actions(sessionId!),
    queryFn: () => bowserApi.listActions(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 2000,
  });
}

export function useAllowlist(projectId: string | undefined) {
  return useQuery({
    queryKey: bowserKeys.allowlist(projectId!),
    queryFn: () => bowserApi.listAllowlist(projectId!),
    enabled: !!projectId,
  });
}

// ========== Mutation Hooks ==========

export function useCheckUrl(projectId: string | undefined) {
  return useMutationWithToast({
    mutationFn: (url: string) => bowserApi.checkUrl(projectId!, url),
    errorMessage: 'Failed to check URL',
  });
}

export function useStartSession() {
  return useMutationWithToast({
    mutationFn: bowserApi.startSession,
    successMessage: 'Browser session started',
    errorMessage: 'Failed to start session',
    invalidateKeys: [bowserKeys.sessionsActive()],
  });
}

export function useCloseSession() {
  return useMutationWithToast({
    mutationFn: bowserApi.closeSession,
    successMessage: 'Browser session closed',
    errorMessage: 'Failed to close session',
    invalidateKeys: [bowserKeys.sessionsActive()],
  });
}

export function useNavigate(sessionId: string, projectId: string) {
  return useMutationWithToast({
    mutationFn: (url: string) => bowserApi.navigate(sessionId, projectId, url),
    errorMessage: 'Failed to navigate',
    invalidateKeys: [bowserKeys.session(sessionId)],
  });
}

export function useAddToAllowlist() {
  return useMutationWithToast({
    mutationFn: bowserApi.addToAllowlist,
    successMessage: 'URL pattern added to allowlist',
    errorMessage: 'Failed to add to allowlist',
    invalidateKeys: [bowserKeys.all],
  });
}

export function useRemoveFromAllowlist() {
  return useMutationWithToast({
    mutationFn: bowserApi.removeFromAllowlist,
    successMessage: 'Removed from allowlist',
    errorMessage: 'Failed to remove from allowlist',
    invalidateKeys: [bowserKeys.all],
  });
}
