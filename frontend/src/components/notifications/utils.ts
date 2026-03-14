import type { ActivityItem } from './types';

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
  return item.actor_id;
}

export function formatAction(item: ActivityItem): string {
  const actor = formatActor(item);
  let meta: Record<string, any> = {};
  try {
    if (item.metadata) meta = JSON.parse(item.metadata);
  } catch {}

  switch (item.action) {
    case 'created':
    case 'create':
    case 'task_created':
      return `${actor} created${meta.title ? ` "${meta.title}"` : ' a task'}`;
    case 'updated':
    case 'update':
    case 'task_updated':
      if (meta.fields_changed) {
        return `${actor} updated ${meta.fields_changed.join(', ')}`;
      }
      return `${actor} updated a task`;
    case 'status_change':
      return `${actor} changed status${meta.to ? ` to ${meta.to}` : ''}`;
    case 'comment':
      return `${actor} commented`;
    case 'create_and_start':
      return `${actor} started execution`;
    case 'deleted':
    case 'delete':
      return `${actor} deleted${meta.title ? ` "${meta.title}"` : ' an item'}`;
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
