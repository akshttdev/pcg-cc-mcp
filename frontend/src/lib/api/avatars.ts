import type { AvatarProfile, CreateAvatarProfile } from 'shared/types';

import { ApiError, makeRequest, resolveApiUrl } from '@/lib/api/client';

export interface PortraitShot {
  slot: string;
  category: string;
  url: string;
  prompt: string;
  locked: boolean;
  generated_at: string;
}

export interface CharacterBible {
  age_range?: string | null;
  gender_presentation?: string | null;
  hair?: string | null;
  eyes?: string | null;
  skin?: string | null;
  build?: string | null;
  distinguishing_features?: string[] | null;
  wardrobe_defaults?: string | null;
  style_vibe?: string | null;
  do_not_do?: string[] | null;
}

// Avatar routes return raw JSON, not the ApiResponse envelope used elsewhere —
// parse directly rather than going through handleApiResponse.
async function rawJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const text = await response.text();
    throw new ApiError(
      text || `HTTP ${response.status}`,
      response.status,
      response
    );
  }
  return (await response.json()) as T;
}

export const avatarsApi = {
  list: async (orgId?: string): Promise<AvatarProfile[]> => {
    const qs = orgId ? `?org_id=${encodeURIComponent(orgId)}` : '';
    const r = await makeRequest(`/api/video-gen/avatars${qs}`);
    return rawJson<AvatarProfile[]>(r);
  },
  get: async (id: string): Promise<AvatarProfile> => {
    const r = await makeRequest(`/api/video-gen/avatars/${id}`);
    return rawJson<AvatarProfile>(r);
  },
  create: async (body: CreateAvatarProfile): Promise<AvatarProfile> => {
    const r = await makeRequest('/api/video-gen/avatars', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    return rawJson<AvatarProfile>(r);
  },
  uploadReference: async (id: string, file: File): Promise<AvatarProfile> => {
    const fd = new FormData();
    fd.append('image', file);
    const response = await fetch(
      resolveApiUrl(`/api/video-gen/avatars/${id}/upload-reference`),
      { method: 'POST', body: fd, credentials: 'include' }
    );
    return rawJson<AvatarProfile>(response);
  },
  generateProfile: async (id: string): Promise<AvatarProfile> => {
    const r = await makeRequest(
      `/api/video-gen/avatars/${id}/generate-profile`,
      {
        method: 'POST',
      }
    );
    return rawJson<AvatarProfile>(r);
  },
  delete: async (id: string): Promise<void> => {
    const r = await makeRequest(`/api/video-gen/avatars/${id}`, {
      method: 'DELETE',
    });
    if (!r.ok && r.status !== 204) {
      const text = await r.text();
      throw new ApiError(`Failed to delete: ${text}`, r.status, r);
    }
  },
};

export function parsePortraitSet(
  json: string | null | undefined
): PortraitShot[] {
  if (!json) return [];
  try {
    const parsed = JSON.parse(json);
    return Array.isArray(parsed) ? (parsed as PortraitShot[]) : [];
  } catch {
    return [];
  }
}

export function parseBible(
  json: string | null | undefined
): CharacterBible | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as CharacterBible;
  } catch {
    return null;
  }
}
