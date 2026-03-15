import { makeRequest, handleApiResponse, resolveApiUrl } from './client';

// ============================================
// Artifact Review APIs
// ============================================

export interface ArtifactReview {
  id: string;
  artifact_id: string;
  reviewer_id?: string;
  reviewer_agent_id?: string;
  reviewer_name?: string;
  review_type: string;
  status: string;
  feedback_text?: string;
  rating?: number;
  revision_notes?: string;
  revision_deadline?: string;
  resolved_at?: string;
  resolved_by?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateArtifactReview {
  artifact_id: string;
  reviewer_id?: string;
  reviewer_agent_id?: string;
  reviewer_name?: string;
  review_type: string;
  feedback_text?: string;
  rating?: number;
  revision_notes?: Record<string, unknown>;
  revision_deadline?: string;
}

export interface ResolveReview {
  status: string;
  feedback_text?: string;
  rating?: number;
  resolved_by: string;
}

export const artifactReviewsApi = {
  list: async (params?: {
    artifact_id?: string;
    reviewer_id?: string;
    status?: string;
    pending_only?: boolean;
  }): Promise<ArtifactReview[]> => {
    const searchParams = new URLSearchParams();
    if (params?.artifact_id)
      searchParams.set('artifact_id', params.artifact_id);
    if (params?.reviewer_id)
      searchParams.set('reviewer_id', params.reviewer_id);
    if (params?.status) searchParams.set('status', params.status);
    if (params?.pending_only)
      searchParams.set('pending_only', params.pending_only.toString());
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/artifact-reviews${query ? `?${query}` : ''}`
    );
    return handleApiResponse<ArtifactReview[]>(response);
  },

  listPending: async (): Promise<ArtifactReview[]> => {
    const response = await makeRequest('/api/artifact-reviews/pending');
    return handleApiResponse<ArtifactReview[]>(response);
  },

  getById: async (reviewId: string): Promise<ArtifactReview> => {
    const response = await makeRequest(`/api/artifact-reviews/${reviewId}`);
    return handleApiResponse<ArtifactReview>(response);
  },

  create: async (data: CreateArtifactReview): Promise<ArtifactReview> => {
    const response = await makeRequest('/api/artifact-reviews', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ArtifactReview>(response);
  },

  delete: async (reviewId: string): Promise<void> => {
    const response = await makeRequest(`/api/artifact-reviews/${reviewId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  resolve: async (
    reviewId: string,
    data: ResolveReview
  ): Promise<ArtifactReview> => {
    const response = await makeRequest(
      `/api/artifact-reviews/${reviewId}/resolve`,
      {
        method: 'POST',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<ArtifactReview>(response);
  },
};

// ============================================
// Task Artifacts APIs
// ============================================

export interface TaskArtifact {
  id: string;
  task_id: string;
  artifact_id: string;
  artifact_role: string;
  display_order: number;
  pinned: boolean;
  added_by?: string;
  created_at: string;
}

export interface TaskArtifactWithDetails {
  link: TaskArtifact;
  artifact?: ExecutionArtifact;
}

export interface LinkArtifactToTask {
  artifact_id: string;
  artifact_role?: string;
  display_order?: number;
  pinned?: boolean;
  added_by?: string;
}

export interface ExecutionArtifact {
  id: string;
  execution_process_id?: string;
  artifact_type: string;
  title?: string;
  content?: string;
  file_path?: string;
  metadata?: string;
  phase?: string;
  created_by_agent_id?: string;
  review_status?: string;
  parent_artifact_id?: string;
  created_at: string;
}

export const taskArtifactsApi = {
  list: async (
    taskId: string,
    params?: { role?: string; pinned_only?: boolean }
  ): Promise<TaskArtifactWithDetails[]> => {
    const searchParams = new URLSearchParams();
    if (params?.role) searchParams.set('role', params.role);
    if (params?.pinned_only)
      searchParams.set('pinned_only', params.pinned_only.toString());
    const query = searchParams.toString();
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts${query ? `?${query}` : ''}`
    );
    return handleApiResponse<TaskArtifactWithDetails[]>(response);
  },

  link: async (
    taskId: string,
    data: LinkArtifactToTask
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(`/api/tasks/${taskId}/artifacts`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<TaskArtifact>(response);
  },

  unlink: async (taskId: string, artifactId: string): Promise<boolean> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}`,
      { method: 'DELETE' }
    );
    return handleApiResponse<boolean>(response);
  },

  updateRole: async (
    taskId: string,
    artifactId: string,
    role: string
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}/role`,
      {
        method: 'POST',
        body: JSON.stringify({ role }),
      }
    );
    return handleApiResponse<TaskArtifact>(response);
  },

  togglePin: async (
    taskId: string,
    artifactId: string
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}/pin`,
      { method: 'POST' }
    );
    return handleApiResponse<TaskArtifact>(response);
  },

  reorder: async (
    taskId: string,
    artifactId: string,
    newOrder: number
  ): Promise<TaskArtifact> => {
    const response = await makeRequest(
      `/api/tasks/${taskId}/artifacts/${artifactId}/reorder`,
      {
        method: 'POST',
        body: JSON.stringify({ new_order: newOrder }),
      }
    );
    return handleApiResponse<TaskArtifact>(response);
  },

  getByArtifact: async (artifactId: string): Promise<TaskArtifact[]> => {
    const response = await makeRequest(`/api/artifacts/${artifactId}/tasks`);
    return handleApiResponse<TaskArtifact[]>(response);
  },
};

// ============================================
// Artifact Content & Download API
// ============================================

export const artifactContentApi = {
  getContentUrl: (artifactId: string): string =>
    resolveApiUrl(`/api/artifacts/${artifactId}/content`),

  getDownloadUrl: (artifactId: string): string =>
    resolveApiUrl(`/api/artifacts/${artifactId}/download`),

  getFileUrl: (artifactId: string, filename: string): string =>
    resolveApiUrl(`/api/artifacts/${artifactId}/files/${encodeURIComponent(filename)}`),

  getContent: async (artifactId: string): Promise<unknown> => {
    const response = await makeRequest(`/api/artifacts/${artifactId}/content`);
    if (!response.ok) {
      throw new Error(`Failed to fetch artifact content: ${response.statusText}`);
    }
    return response.json();
  },
};

// ============================================
// Editron Export API
// ============================================

export const editronApi = {
  getExportXmlUrl: (artifactId: string): string =>
    resolveApiUrl(`/api/editron/export/${artifactId}?format=xml`),
};
