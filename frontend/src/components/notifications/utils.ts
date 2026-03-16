import type { ActivityItem } from './types';

// ── Read-activity localStorage persistence ─────────────────────────────────

const READ_ACTIVITY_IDS_KEY = 'orcha:read-activity-ids';

export function loadReadActivityIds(): Set<string> {
  try {
    const raw = localStorage.getItem(READ_ACTIVITY_IDS_KEY);
    return raw ? new Set(JSON.parse(raw) as string[]) : new Set();
  } catch {
    return new Set();
  }
}

export function persistReadActivityIds(ids: Set<string>) {
  try {
    localStorage.setItem(READ_ACTIVITY_IDS_KEY, JSON.stringify([...ids]));
  } catch { /* non-fatal */ }
}

export function timeAgo(dateStr: string): string {
  const now = Date.now();
  const then = new Date(dateStr).getTime();
  const diff = Math.max(0, now - then);
  const seconds = Math.floor(diff / 1000);
  if (seconds < 60) return 'just now';
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}d ago`;
  return new Date(dateStr).toLocaleDateString();
}

function formatActor(item: ActivityItem): string {
  if (item.actor_type === 'agent') return 'Agent';
  if (item.actor_type === 'system') return 'System';
  if (item.actor_id === 'current-user' || !item.actor_id) return 'You';
  if (item.actor_name) return item.actor_name;
  return item.actor_id;
}

/** Extract title from metadata or new_state JSON. */
function extractTitle(item: ActivityItem): string | undefined {
  try {
    if (item.metadata) {
      const m = JSON.parse(item.metadata);
      if (m.title) return m.title;
    }
  } catch {}
  try {
    if (item.new_state) {
      const s = JSON.parse(item.new_state);
      if (s.title) return s.title;
    }
  } catch {}
  return undefined;
}

export function formatAction(item: ActivityItem): string {
  const actor = formatActor(item);
  let meta: Record<string, unknown> = {};
  try {
    if (item.metadata) meta = JSON.parse(item.metadata);
  } catch {}

  const title = extractTitle(item);

  switch (item.action) {
    case 'created':
    case 'create':
    case 'task_created':
      return `${actor} created${title ? ` "${title}"` : ' a task'}`;
    case 'updated':
    case 'update':
    case 'task_updated':
      if (meta.fields_changed && Array.isArray(meta.fields_changed)) {
        return `${actor} updated ${(meta.fields_changed as string[]).join(', ')}${title ? ` on "${title}"` : ''}`;
      }
      return `${actor} updated${title ? ` "${title}"` : ' a task'}`;
    case 'status_change':
      return `${actor} changed status${meta.to ? ` to ${meta.to}` : ''}${title ? ` on "${title}"` : ''}`;
    case 'comment':
      return `${actor} commented${title ? ` on "${title}"` : ''}`;
    case 'create_and_start':
      return `${actor} started execution`;
    case 'deleted':
    case 'delete':
      return `${actor} deleted${title ? ` "${title}"` : ' an item'}`;
    default:
      return `${actor} ${item.action.replace(/_/g, ' ')}`;
  }
}

export function getProjectId(item: ActivityItem): string | null {
  try {
    if (item.metadata) {
      const meta = JSON.parse(item.metadata);
      if (meta.project_id) return meta.project_id;
    }
  } catch {}
  return null;
}
