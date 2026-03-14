import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Bell, CheckCheck } from 'lucide-react';
import { resolveApiUrl } from '@/lib/api';
import type { ActivityItem, InboxNotification } from './types';
import { getProjectId } from './utils';
import { ActivityNotificationItem } from './ActivityNotificationItem';
import { InboxNotificationItem } from './InboxNotificationItem';

export function NotificationCenter() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [dismissedAt, setDismissedAt] = useState<string | null>(() => {
    try {
      return localStorage.getItem('orcha:activity-dismissed-at');
    } catch {
      return null;
    }
  });

  const { data: activityNotifications = [], isLoading: activityLoading } = useQuery({
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

  const { data: inboxNotifications = [], isLoading: inboxLoading } = useQuery({
    queryKey: ['notifications-inbox'],
    queryFn: async (): Promise<InboxNotification[]> => {
      const res = await fetch(resolveApiUrl('/api/notifications/inbox?limit=30'), {
        credentials: 'include',
      });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
    refetchInterval: 30000,
    staleTime: 10000,
  });

  const isLoading = activityLoading || inboxLoading;
  const unreadInbox = inboxNotifications.filter((n) => !n.read_at);
  const visibleNotifications = dismissedAt
    ? activityNotifications.filter((n) => new Date(n.timestamp) > new Date(dismissedAt))
    : activityNotifications;
  const unreadCount = visibleNotifications.length + unreadInbox.length;

  const persistDismissedAt = useCallback((ts: string) => {
    setDismissedAt(ts);
    try { localStorage.setItem('orcha:activity-dismissed-at', ts); } catch {}
  }, []);

  const handleMarkAllRead = useCallback(async () => {
    persistDismissedAt(new Date().toISOString());
    try {
      await fetch(resolveApiUrl('/api/notifications/mark-all-read'), {
        method: 'PUT',
        credentials: 'include',
      });
      queryClient.invalidateQueries({ queryKey: ['notifications-inbox'] });
    } catch {
      // Non-fatal
    }
  }, [queryClient, persistDismissedAt]);

  const handleActivityClick = useCallback(
    (item: ActivityItem) => {
      const projectId = getProjectId(item);
      if (projectId && item.task_id) {
        navigate(`/projects/${projectId}/tasks/${item.task_id}`);
      } else if (item.task_id) {
        navigate('/my-tasks');
      }
      persistDismissedAt(new Date().toISOString());
    },
    [navigate, persistDismissedAt],
  );

  const handleInboxClick = useCallback(
    async (item: InboxNotification) => {
      try {
        await fetch(resolveApiUrl(`/api/notifications/${item.id}/read`), {
          method: 'PUT',
          credentials: 'include',
        });
        queryClient.invalidateQueries({ queryKey: ['notifications-inbox'] });
      } catch {
        // Non-fatal
      }
      if (item.source === 'task' && item.source_id) {
        navigate('/my-tasks');
      } else if (item.source === 'workflow' && item.source_id) {
        navigate('/workflows');
      }
    },
    [navigate, queryClient],
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
          ) : visibleNotifications.length === 0 && unreadInbox.length === 0 ? (
            <div className="p-4 text-center text-sm text-muted-foreground">
              {dismissedAt ? 'All caught up' : 'No recent activity'}
            </div>
          ) : (
            <>
              {unreadInbox.map((item) => (
                <InboxNotificationItem
                  key={`inbox-${item.id}`}
                  item={item}
                  onClick={handleInboxClick}
                />
              ))}
              {visibleNotifications.map((item) => (
                <ActivityNotificationItem
                  key={item.id}
                  item={item}
                  onClick={handleActivityClick}
                />
              ))}
            </>
          )}
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
