import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { CheckCheck, Search } from 'lucide-react';
import { makeRequest } from '@/lib/api';
import { notificationKeys } from '@/lib/query-keys';
import type { ActivityItem, InboxNotification } from '@/components/notifications/types';
import { getProjectId } from '@/components/notifications/utils';
import { ActivityNotificationItem } from '@/components/notifications/ActivityNotificationItem';
import { InboxNotificationItem } from '@/components/notifications/InboxNotificationItem';

const READ_ACTIVITY_IDS_KEY = 'orcha:read-activity-ids';

function loadReadActivityIds(): Set<string> {
  try {
    const raw = localStorage.getItem(READ_ACTIVITY_IDS_KEY);
    return raw ? new Set(JSON.parse(raw) as string[]) : new Set();
  } catch {
    return new Set();
  }
}

function persistReadActivityIds(ids: Set<string>) {
  try {
    localStorage.setItem(READ_ACTIVITY_IDS_KEY, JSON.stringify([...ids]));
  } catch { /* non-fatal */ }
}

type NotificationTab = 'all' | 'inbox' | 'activity';
type ReadFilter = 'all' | 'unread' | 'read';

export function NotificationsPage() {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<NotificationTab>('all');
  const [readFilter, setReadFilter] = useState<ReadFilter>('all');
  const [searchQuery, setSearchQuery] = useState('');
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
      const res = await makeRequest('/api/notifications?limit=100');
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
    staleTime: 10000,
  });

  const { data: inboxNotifications = [], isLoading: inboxLoading } = useQuery({
    queryKey: notificationKeys.inbox(),
    queryFn: async (): Promise<InboxNotification[]> => {
      const res = await makeRequest('/api/notifications/inbox?limit=100');
      if (!res.ok) return [];
      const json = await res.json();
      return json.data || [];
    },
    staleTime: 10000,
  });

  const isLoading = activityLoading || inboxLoading;
  const unreadInbox = inboxNotifications.filter((n) => !n.read_at);
  const visibleActivity = dismissedAt
    ? activityNotifications.filter((n) => new Date(n.timestamp) > new Date(dismissedAt))
    : activityNotifications;
  const unreadActivityCount = visibleActivity.filter((n) => !readActivityIds.has(n.id)).length;
  const unreadCount = unreadActivityCount + unreadInbox.length;

  const persistDismissedAt = useCallback((ts: string) => {
    setDismissedAt(ts);
    try { localStorage.setItem('orcha:activity-dismissed-at', ts); } catch { /* non-fatal */ }
  }, []);

  const handleMarkAllRead = useCallback(async () => {
    persistDismissedAt(new Date().toISOString());
    try {
      await makeRequest('/api/notifications/mark-all-read', { method: 'PUT' });
      queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
    } catch { /* Non-fatal */ }
  }, [queryClient, persistDismissedAt]);

  const handleActivityClick = useCallback(
    (item: ActivityItem) => {
      setReadActivityIds((prev) => {
        const next = new Set(prev);
        next.add(item.id);
        persistReadActivityIds(next);
        return next;
      });
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
      } catch { /* Non-fatal */ }
    },
    [queryClient],
  );

  const handleInboxClick = useCallback(
    async (item: InboxNotification) => {
      try {
        await makeRequest(`/api/notifications/${item.id}/read`, { method: 'PUT' });
        queryClient.invalidateQueries({ queryKey: notificationKeys.inbox() });
      } catch { /* Non-fatal */ }
      if (item.source === 'task' && item.source_id) {
        navigate('/my-tasks');
      } else if (item.source === 'workflow' && item.source_id) {
        navigate('/workflows');
      }
    },
    [navigate, queryClient],
  );

  // Filter items based on tab, read status, and search
  const getFilteredItems = () => {
    const showInbox = activeTab === 'all' || activeTab === 'inbox';
    const showActivity = activeTab === 'all' || activeTab === 'activity';
    const query = searchQuery.toLowerCase().trim();

    let inboxItems = showInbox ? inboxNotifications : [];
    let activityItems = showActivity ? visibleActivity : [];

    // Read filter
    if (readFilter === 'unread') {
      inboxItems = inboxItems.filter((n) => !n.read_at);
      activityItems = activityItems.filter((n) => !readActivityIds.has(n.id));
    } else if (readFilter === 'read') {
      inboxItems = inboxItems.filter((n) => !!n.read_at);
      activityItems = activityItems.filter((n) => readActivityIds.has(n.id));
    }

    // Search filter
    if (query) {
      inboxItems = inboxItems.filter((n) =>
        n.message.toLowerCase().includes(query) ||
        n.source?.toLowerCase().includes(query)
      );
      activityItems = activityItems.filter((n) =>
        n.action.toLowerCase().includes(query) ||
        n.actor_name?.toLowerCase().includes(query) ||
        n.metadata?.toLowerCase().includes(query)
      );
    }

    return { inboxItems, activityItems };
  };

  const { inboxItems, activityItems } = getFilteredItems();
  const totalFiltered = inboxItems.length + activityItems.length;

  const TABS: { key: NotificationTab; label: string; count: number }[] = [
    { key: 'all', label: 'All', count: unreadCount },
    { key: 'inbox', label: 'Inbox', count: unreadInbox.length },
    { key: 'activity', label: 'Activity', count: unreadActivityCount },
  ];

  return (
    <div className="max-w-4xl mx-auto p-6">
      <div className="flex items-center justify-between mb-6">
        <h1 className="text-2xl font-bold">Notifications</h1>
        {unreadCount > 0 && (
          <Button
            variant="outline"
            size="sm"
            className="gap-2"
            onClick={handleMarkAllRead}
          >
            <CheckCheck className="h-4 w-4" />
            Mark all read
          </Button>
        )}
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-4 border-b border-border mb-4">
        {TABS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => setActiveTab(tab.key)}
            className={`flex items-center gap-1.5 px-1 pb-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.key
                ? 'border-primary text-foreground'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            {tab.label}
            {tab.count > 0 && (
              <span className="min-w-[18px] h-5 rounded-full bg-blue-500/15 text-blue-600 dark:text-blue-400 px-1.5 text-xs font-bold flex items-center justify-center">
                {tab.count > 99 ? '99+' : tab.count}
              </span>
            )}
          </button>
        ))}
      </div>

      {/* Search and Filter */}
      <div className="flex items-center gap-3 mb-4">
        <div className="relative flex-1">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Search notifications..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>
        <div className="flex items-center gap-1 border rounded-md p-0.5">
          {(['all', 'unread', 'read'] as const).map((f) => (
            <button
              key={f}
              onClick={() => setReadFilter(f)}
              className={`px-3 py-1 text-xs font-medium rounded transition-colors ${
                readFilter === f
                  ? 'bg-accent text-accent-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {/* Notifications List */}
      <div className="border rounded-lg divide-y divide-border bg-card">
        {isLoading ? (
          <div className="p-8 text-center text-sm text-muted-foreground">Loading notifications...</div>
        ) : totalFiltered === 0 ? (
          <div className="p-8 text-center text-sm text-muted-foreground">
            {searchQuery ? 'No notifications match your search' : 'All caught up'}
          </div>
        ) : (
          <>
            {inboxItems.map((item) => (
              <div key={`inbox-${item.id}`} className="px-2">
                <InboxNotificationItem
                  item={item}
                  onClick={handleInboxClick}
                  onDismiss={handleDismiss}
                />
              </div>
            ))}
            {activityItems.map((item) => (
              <div key={item.id} className="px-2">
                <ActivityNotificationItem
                  item={item}
                  onClick={handleActivityClick}
                  isRead={readActivityIds.has(item.id)}
                />
              </div>
            ))}
          </>
        )}
      </div>

      {totalFiltered > 0 && (
        <p className="text-xs text-muted-foreground mt-3 text-center">
          Showing {totalFiltered} notification{totalFiltered !== 1 ? 's' : ''}
        </p>
      )}
    </div>
  );
}
