import { makeRequest, handleApiResponse, ApiError, resolveApiUrl } from './client';

// ── Types ────────────────────────────────────────────────────────────────

export interface CloudFile {
  id: string;
  organization_id: string;
  project_id: string | null;
  task_id: string | null;
  file_name: string;
  file_path: string;
  storage_volume: string;
  content_hash: string | null;
  file_size_bytes: number;
  mime_type: string | null;
  source_type: string;
  source_id: string | null;
  source_table: string | null;
  visibility: string;
  contributed_by: string | null;
  contributor_wallet: string | null;
  contributor_device: string | null;
  created_at: string;
  updated_at: string;
  deleted_at: string | null;
}

export interface CloudContribution {
  id: string;
  organization_id: string;
  cloud_file_id: string;
  user_id: string | null;
  wallet_address: string | null;
  device_id: string | null;
  contribution_type: string;
  channel: string;
  project_id: string | null;
  task_id: string | null;
  description: string | null;
  metadata: string;
  created_at: string;
}

export interface OrgCloudSettings {
  organization_id: string;
  storage_quota_bytes: number;
  viewer_can_download: boolean;
  member_can_upload: boolean;
  auto_index_data_sources: boolean;
  auto_index_artifacts: boolean;
  auto_index_media: boolean;
  nats_contribution_enabled: boolean;
  updated_at: string;
}

export interface CloudBrowseResponse {
  files: CloudFile[];
  total: number;
  page: number;
  per_page: number;
}

export interface CloudStats {
  total_files: number;
  total_size_bytes: number;
  total_contributions: number;
  quota_bytes: number;
}

export interface CloudBrowseParams {
  volume?: string;
  project_id?: string;
  task_id?: string;
  mime_type?: string;
  search?: string;
  visibility?: string;
  page?: number;
  per_page?: number;
}

// ── API Client ───────────────────────────────────────────────────────────

export const orgCloudApi = {
  browse: async (orgId: string, params?: CloudBrowseParams): Promise<CloudBrowseResponse> => {
    const qs = new URLSearchParams();
    if (params) {
      Object.entries(params).forEach(([k, v]) => {
        if (v !== undefined && v !== null && v !== '') qs.set(k, String(v));
      });
    }
    const query = qs.toString();
    const url = `/api/org-cloud/${orgId}/browse${query ? `?${query}` : ''}`;
    const response = await makeRequest(url);
    return handleApiResponse<CloudBrowseResponse>(response);
  },

  downloadUrl: (orgId: string, fileId: string): string => {
    return resolveApiUrl(`/api/org-cloud/${orgId}/files/${fileId}/download`);
  },

  previewUrl: (orgId: string, fileId: string): string => {
    return resolveApiUrl(`/api/org-cloud/${orgId}/files/${fileId}/preview`);
  },

  contribute: async (orgId: string, formData: FormData): Promise<CloudFile> => {
    const response = await fetch(resolveApiUrl(`/api/org-cloud/${orgId}/contribute`), {
      method: 'POST',
      body: formData,
      credentials: 'include',
    });
    if (!response.ok) {
      const errorText = await response.text().catch(() => 'Upload failed');
      throw new ApiError(`Failed to upload: ${errorText}`, response.status, response);
    }
    const json = await response.json();
    return json.data as CloudFile;
  },

  getStats: async (orgId: string): Promise<CloudStats> => {
    const response = await makeRequest(`/api/org-cloud/${orgId}/stats`);
    return handleApiResponse<CloudStats>(response);
  },

  getSettings: async (orgId: string): Promise<OrgCloudSettings> => {
    const response = await makeRequest(`/api/org-cloud/${orgId}/settings`);
    return handleApiResponse<OrgCloudSettings>(response);
  },

  updateSettings: async (orgId: string, data: Partial<OrgCloudSettings>): Promise<OrgCloudSettings> => {
    const response = await makeRequest(`/api/org-cloud/${orgId}/settings`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrgCloudSettings>(response);
  },

  getContributions: async (orgId: string, limit?: number): Promise<CloudContribution[]> => {
    const qs = limit ? `?limit=${limit}` : '';
    const response = await makeRequest(`/api/org-cloud/${orgId}/contributions${qs}`);
    return handleApiResponse<CloudContribution[]>(response);
  },

  triggerIndex: async (orgId: string): Promise<{ indexed_count: number }> => {
    const response = await makeRequest(`/api/org-cloud/${orgId}/index`, { method: 'POST' });
    return handleApiResponse<{ indexed_count: number }>(response);
  },

  updateFile: async (orgId: string, fileId: string, data: { file_name?: string; visibility?: string; project_id?: string; task_id?: string }): Promise<CloudFile> => {
    const response = await makeRequest(`/api/org-cloud/${orgId}/files/${fileId}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CloudFile>(response);
  },
};
