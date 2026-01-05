import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

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

// ========== API Functions ==========

async function fetchActiveSessions(): Promise<BrowserSession[]> {
  const response = await fetch('/api/bowser/sessions');
  if (!response.ok) throw new Error('Failed to fetch active sessions');
  const json = await response.json();
  return json.data;
}

async function fetchSession(sessionId: string): Promise<BrowserSession> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}`);
  if (!response.ok) throw new Error('Failed to fetch session');
  const json = await response.json();
  return json.data;
}

async function fetchSessionDetails(sessionId: string): Promise<BrowserSessionDetails> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}/details`);
  if (!response.ok) throw new Error('Failed to fetch session details');
  const json = await response.json();
  return json.data;
}

async function fetchScreenshots(sessionId: string): Promise<BrowserScreenshot[]> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}/screenshots`);
  if (!response.ok) throw new Error('Failed to fetch screenshots');
  const json = await response.json();
  return json.data;
}

async function fetchScreenshotsWithDiffs(sessionId: string): Promise<BrowserScreenshot[]> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}/screenshots/diffs`);
  if (!response.ok) throw new Error('Failed to fetch screenshots with diffs');
  const json = await response.json();
  return json.data;
}

async function fetchActions(sessionId: string): Promise<BrowserAction[]> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}/actions`);
  if (!response.ok) throw new Error('Failed to fetch actions');
  const json = await response.json();
  return json.data;
}

async function fetchAllowlist(projectId: string): Promise<BrowserAllowlist[]> {
  const response = await fetch(`/api/bowser/projects/${projectId}/allowlist`);
  if (!response.ok) throw new Error('Failed to fetch allowlist');
  const json = await response.json();
  return json.data;
}

async function fetchSummary(): Promise<BowserSummary> {
  const response = await fetch('/api/bowser/summary');
  if (!response.ok) throw new Error('Failed to fetch Bowser summary');
  const json = await response.json();
  return json.data;
}

async function checkUrl(projectId: string, url: string): Promise<{ allowed: boolean; url: string }> {
  const response = await fetch(`/api/bowser/projects/${projectId}/check-url`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ url }),
  });
  if (!response.ok) throw new Error('Failed to check URL');
  const json = await response.json();
  return json.data;
}

async function startSession(data: {
  execution_process_id: string;
  browser_type?: BrowserType;
  viewport_width?: number;
  viewport_height?: number;
  headless?: boolean;
}): Promise<BrowserSession> {
  const response = await fetch('/api/bowser/sessions', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to start session');
  const json = await response.json();
  return json.data;
}

async function closeSession(sessionId: string): Promise<BrowserSession> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}/close`, {
    method: 'POST',
  });
  if (!response.ok) throw new Error('Failed to close session');
  const json = await response.json();
  return json.data;
}

async function navigate(sessionId: string, projectId: string, url: string): Promise<BrowserAction> {
  const response = await fetch(`/api/bowser/sessions/${sessionId}/navigate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ url, project_id: projectId }),
  });
  if (!response.ok) throw new Error('Failed to navigate');
  const json = await response.json();
  return json.data;
}

async function addToAllowlist(data: {
  project_id?: string;
  pattern: string;
  pattern_type?: PatternType;
  description?: string;
  is_global?: boolean;
}): Promise<BrowserAllowlist> {
  const response = await fetch('/api/bowser/allowlist', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data),
  });
  if (!response.ok) throw new Error('Failed to add to allowlist');
  const json = await response.json();
  return json.data;
}

async function removeFromAllowlist(entryId: string): Promise<void> {
  const response = await fetch(`/api/bowser/allowlist/${entryId}`, {
    method: 'DELETE',
  });
  if (!response.ok) throw new Error('Failed to remove from allowlist');
}

// ========== Hooks ==========

export function useBowserSummary() {
  return useQuery({
    queryKey: ['bowser', 'summary'],
    queryFn: fetchSummary,
    refetchInterval: 5000,
  });
}

export function useActiveSessions() {
  return useQuery({
    queryKey: ['bowser', 'sessions', 'active'],
    queryFn: fetchActiveSessions,
    refetchInterval: 3000,
  });
}

export function useSession(sessionId: string | undefined) {
  return useQuery({
    queryKey: ['bowser', 'session', sessionId],
    queryFn: () => fetchSession(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 2000,
  });
}

export function useSessionDetails(sessionId: string | undefined) {
  return useQuery({
    queryKey: ['bowser', 'session', sessionId, 'details'],
    queryFn: () => fetchSessionDetails(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 2000,
  });
}

export function useScreenshots(sessionId: string | undefined) {
  return useQuery({
    queryKey: ['bowser', 'session', sessionId, 'screenshots'],
    queryFn: () => fetchScreenshots(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 3000,
  });
}

export function useScreenshotsWithDiffs(sessionId: string | undefined) {
  return useQuery({
    queryKey: ['bowser', 'session', sessionId, 'screenshots', 'diffs'],
    queryFn: () => fetchScreenshotsWithDiffs(sessionId!),
    enabled: !!sessionId,
  });
}

export function useActions(sessionId: string | undefined) {
  return useQuery({
    queryKey: ['bowser', 'session', sessionId, 'actions'],
    queryFn: () => fetchActions(sessionId!),
    enabled: !!sessionId,
    refetchInterval: 2000,
  });
}

export function useAllowlist(projectId: string | undefined) {
  return useQuery({
    queryKey: ['bowser', 'allowlist', projectId],
    queryFn: () => fetchAllowlist(projectId!),
    enabled: !!projectId,
  });
}

export function useCheckUrl(projectId: string | undefined) {
  return useMutation({
    mutationFn: (url: string) => checkUrl(projectId!, url),
  });
}

export function useStartSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: startSession,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bowser', 'sessions'] });
    },
  });
}

export function useCloseSession() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: closeSession,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bowser', 'sessions'] });
    },
  });
}

export function useNavigate(sessionId: string, projectId: string) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (url: string) => navigate(sessionId, projectId, url),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bowser', 'session', sessionId] });
    },
  });
}

export function useAddToAllowlist() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: addToAllowlist,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bowser', 'allowlist'] });
    },
  });
}

export function useRemoveFromAllowlist() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: removeFromAllowlist,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['bowser', 'allowlist'] });
    },
  });
}
