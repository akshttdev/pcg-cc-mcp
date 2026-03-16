import { makeRequest, handleApiResponse, ApiError } from './client';

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

// Virtual Spaces API
export interface VirtualSpaceRecord {
  space_name: string;
  host_username: string;
  world_x: number;
  spawn_x: number;
  spawn_y: number;
  spawn_z: number;
}

export const virtualSpacesApi = {
  list: async (): Promise<VirtualSpaceRecord[]> => {
    const r = await makeRequest('/api/virtual-spaces');
    return handleApiResponse<VirtualSpaceRecord[]>(r);
  },
};
