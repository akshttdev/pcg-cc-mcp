import { makeRequest, handleApiResponse } from './client';

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
