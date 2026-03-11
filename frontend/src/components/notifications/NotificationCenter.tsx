import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Bell, CheckCircle2, Edit, Plus, Trash2, ArrowRight, MessageSquare, CheckCheck } from 'lucide-react';
import { resolveApiUrl } from '@/lib/api';

function timeAgo(dateStr: string): string {
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

interface ActivityItem {
  id: string;
  task_id: string;
  actor_id: string;
  actor_type: string;
  action: string;
  previous_state: string | null;
  new_state: string | null;
  metadata: string | null;
  timestamp: string;
}

function getActionIcon(action: string) {
  switch (action) {
    case 'created':
    case 'create':
    case 'task_created':
      return <Plus className="h-3 w-3 text-green-500" />;
    case 'updated':
    case 'update':
    case 'task_updated':
      return <Edit className="h-3 w-3 text-blue-500" />;
    case 'status_change':
      return <ArrowRight className="h-3 w-3 text-yellow-500" />;
    case 'complete':
    case 'done':
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case 'deleted':
    case 'delete':
      return <Trash2 className="h-3 w-3 text-red-500" />;
    case 'comment':
      return <MessageSquare className="h-3 w-3 text-purple-500" />;
    default:
      return <Bell className="h-3 w-3 text-muted-foreground" />;
  }
}

function formatActor(item: ActivityItem): string {
  if (item.actor_type === 'agent') return 'Agent';
  if (item.actor_type === 'system') return 'System';
  if (item.actor_id === 'current-user' || !item.actor_id) return 'You';
  return item.actor_id;
}

function formatAction(item: ActivityItem): string {
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

/** Extract project_id from metadata if available */
function getProjectId(item: ActivityItem): string | null {
  try {
    if (item.metadata) {
      const meta = JSON.parse(item.metadata);
      if (meta.project_id) return meta.project_id;
    }
  } catch {}
  return null;
}

// TODO: Activity data unification
// This component fetches from GET /api/notifications which returns task-level ActivityLog entries.
// The org overview "Recent Activity" section uses crmActivitiesApi.listActivities() for CRM events.
// These should be unified into a single activity feed. Options:
// 1. Server-side: merge ActivityLog + crm_activities into one endpoint
// 2. Server-side: write CRM events (deal created, contact added) into ActivityLog table
// 3. Client-side: query both APIs and merge by timestamp (interim solution)
// Also: dismissedAt is client-side only (resets on refresh). Future: persist read state server-side.
export function NotificationCenter() {
  const navigate = useNavigate();
  const [dismissedAt, setDismissedAt] = useState<string | null>(null);

  const { data: notifications = [], isLoading } = useQuery({
    queryKey: ['notifications'],
    queryFn: async (): Promise<ActivityItem[]> => {
      const res = await fetch(resolveApiUrl('/api/notifications?limit=30'), {
        credentials: 'include',
      });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
    refetchInterval: 30000,
    staleTime: 10000,
  });

  // Filter to only show unread items (newer than last dismiss)
  const visibleNotifications = dismissedAt
    ? notifications.filter((n) => new Date(n.timestamp) > new Date(dismissedAt))
    : notifications;

  const unreadCount = visibleNotifications.length;

  const handleMarkAllRead = useCallback(() => {
    setDismissedAt(new Date().toISOString());
  }, []);

  const handleItemClick = useCallback(
    (item: ActivityItem) => {
      const projectId = getProjectId(item);
      if (projectId && item.task_id) {
        // Navigate to the project tasks page — the task can be found there
        navigate(`/projects/${projectId}/tasks`);
      } else if (item.task_id) {
        // Fallback: navigate to my tasks
        navigate('/my-tasks');
      }
    },
    [navigate],
  );

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="relative" aria-label="Notifications">
          <Bell className="h-4 w-4" />
          {unreadCount > 0 && (
            <span className="absolute top-1.5 right-1.5 h-2 w-2 rounded-full bg-blue-500" />
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-80" align="end">
        <DropdownMenuLabel className="font-normal">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-semibold">Activity</p>
              <p className="text-xs text-muted-foreground">Recent activity across your projects</p>
            </div>
            {unreadCount > 0 && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 text-xs gap-1 text-muted-foreground hover:text-foreground"
                onClick={handleMarkAllRead}
              >
                <CheckCheck className="h-3 w-3" />
                Mark all read
              </Button>
            )}
          </div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <div className="max-h-80 overflow-y-auto">
          {isLoading ? (
            <div className="p-4 text-center text-sm text-muted-foreground">Loading...</div>
          ) : visibleNotifications.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              {dismissedAt ? 'All caught up' : 'No recent activity'}
            </div>
          ) : (
            visibleNotifications.map((item) => (
              <div
                key={item.id}
                className="px-3 py-2 transition-colors cursor-pointer hover:bg-muted/50 bg-blue-50/50 dark:bg-blue-950/20"
                onClick={() => handleItemClick(item)}
              >
                <div className="flex items-start gap-2">
                  <div className="mt-0.5 shrink-0">{getActionIcon(item.action)}</div>
                  <div className="flex-1 min-w-0">
                    <p className="text-xs leading-tight">{formatAction(item)}</p>
                    <p className="text-[10px] text-muted-foreground mt-0.5">
                      {timeAgo(item.timestamp)}
                    </p>
                  </div>
                  <div className="mt-1.5 h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />
                </div>
              </div>
            ))
          )}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
