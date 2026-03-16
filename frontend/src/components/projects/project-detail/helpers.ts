import type { ProjectAsset, ProjectBoard } from 'shared/types';

export function hexToRgba(hex: string, alpha: number) {
  const sanitized = hex.replace('#', '');
  if (sanitized.length !== 6) {
    return `rgba(0, 0, 0, ${alpha})`;
  }
  const value = Number.parseInt(sanitized, 16);
  const r = (value >> 16) & 255;
  const g = (value >> 8) & 255;
  const b = value & 255;
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

export function formatStatusLabel(value: string) {
  return value
    .split('_')
    .map((chunk) => chunk.charAt(0).toUpperCase() + chunk.slice(1))
    .join(' ');
}

export function formatBoardLabel(
  value: ProjectBoard['board_type'],
  boardTypeLabels: Record<ProjectBoard['board_type'], string>
) {
  return boardTypeLabels[value] ?? formatStatusLabel(value);
}

export function formatByteSize(value: ProjectAsset['byte_size']) {
  if (!value) return '\u2014';
  const size = Number(value);
  if (!Number.isFinite(size) || size <= 0) return '\u2014';
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(
    Math.floor(Math.log10(size) / Math.log10(1024)),
    units.length - 1
  );
  const formatted = size / 1024 ** index;
  return `${formatted.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
}

export function formatRelativeTime(dateString?: string | null) {
  if (!dateString) return '\u2014';
  const timestamp = new Date(dateString).getTime();
  if (Number.isNaN(timestamp)) return '\u2014';
  const diffMs = Date.now() - timestamp;
  const diffMinutes = Math.floor(diffMs / 60000);
  if (diffMinutes < 1) return 'just now';
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 30) return `${diffDays}d ago`;
  const diffMonths = Math.floor(diffDays / 30);
  if (diffMonths < 12) return `${diffMonths}mo ago`;
  const diffYears = Math.floor(diffMonths / 12);
  return `${diffYears}y ago`;
}

export function slugify(value: string) {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '');
}

export function getBrandInitials(name?: string) {
  if (!name) return 'PR';
  const parts = name.split(/\s+/).filter(Boolean);
  if (parts.length === 0) return 'PR';
  if (parts.length === 1) {
    const [first] = parts;
    return first.slice(0, 2).toUpperCase();
  }
  return `${parts[0][0]}${parts[1][0]}`.toUpperCase();
}

export function getBrandTagline(project: { git_repo_path?: string | null; dev_script?: string | null } | null) {
  if (!project) return 'Centralized brand + ops workspace.';
  const repoPath = project.git_repo_path?.trim();
  if (repoPath && repoPath !== '.') {
    return `Source of truth: ${repoPath}`;
  }
  if (project.dev_script) {
    return `Runs ${project.dev_script} with live previews.`;
  }
  return 'Orchestrate brand systems, deliverables, and agents in one view.';
}

export function getRepoLabel(gitRepoPath?: string | null) {
  if (!gitRepoPath) return 'Not linked';
  const normalized = gitRepoPath.replace(/\.git$/, '');
  const segments = normalized.split('/');
  if (segments.length >= 2) {
    return segments.slice(-2).join('/');
  }
  return normalized;
}

export const BOARD_TYPE_LABELS: Record<ProjectBoard['board_type'], string> = {
  default: 'Main Board',
  custom: 'Custom',
  brand_assets: 'Brand Assets',
  executive_assets: 'Executive Assets',
};

/** Simplified board types available for user creation */
export const BOARD_TYPE_OPTIONS: ProjectBoard['board_type'][] = ['default', 'custom'];

export const ASSET_CATEGORY_OPTIONS = ['file', 'transcript', 'link', 'note'] as const;
export const ASSET_SCOPE_OPTIONS = ['owner', 'client', 'team', 'public'] as const;
