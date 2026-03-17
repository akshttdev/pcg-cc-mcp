import { makeRequest, handleApiResponse } from './client';

// ── Types ───────────────────────────────────────────────────────────────────

export interface TopsiStatusResponse {
  isActive: boolean;
  topsiId?: string;
  uptimeMs?: number;
  accessScope: string;
  projectsVisible: number;
  systemHealth?: number;
}

export interface TopologyOverview {
  totalNodes: number;
  totalEdges: number;
  totalClusters: number;
  systemHealth?: number;
}

export interface DetectedIssue {
  issueType: string;
  severity: string;
  description: string;
  affectedNodes: string[];
  suggestedAction?: string;
}

export interface ProjectAccess {
  projectId: string;
  projectName: string;
  role: string;
  grantedAt: string;
}

export interface TopsiChatResponse {
  message?: string;
}

export interface TopsiVoiceResponse {
  data: {
    transcription?: string;
    responseText?: string;
    audioResponse?: string;
  };
}

export interface Recommendation {
  taskId: string;
  taskName: string;
  reason: string;
  vibeEstimate: number | null;
  timeContext: string | null;
  followUps: string[];
}

export interface RecommendationBatch {
  recommendations: Recommendation[];
  summary: string;
  generatedAt: string;
}

// ── API Module ──────────────────────────────────────────────────────────────

export const topsiApi = {
  getStatus: (): Promise<TopsiStatusResponse> =>
    makeRequest('/api/topsi/status').then(handleApiResponse<TopsiStatusResponse>),

  initialize: (activateImmediately: boolean): Promise<unknown> =>
    makeRequest('/api/topsi/initialize', {
      method: 'POST',
      body: JSON.stringify({ activateImmediately }),
    }).then(handleApiResponse),

  getTopology: (): Promise<TopologyOverview> =>
    makeRequest('/api/topsi/topology').then(handleApiResponse<TopologyOverview>),

  getIssues: (): Promise<{ issues: DetectedIssue[] }> =>
    makeRequest('/api/topsi/issues').then(handleApiResponse<{ issues: DetectedIssue[] }>),

  getProjects: (): Promise<{ projects: ProjectAccess[] }> =>
    makeRequest('/api/topsi/projects').then(handleApiResponse<{ projects: ProjectAccess[] }>),

  chat: (
    data: { message: string; sessionId: string; projectId?: string | null; context?: unknown },
    signal?: AbortSignal,
  ): Promise<TopsiChatResponse> =>
    makeRequest('/api/topsi/chat', {
      method: 'POST',
      body: JSON.stringify(data),
      signal,
    }).then(handleApiResponse<TopsiChatResponse>),

  voiceInteraction: (sessionId: string, audioInput: string): Promise<TopsiVoiceResponse> =>
    makeRequest('/api/topsi/voice/interaction', {
      method: 'POST',
      body: JSON.stringify({ sessionId, audioInput }),
    }).then(handleApiResponse<TopsiVoiceResponse>),

  getRecommendations: (projectId?: string): Promise<RecommendationBatch> =>
    makeRequest(
      projectId
        ? `/api/topsi/recommendations/${projectId}`
        : '/api/topsi/recommendations',
    ).then(handleApiResponse<RecommendationBatch>),
};
