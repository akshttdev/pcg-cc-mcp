import { makeRequest, handleApiResponse } from './client';

// =====================
// Pulse Engine Types
// =====================
// These are manually defined (not generated from Rust via ts-rs) because the
// pulse engine API responses aren't covered by shared/types.ts. Keep in sync
// with the API response shapes — PulseWidget.tsx and pulse.tsx import from here.

export interface PulseSource {
  id: string;
  project_id: string;
  source_id: string;
  source_type: string;
  name: string;
  url: string;
  config?: Record<string, unknown>;
  enabled: boolean;
  status?: string;
  collection_interval_secs?: number;
  category?: string;
  created_at: string;
  updated_at: string;
}

export interface PulseContentItem {
  id: string;
  source_id: string;
  source_type?: string;
  title?: string;
  content?: string;
  summary?: string;
  url?: string;
  author?: string;
  status?: string;
  pcg_status?: string;
  published_at?: string;
  collected_at?: string;
  relevance_score?: number;
  content_hash?: string;
  created_at: string;
}

export interface PulseAlert {
  id: string;
  project_id: string;
  rule_id?: string;
  title: string;
  message?: string;
  priority?: string;
  acknowledged: boolean;
  created_at: string;
}

export interface PulseAlertRule {
  id: string;
  project_id: string;
  name: string;
  conditions: Record<string, unknown>;
  actions: Record<string, unknown>;
  priority?: string;
  trigger_count?: number;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface PulseTrackingConfig {
  keywords?: Record<string, unknown>;
  entities?: Record<string, unknown>;
  llm_enabled?: boolean;
  llm_model?: string;
  notification_config?: Record<string, unknown>;
}

export interface PulseEngineStatus {
  running: boolean;
  last_run_at?: string;
  next_run_at?: string;
}

export interface PulseCollectionRun {
  id: string;
  project_id: string;
  started_at: string;
  completed_at?: string;
  status: string;
  items_collected?: number;
}

export interface PulseProject {
  id: string;
  name: string;
  description: string | null;
  adapters: number;
  active_sources: string[];
  scheduler_running: boolean;
  source_count?: number;
  last_collection_at?: string;
}

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
    return handleApiResponse<PulseSource[]>(response);
  },
  createSource: async (projectId: string, data: { source_id: string; source_type: string; name: string; url: string; config?: Record<string, unknown>; enabled?: boolean; collection_interval_secs?: number; category?: string }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources`, {
      method: 'POST',
      body: JSON.stringify({ ...data, project_id: projectId }),
    });
    return handleApiResponse<PulseSource>(response);
  },
  updateSource: async (projectId: string, sourceId: string, data: { name?: string; url?: string; config?: Record<string, unknown>; enabled?: boolean; collection_interval_secs?: number; category?: string }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources/${sourceId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PulseSource>(response);
  },
  deleteSource: async (projectId: string, sourceId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/sources/${sourceId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
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
    return handleApiResponse<{ items: PulseContentItem[]; count: number }>(response);
  },
  getLatestContent: async (projectId: string, limit = 20) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/content/latest?limit=${limit}`);
    return handleApiResponse<{ items: PulseContentItem[]; count: number }>(response);
  },
  contentAction: async (projectId: string, contentId: string, action: { action: string; task_title?: string; task_description?: string; crm_contact_id?: string }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/content/${contentId}/action`, {
      method: 'POST',
      body: JSON.stringify(action),
    });
    return handleApiResponse<Record<string, unknown>>(response);
  },

  // Alerts
  getAlerts: async (projectId: string, limit = 50) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alerts?limit=${limit}`);
    return handleApiResponse<PulseAlert[]>(response);
  },
  getAlertRules: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules`);
    return handleApiResponse<PulseAlertRule[]>(response);
  },
  createAlertRule: async (projectId: string, data: { name: string; conditions: Record<string, unknown>; actions: Record<string, unknown>; priority?: string; enabled?: boolean }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules`, {
      method: 'POST',
      body: JSON.stringify({ ...data, project_id: projectId }),
    });
    return handleApiResponse<PulseAlertRule>(response);
  },
  updateAlertRule: async (projectId: string, ruleId: string, data: { name?: string; conditions?: Record<string, unknown>; actions?: Record<string, unknown>; priority?: string; enabled?: boolean }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules/${ruleId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PulseAlertRule>(response);
  },
  deleteAlertRule: async (projectId: string, ruleId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/alert-rules/${ruleId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  // Collection
  triggerCollection: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/collect`, {
      method: 'POST',
    });
    return handleApiResponse<Record<string, unknown>>(response);
  },
  getRuns: async (projectId: string, limit = 50) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/runs?limit=${limit}`);
    return handleApiResponse<PulseCollectionRun[]>(response);
  },

  // Tracking config
  getTrackingConfig: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/tracking`);
    return handleApiResponse<PulseTrackingConfig>(response);
  },
  updateTrackingConfig: async (projectId: string, data: { keywords?: Record<string, unknown>; entities?: Record<string, unknown>; llm_enabled?: boolean; llm_model?: string; notification_config?: Record<string, unknown> }) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/tracking`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PulseTrackingConfig>(response);
  },

  // Engine status
  getEngineStatus: async (projectId: string) => {
    const response = await makeRequest(`/api/pulse/projects/${projectId}/engine/status`);
    return handleApiResponse<PulseEngineStatus>(response);
  },

  // Global (non-project-scoped) endpoints
  listProjects: async (): Promise<{ projects: PulseProject[]; count: number }> => {
    const response = await makeRequest('/api/pulse/projects');
    return handleApiResponse<{ projects: PulseProject[]; count: number }>(response);
  },

  collectAll: async (): Promise<void> => {
    const response = await makeRequest('/api/pulse/collect', {
      method: 'POST',
    });
    return handleApiResponse<void>(response);
  },
};
