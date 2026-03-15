import type {
  Project,
  CreateProject,
  UpdateProject,
  EditorType,
  GitBranch,
  SearchResult,
  ProjectBoard,
  CreateProjectBoard,
  UpdateProjectBoard,
  ProjectAsset,
  CreateProjectAsset,
  UpdateProjectAsset,
  BrandProfile,
  UpsertBrandProfile,
} from 'shared/types';
import { makeRequest, handleApiResponse } from './client';

type ProjectBoardCreateInput = Omit<CreateProjectBoard, 'project_id'>;
type ProjectBoardUpdateInput = UpdateProjectBoard;
type ProjectAssetCreateInput = Omit<CreateProjectAsset, 'project_id'>;
type ProjectAssetUpdateInput = UpdateProjectAsset;

// Project Management APIs
export const projectsApi = {
  getAll: async (): Promise<Project[]> => {
    const response = await makeRequest('/api/projects');
    return handleApiResponse<Project[]>(response);
  },

  getById: async (id: string): Promise<Project> => {
    const response = await makeRequest(`/api/projects/${id}`);
    return handleApiResponse<Project>(response);
  },

  getByClientId: async (clientId: string): Promise<Project[]> => {
    const response = await makeRequest(`/api/projects/by-client/${encodeURIComponent(clientId)}`);
    return handleApiResponse<Project[]>(response);
  },

  create: async (data: CreateProject): Promise<Project> => {
    const response = await makeRequest('/api/projects', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<Project>(response);
  },

  update: async (id: string, data: UpdateProject): Promise<Project> => {
    const response = await makeRequest(`/api/projects/${id}`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<Project>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/projects/${id}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  openEditor: async (id: string, editorType?: EditorType): Promise<void> => {
    const requestBody: any = {};
    if (editorType) requestBody.editor_type = editorType;

    const response = await makeRequest(`/api/projects/${id}/open-editor`, {
      method: 'POST',
      body: JSON.stringify(
        Object.keys(requestBody).length > 0 ? requestBody : null
      ),
    });
    return handleApiResponse<void>(response);
  },

  getBranches: async (id: string): Promise<GitBranch[]> => {
    const response = await makeRequest(`/api/projects/${id}/branches`);
    return handleApiResponse<GitBranch[]>(response);
  },

  searchFiles: async (
    id: string,
    query: string,
    mode?: string,
    options?: RequestInit
  ): Promise<SearchResult[]> => {
    const modeParam = mode ? `&mode=${encodeURIComponent(mode)}` : '';
    const response = await makeRequest(
      `/api/projects/${id}/search?q=${encodeURIComponent(query)}${modeParam}`,
      options
    );
    return handleApiResponse<SearchResult[]>(response);
  },

  listBoards: async (projectId: string): Promise<ProjectBoard[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/boards`);
    return handleApiResponse<ProjectBoard[]>(response);
  },

  createBoard: async (
    projectId: string,
    data: ProjectBoardCreateInput
  ): Promise<ProjectBoard> => {
    const response = await makeRequest(`/api/projects/${projectId}/boards`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProjectBoard>(response);
  },

  updateBoard: async (
    projectId: string,
    boardId: string,
    data: ProjectBoardUpdateInput
  ): Promise<ProjectBoard> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/boards/${boardId}`,
      {
        method: 'PATCH',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<ProjectBoard>(response);
  },

  deleteBoard: async (
    projectId: string,
    boardId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/boards/${boardId}`,
      {
        method: 'DELETE',
      }
    );
    return handleApiResponse<void>(response);
  },

  listAssets: async (projectId: string): Promise<ProjectAsset[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/assets`);
    return handleApiResponse<ProjectAsset[]>(response);
  },

  createAsset: async (
    projectId: string,
    data: ProjectAssetCreateInput
  ): Promise<ProjectAsset> => {
    // Convert bigint to number for JSON serialization
    const payload = {
      ...data,
      byte_size: data.byte_size !== undefined ? Number(data.byte_size) : undefined,
    };
    const response = await makeRequest(`/api/projects/${projectId}/assets`, {
      method: 'POST',
      body: JSON.stringify(payload),
    });
    return handleApiResponse<ProjectAsset>(response);
  },

  updateAsset: async (
    projectId: string,
    assetId: string,
    data: ProjectAssetUpdateInput
  ): Promise<ProjectAsset> => {
    // Convert bigint to number for JSON serialization
    const payload = {
      ...data,
      byte_size: data.byte_size !== undefined ? Number(data.byte_size) : undefined,
    };
    const response = await makeRequest(
      `/api/projects/${projectId}/assets/${assetId}`,
      {
        method: 'PATCH',
        body: JSON.stringify(payload),
      }
    );
    return handleApiResponse<ProjectAsset>(response);
  },

  deleteAsset: async (
    projectId: string,
    assetId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/assets/${assetId}`,
      {
        method: 'DELETE',
      }
    );
    return handleApiResponse<void>(response);
  },

  // Client Assignment
  setClient: async (projectId: string, clientId: string | null): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/client`, {
      method: 'PUT',
      body: JSON.stringify({ client_id: clientId }),
    });
    return handleApiResponse<void>(response);
  },

  // Brand Profile APIs
  getBrandProfile: async (projectId: string): Promise<BrandProfile | null> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/brand-profile`
    );
    return handleApiResponse<BrandProfile | null>(response);
  },

  upsertBrandProfile: async (
    projectId: string,
    data: UpsertBrandProfile
  ): Promise<BrandProfile> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/brand-profile`,
      {
        method: 'PUT',
        body: JSON.stringify(data),
      }
    );
    return handleApiResponse<BrandProfile>(response);
  },

  // Project hierarchy APIs
  setParent: async (projectId: string, parentProjectId: string | null): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/parent`, {
      method: 'PUT',
      body: JSON.stringify({ parent_project_id: parentProjectId }),
    });
    return handleApiResponse<void>(response);
  },

  reorder: async (projectId: string, sortOrder: number): Promise<void> => {
    const response = await makeRequest(`/api/projects/${projectId}/reorder`, {
      method: 'PUT',
      body: JSON.stringify({ sort_order: sortOrder }),
    });
    return handleApiResponse<void>(response);
  },
};

// Project Controller Types
export interface ProjectControllerConfig {
  id: string;
  project_id: string;
  name: string;
  personality: string;
  system_prompt: string | null;
  voice_id: string | null;
  avatar_url: string | null;
  model: string | null;
  temperature: number | null;
  max_tokens: number | null;
  created_at: string;
  updated_at: string;
}

export interface UpdateControllerConfig {
  name?: string;
  personality?: string;
  system_prompt?: string;
  voice_id?: string;
  avatar_url?: string;
  model?: string;
  temperature?: number;
  max_tokens?: number;
}

export interface ProjectControllerConversation {
  id: string;
  project_id: string;
  user_id: string;
  title: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProjectControllerMessage {
  id: string;
  conversation_id: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  tokens_used: number | null;
  created_at: string;
}

export interface ControllerChatResponse {
  message: ProjectControllerMessage;
  conversation_id: string;
}

// Project Controller APIs
export const projectControllersApi = {
  getConfig: async (projectId: string): Promise<ProjectControllerConfig> => {
    const response = await makeRequest(`/api/projects/${projectId}/controller`);
    return handleApiResponse<ProjectControllerConfig>(response);
  },

  updateConfig: async (
    projectId: string,
    data: UpdateControllerConfig
  ): Promise<ProjectControllerConfig> => {
    const response = await makeRequest(`/api/projects/${projectId}/controller`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProjectControllerConfig>(response);
  },

  getConversations: async (
    projectId: string
  ): Promise<ProjectControllerConversation[]> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/conversations`
    );
    return handleApiResponse<ProjectControllerConversation[]>(response);
  },

  getConversation: async (
    projectId: string,
    conversationId: string
  ): Promise<{ conversation: ProjectControllerConversation; messages: ProjectControllerMessage[] }> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/conversations/${conversationId}`
    );
    return handleApiResponse<{ conversation: ProjectControllerConversation; messages: ProjectControllerMessage[] }>(response);
  },

  deleteConversation: async (
    projectId: string,
    conversationId: string
  ): Promise<void> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/conversations/${conversationId}`,
      { method: 'DELETE' }
    );
    return handleApiResponse<void>(response);
  },

  sendMessage: async (
    projectId: string,
    content: string,
    conversationId?: string
  ): Promise<ControllerChatResponse> => {
    const response = await makeRequest(
      `/api/projects/${projectId}/controller/chat`,
      {
        method: 'POST',
        body: JSON.stringify({ content, conversation_id: conversationId }),
      }
    );
    return handleApiResponse<ControllerChatResponse>(response);
  },
};

// Assigned Task type for My Tasks
export interface AssignedTask {
  id: string;
  title: string;
  status: string;
  priority: string;
  due_date: string | null;
  project_id: string;
  project_name: string;
  description?: string | null;
  assigned_agent?: string | null;
  assignee_id?: string | null;
  created_by?: string | null;
  tags?: string | null;
}
