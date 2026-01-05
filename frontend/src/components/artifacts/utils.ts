import type { ArtifactPhase, ArtifactReviewStatus } from 'shared/types';

export type ArtifactMetadata = Partial<{
  phase: ArtifactPhase;
  created_by_agent_id: string | null;
  created_by_agent_name: string | null;
  created_by: string | null;
  review_status: ArtifactReviewStatus;
  created_by_user_id: string | null;
  preview_url: string | null;
  summary: string | null;
  tags: string[];
}> &
  Record<string, unknown>;

export function parseArtifactMetadata(
  metadata?: string | null
): ArtifactMetadata {
  if (!metadata) {
    return {};
  }

  try {
    const parsed = JSON.parse(metadata) as ArtifactMetadata;
    return parsed ?? {};
  } catch (error) {
    console.warn('Failed to parse artifact metadata', error);
    return {};
  }
}

export function formatArtifactPhase(phase?: ArtifactPhase): string {
  if (!phase) {
    return 'No phase';
  }

  return phase
    .split('_')
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(' ');
}
