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
import { Bell, CheckCheck, ArrowRight } from 'lucide-react';
import { makeRequest } from '@/lib/api';
import { notificationKeys } from '@/lib/query-keys';
import type { ActivityItem, InboxNotification } from './types';
import { getProjectId, loadReadActivityIds, persistReadActivityIds } from './utils';
import { ActivityNotificationItem } from './ActivityNotificationItem';
import { InboxNotificationItem } from './InboxNotificationItem';

type NotificationTab = 'all' | 'inbox' | 'activity';

export function NotificationCenter() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<NotificationTab>('all');
  const [dismissedAt, setDismissedAt] = useState<string | null>(() => {
    try {
      return localStorage.getItem('orcha:activity-dismissed-at');
    } catch {
      return null;
    }
  });
  const [readActivityIds, setReadActivityIds] = useState<Set<string>>(loadReadActivityIds);

  const { data: activityNotifications = [], isLoading: activityLoading } = useQuery({
    queryKey: notificationKeys.activity(),
    queryFn: async (): Promise<ActivityItem[]> => {
      const res = await makeRequest('/api/notifications?limit=30');
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
    refetchInterval: 30000,
    staleTime: 10000,
  });

  const { data: inboxNotifications = [], isLoading: inboxLoading } = useQuery({
    queryKey: notificationKeys.inbox(),
    queryFn: async (): Promise<InboxNotification[]> => {
      const res = await makeRequest('/api/notifications/inbox?limit=30');
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
    refetchInterval: 30000,
    staleTime: 10000,
    retry: 1,
    retryDelay: 5000,
  });

  const isLoading = activityLoading || inboxLoading;
  const unreadInbox = inboxNotifications.filter((n) => !n.read_at);
  const visibleNotifications = dismissedAt
    ? activityNotifications.filter((n) => new Date(n.timestamp) > new Date(dismissedAt))
    : activityNotifications;
  const unreadActivityCount = visibleNotifications.filter((n) => !readActivityIds.has(n.id)).length;
  const unreadCount = unreadActivityCount + unreadInbox.length;

  const persistDismissedAt = useCallback((ts: string) => {
    setDismissedAt(ts);
    try { localStorage.setItem('orcha:activity-dismissed-at', ts); } catch { /* non-fatal */ }
  }, []);

  const handleMarkAllRead = useCallback(async () => {
    // Mark all activity as dismissed
    persistDismissedAt(new Date().toISOString());
    // Also mark all inbox as read via API
    try {
      await makeRequest('/api/notifications/mark-all-read', { method: 'PUT' });
      queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
    } catch {
      // Non-fatal
    }
  }, [queryClient, persistDismissedAt]);

  const handleActivityClick = useCallback(
    (item: ActivityItem) => {
      // Mark this individual activity as read
      setReadActivityIds((prev) => {
        const next = new Set(prev);
        next.add(item.id);
        persistReadActivityIds(next);
        return next;
      });

      // Deep-link to the task
      const projectId = getProjectId(item);
      if (projectId && item.task_id) {
        navigate(`/projects/${projectId}/tasks/${item.task_id}`);
      } else if (item.task_id) {
        navigate('/my-tasks');
      }
    },
    [navigate],
  );

  const handleDismiss = useCallback(
    async (item: InboxNotification) => {
      try {
        await makeRequest(`/api/notifications/${item.id}/read`, { method: 'PUT' });
        queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
      } catch {
        // Non-fatal
      }
    },
    [queryClient],
  );

  const handleInboxClick = useCallback(
    async (item: InboxNotification) => {
      // Mark as read via API
      try {
        await makeRequest(`/api/notifications/${item.id}/read`, { method: 'PUT' });
        queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
      } catch {
        // Non-fatal
      }

      // Deep-link based on source type
      if (item.source === 'task' && item.source_id) {
        // Try to find a project context from the notification's organization
        // Fall back to my-tasks if no project context available
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
            <span className="absolute -top-0.5 -right-0.5 flex items-center justify-center min-w-[16px] h-4 rounded-full bg-blue-500 px-1 text-xs font-semibold text-white">
              {unreadCount > 99 ? '99+' : unreadCount}
            </span>
          )}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent className="w-80" align="end">
        <DropdownMenuLabel className="font-normal">
          <div className="flex items-center justify-between">
            <div>
              <p className="text-sm font-semibold">Notifications</p>
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

        {/* Tab bar */}
        <div className="flex border-b border-border px-1">
          {([
            { key: 'all' as const, label: 'All', count: unreadCount },
            { key: 'inbox' as const, label: 'Inbox', count: unreadInbox.length },
            { key: 'activity' as const, label: 'Activity', count: unreadActivityCount },
          ]).map((tab) => (
            <button
              key={tab.key}
              onClick={(e) => { e.preventDefault(); e.stopPropagation(); setActiveTab(tab.key); }}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium border-b-2 transition-colors ${
                activeTab === tab.key
                  ? 'border-primary text-foreground'
                  : 'border-transparent text-muted-foreground hover:text-foreground'
              }`}
            >
              {tab.label}
              {tab.count > 0 && (
                <span className="min-w-[16px] h-4 rounded-full bg-blue-500/15 text-blue-600 dark:text-blue-400 px-1 text-xs font-semibold flex items-center justify-center">
                  {tab.count > 99 ? '99+' : tab.count}
                </span>
              )}
            </button>
          ))}
        </div>

        <div className="max-h-80 overflow-y-auto">
          {isLoading ? (
            <div className="p-4 text-center text-sm text-muted-foreground">Loading...</div>
          ) : (() => {
            const showInbox = activeTab === 'all' || activeTab === 'inbox';
            const showActivity = activeTab === 'all' || activeTab === 'activity';
            const inboxItems = showInbox ? unreadInbox : [];
            const activityItems = showActivity ? visibleNotifications : [];
            const hasItems = inboxItems.length > 0 || activityItems.length > 0;

            if (!hasItems) {
              return (
                <div className="p-4 text-center text-sm text-muted-foreground">
                  {dismissedAt ? 'All caught up' : 'No recent activity'}
                </div>
              );
            }

            return (
              <>
                {inboxItems.map((item) => (
                  <InboxNotificationItem
                    key={`inbox-${item.id}`}
                    item={item}
                    onClick={handleInboxClick}
                    onDismiss={handleDismiss}
                  />
                ))}
                {activityItems.map((item) => (
                  <ActivityNotificationItem
                    key={item.id}
                    item={item}
                    onClick={handleActivityClick}
                    isRead={readActivityIds.has(item.id)}
                  />
                ))}
              </>
            );
          })()}
        </div>

        {/* View All link */}
        <DropdownMenuSeparator />
        <div className="p-1">
          <button
            onClick={() => navigate('/notifications')}
            className="w-full flex items-center justify-center gap-1.5 px-3 py-1.5 text-xs font-medium text-muted-foreground hover:text-foreground transition-colors rounded-sm hover:bg-accent"
          >
            View All Notifications
            <ArrowRight className="h-3 w-3" />
          </button>
        </div>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
