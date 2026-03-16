import { makeRequest, handleApiResponse } from './client';

// Shared segment/status types (used by both project and org onboarding)
export interface OnboardingSegmentData {
  id: string;
  segment_type: string;
  name: string;
  assigned_agent_id?: string | null;
  assigned_agent_name?: string | null;
  status: string;
  recommendations?: string | null;
  user_decisions?: string | null;
  order_index: number;
  started_at?: string | null;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrgOnboardingData {
  id: string;
  organization_id: string;
  status: string;
  current_phase: string;
  context_data?: string | null;
  recommendations?: string | null;
  started_at: string;
  completed_at?: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrgOnboardingWithSegments {
  onboarding: OrgOnboardingData;
  segments: (OnboardingSegmentData & { organization_id: string; onboarding_id: string })[];
}

// --- Organization Onboarding (primary) ---

export const orgOnboardingApi = {
  getByOrg: async (orgId: string): Promise<OrgOnboardingWithSegments | null> => {
    const response = await makeRequest(`/api/onboarding/organization/${orgId}`);
    if (response.status === 404) return null;
    return handleApiResponse<OrgOnboardingWithSegments>(response);
  },

  startOrg: async (orgId: string, contextData?: string): Promise<OrgOnboardingWithSegments> => {
    const response = await makeRequest(`/api/onboarding/organization/${orgId}/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ context_data: contextData }),
    });
    return handleApiResponse<OrgOnboardingWithSegments>(response);
  },

  updateOrg: async (id: string, data: Record<string, unknown>): Promise<OrgOnboardingData> => {
    const response = await makeRequest(`/api/onboarding/organization/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrgOnboardingData>(response);
  },

  getOrgSegments: async (onboardingId: string): Promise<OrgOnboardingWithSegments['segments']> => {
    const response = await makeRequest(`/api/onboarding/organization/${onboardingId}/segments`);
    return handleApiResponse<OrgOnboardingWithSegments['segments']>(response);
  },

  getOrgSegment: async (segmentId: string): Promise<OrgOnboardingWithSegments['segments'][0]> => {
    const response = await makeRequest(`/api/onboarding/organization-segment/${segmentId}`);
    return handleApiResponse<OrgOnboardingWithSegments['segments'][0]>(response);
  },

  updateOrgSegment: async (segmentId: string, data: Record<string, unknown>): Promise<OrgOnboardingWithSegments['segments'][0]> => {
    const response = await makeRequest(`/api/onboarding/organization-segment/${segmentId}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrgOnboardingWithSegments['segments'][0]>(response);
  },

  startOrgSegment: async (segmentId: string): Promise<OrgOnboardingWithSegments['segments'][0]> => {
    const response = await makeRequest(`/api/onboarding/organization-segment/${segmentId}/start`, {
      method: 'POST',
    });
    return handleApiResponse<OrgOnboardingWithSegments['segments'][0]>(response);
  },

  completeOrgSegment: async (
    segmentId: string,
    opts?: { user_decisions?: string; skip?: boolean }
  ): Promise<OrgOnboardingWithSegments['segments'][0]> => {
    const response = await makeRequest(`/api/onboarding/organization-segment/${segmentId}/complete`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(opts ?? {}),
    });
    return handleApiResponse<OrgOnboardingWithSegments['segments'][0]>(response);
  },
};

// --- Project Onboarding (DEPRECATED — use orgOnboardingApi) ---
// Retained for backwards compatibility. See planning/2026-03-16--plan--ux-engagement-polish-sprint2.md

export interface ProjectOnboardingWithSegments {
  onboarding: {
    id: string;
    project_id: string;
    status: string;
    current_phase: string;
    context_data?: string | null;
    recommendations?: string | null;
    started_at: string;
    completed_at?: string | null;
    created_at: string;
    updated_at: string;
  };
  segments: (OnboardingSegmentData & { project_id: string; onboarding_id: string })[];
}

/** @deprecated Use orgOnboardingApi instead */
export const onboardingApi = {
  /** @deprecated Use orgOnboardingApi.getByOrg */
  getByProject: async (projectId: string): Promise<ProjectOnboardingWithSegments | null> => {
    const response = await makeRequest(`/api/onboarding/project/${projectId}`);
    if (response.status === 404) return null;
    return handleApiResponse<ProjectOnboardingWithSegments>(response);
  },

  /** @deprecated Use orgOnboardingApi.startOrg */
  start: async (projectId: string, contextData?: string): Promise<ProjectOnboardingWithSegments> => {
    const response = await makeRequest(`/api/onboarding/project/${projectId}/start`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ context_data: contextData }),
    });
    return handleApiResponse<ProjectOnboardingWithSegments>(response);
  },
};
