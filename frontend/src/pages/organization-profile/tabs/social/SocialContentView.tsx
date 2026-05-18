import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query';
import {
  CalendarDays,
  CalendarRange,
  ChevronLeft,
  ChevronRight,
  ExternalLink,
  FileText,
  Globe,
  List,
  Plus,
  Trash2,
  X,
} from 'lucide-react';
import { useMemo, useState } from 'react';

import { PostDetailModal } from '@/components/social/PostDetailModal';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { socialApi, type SocialPostRecord } from '@/lib/api';
import { formatCompactNumber } from '@/lib/formatters';
import { socialKeys } from '@/lib/query-keys';

import {
  PLATFORM_BG,
  PLATFORM_COLORS,
  PLATFORM_ICONS,
  STATUS_COLORS,
} from '../../constants';
import {
  buildMonthGrid,
  buildWeekDays,
  DAY_NAMES,
  formatDate,
  isSameDay,
  MONTH_NAMES,
} from '../../helpers';

export function SocialContentView({
  projectEntries,
  orgId,
}: {
  projectEntries: { id: string; name: string }[];
  orgId?: string;
}) {
  const [calView, setCalView] = useState<'list' | 'week' | 'month'>('list');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [projectFilter, setProjectFilter] = useState<string>('all');
  const [showClientContent, setShowClientContent] = useState(false);
  const [clientProjectFilter, setClientProjectFilter] = useState<string[]>([]); // empty = all clients
  const [showCreate, setShowCreate] = useState(false);
  const [newCaption, setNewCaption] = useState('');
  const [newPlatforms, setNewPlatforms] = useState<string[]>([]);
  const [newStatus, setNewStatus] = useState('draft');
  const [newProjectId, setNewProjectId] = useState(projectEntries[0]?.id || '');
  // 'org' = post as org brand; 'project' = post for a specific project/client
  const [postingAs, setPostingAs] = useState<'org' | 'project'>(
    orgId ? 'org' : 'project'
  );
  const [newScheduled, setNewScheduled] = useState('');
  const [calDate, setCalDate] = useState(new Date()); // anchor for week/month navigation
  const [selectedDay, setSelectedDay] = useState<Date | null>(null);
  const [selectedPostId, setSelectedPostId] = useState<string | null>(null);

  // Org-owned posts (fetched as a single query when orgId is present)
  const orgPostsQuery = useQuery({
    queryKey: ['social', 'posts', 'org', orgId],
    queryFn: () =>
      socialApi.listPostsFiltered({ organizationId: orgId!, limit: 200 }),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  // Which project entries to query: in org mode, only when showClientContent is on (and filtered)
  const activeProjectEntries = useMemo(() => {
    if (orgId) {
      if (!showClientContent) return [];
      return clientProjectFilter.length > 0
        ? projectEntries.filter((e) => clientProjectFilter.includes(e.id))
        : projectEntries;
    }
    return projectEntries;
  }, [orgId, showClientContent, clientProjectFilter, projectEntries]);

  // Per-project queries
  const projectPostQueries = useQueries({
    queries: activeProjectEntries.map((e) => ({
      queryKey: socialKeys.posts(e.id),
      queryFn: () =>
        socialApi.listPostsFiltered({ projectId: e.id, limit: 100 }),
      staleTime: 60_000,
    })),
  });

  const allPosts = useMemo(() => {
    const list: (SocialPostRecord & {
      _project: string;
      _projectId: string;
      _isOrgOwned: boolean;
    })[] = [];

    // Org-owned posts
    orgPostsQuery.data?.forEach((p) =>
      list.push({ ...p, _project: 'Org', _projectId: '', _isOrgOwned: true })
    );

    // Project/client posts
    projectPostQueries.forEach((q, i) => {
      q.data?.forEach((p) =>
        list.push({
          ...p,
          _project: activeProjectEntries[i]?.name ?? '',
          _projectId: activeProjectEntries[i]?.id ?? '',
          _isOrgOwned: false,
        })
      );
    });

    return list.sort((a, b) => {
      if (a.scheduled_for && b.scheduled_for)
        return (
          new Date(a.scheduled_for).getTime() -
          new Date(b.scheduled_for).getTime()
        );
      return (
        new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime()
      );
    });
  }, [orgPostsQuery.data, projectPostQueries, activeProjectEntries]);

  const filtered = useMemo(() => {
    return allPosts.filter((p) => {
      if (statusFilter !== 'all' && p.status !== statusFilter) return false;
      if (projectFilter !== 'all' && p._projectId !== projectFilter)
        return false;
      return true;
    });
  }, [allPosts, statusFilter, projectFilter]);

  const counts = useMemo(() => {
    const c: Record<string, number> = { all: allPosts.length };
    allPosts.forEach((p) => {
      c[p.status] = (c[p.status] || 0) + 1;
    });
    return c;
  }, [allPosts]);

  const queryClient = useQueryClient();
  const createMut = useMutation({
    mutationFn: () =>
      socialApi.createPost({
        ...(postingAs === 'org' && orgId
          ? { organization_id: orgId }
          : { project_id: newProjectId }),
        caption: newCaption,
        platforms: newPlatforms,
        status: newStatus,
        scheduled_for: newScheduled || undefined,
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: socialKeys.postsAll() });
      setShowCreate(false);
      setNewCaption('');
      setNewPlatforms([]);
      setNewScheduled('');
    },
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => socialApi.deletePost(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: socialKeys.postsAll() }),
  });

  // Helpers shared across views
  const STATUS_TABS = [
    'all',
    'draft',
    'pending_review',
    'scheduled',
    'published',
    'failed',
  ];
  const parsePlatforms = (p: string): string[] => {
    try {
      return JSON.parse(p);
    } catch {
      return [p].filter(Boolean);
    }
  };

  // Calendar navigation
  const monthYear = `${MONTH_NAMES[calDate.getMonth()]} ${calDate.getFullYear()}`;
  const weekDays = buildWeekDays(calDate);
  const monthGrid = buildMonthGrid(calDate.getFullYear(), calDate.getMonth());

  // Posts indexed by date string "YYYY-MM-DD" using scheduled_for or published_at
  const postsByDate = useMemo(() => {
    const map: Record<string, typeof allPosts> = {};
    allPosts.forEach((p) => {
      const d = p.scheduled_for || p.published_at;
      if (!d) return;
      const key = d.slice(0, 10);
      if (!map[key]) map[key] = [];
      map[key].push(p);
    });
    return map;
  }, [allPosts]);

  // Posts with no date — shown in a drafts strip below calendar views
  const unscheduledPosts = useMemo(
    () => filtered.filter((p) => !p.scheduled_for && !p.published_at),
    [filtered]
  );

  const dateKey = (d: Date) =>
    `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
  const today = new Date();

  // Post pill used in calendar cells
  const PostPill = ({
    post,
    compact = false,
    onClick,
  }: {
    post: (typeof allPosts)[0];
    compact?: boolean;
    onClick?: (e: React.MouseEvent) => void;
  }) => {
    const platforms = parsePlatforms(post.platforms);
    const Icon = PLATFORM_ICONS[platforms[0]] || Globe;
    const statusColor =
      post.status === 'published'
        ? 'bg-green-500/15 text-green-700 dark:text-green-400 border-green-300/50'
        : post.status === 'scheduled'
          ? 'bg-purple-500/15 text-purple-700 dark:text-purple-400 border-purple-300/50'
          : post.status === 'failed'
            ? 'bg-destructive/15 text-destructive border-destructive/30'
            : 'bg-muted text-muted-foreground border-border/50';
    return (
      <div
        onClick={onClick}
        className={`flex items-center gap-1 px-1.5 py-0.5 rounded border text-xs leading-tight truncate ${statusColor} ${onClick ? 'cursor-pointer hover:opacity-80' : ''}`}
      >
        <Icon
          className={`h-2.5 w-2.5 shrink-0 ${PLATFORM_COLORS[platforms[0]] || ''}`}
        />
        {!compact && (
          <span className="truncate">
            {post.caption?.slice(0, 28) || '(no caption)'}
          </span>
        )}
        {compact && <span className="truncate">{platforms[0]}</span>}
      </div>
    );
  };

  // Day detail panel (shown when a day is selected in month view)
  const DayDetail = ({ day }: { day: Date }) => {
    const posts = postsByDate[dateKey(day)] || [];
    return (
      <Card className="bg-card/80 backdrop-blur-sm border-border/50 mt-4">
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-sm font-medium">
              {day.toLocaleDateString('en-US', {
                weekday: 'long',
                month: 'long',
                day: 'numeric',
              })}
            </CardTitle>
            <button
              onClick={() => setSelectedDay(null)}
              className="text-muted-foreground hover:text-foreground"
            >
              <X className="h-4 w-4" />
            </button>
          </div>
        </CardHeader>
        <CardContent>
          {posts.length === 0 ? (
            <p className="text-xs text-muted-foreground py-3 text-center">
              No posts this day
            </p>
          ) : (
            <div className="space-y-2">
              {posts.map((post) => {
                const platforms = parsePlatforms(post.platforms);
                return (
                  <div
                    key={post.id}
                    className="flex items-start gap-2 p-2 rounded-lg border border-border/40 hover:bg-muted/30 cursor-pointer hover:border-primary/30"
                    onPointerUp={(e) => {
                      if ((e.target as HTMLElement).closest('button')) return;
                      setSelectedPostId(post.id);
                    }}
                  >
                    <div className="flex gap-0.5 mt-0.5">
                      {platforms.slice(0, 3).map((p) => {
                        const Icon = PLATFORM_ICONS[p] || Globe;
                        return (
                          <Icon
                            key={p}
                            className={`h-3.5 w-3.5 ${PLATFORM_COLORS[p] || ''}`}
                          />
                        );
                      })}
                    </div>
                    <div className="min-w-0 flex-1">
                      <p className="text-xs line-clamp-2">
                        {post.caption || (
                          <span className="italic text-muted-foreground">
                            No caption
                          </span>
                        )}
                      </p>
                      <div className="flex items-center gap-2 mt-1">
                        <span
                          className={`text-xs px-1.5 py-0.5 rounded border ${STATUS_COLORS[post.status] || ''}`}
                        >
                          {post.status.replace('_', ' ')}
                        </span>
                        {(post.scheduled_for || post.published_at) && (
                          <span className="text-xs text-muted-foreground">
                            {new Date(
                              post.scheduled_for || post.published_at!
                            ).toLocaleTimeString('en-US', {
                              hour: 'numeric',
                              minute: '2-digit',
                            })}
                          </span>
                        )}
                        <span className="text-xs text-muted-foreground">
                          {post._project}
                        </span>
                      </div>
                    </div>
                    <button
                      onClick={() => {
                        if (confirm('Delete?')) deleteMut.mutate(post.id);
                      }}
                      className="p-1 rounded hover:bg-destructive/10 shrink-0"
                    >
                      <Trash2 className="h-3 w-3 text-muted-foreground hover:text-destructive" />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    );
  };

  // Unscheduled drafts strip — posts without a date, shown below calendar views
  const UnscheduledDraftsStrip = ({
    posts,
    onSelect,
  }: {
    posts: typeof allPosts;
    onSelect: (id: string) => void;
  }) => (
    <Card className="bg-card/80 backdrop-blur-sm border-border/50 mt-3">
      <CardHeader className="pb-2 pt-3">
        <CardTitle className="text-xs font-medium text-muted-foreground flex items-center gap-1.5">
          <FileText className="h-3.5 w-3.5" />
          Unscheduled drafts ({posts.length})
        </CardTitle>
      </CardHeader>
      <CardContent className="pt-0 pb-3">
        <div className="flex flex-wrap gap-1.5">
          {posts.map((post) => {
            const platforms = parsePlatforms(post.platforms);
            const Icon = PLATFORM_ICONS[platforms[0]] || Globe;
            return (
              <button
                key={post.id}
                onClick={() => onSelect(post.id)}
                className="flex items-center gap-1.5 px-2 py-1 rounded border border-border/50 bg-muted/40 hover:bg-muted hover:border-primary/30 text-xs text-muted-foreground transition-colors max-w-[200px]"
              >
                <Icon
                  className={`h-3 w-3 shrink-0 ${PLATFORM_COLORS[platforms[0]] || ''}`}
                />
                <span className="truncate">
                  {post.caption?.slice(0, 40) || '(no caption)'}
                </span>
              </button>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );

  // Shared create form
  const CreateForm = () => (
    <Card className="bg-card/80 backdrop-blur-sm border-border/50">
      <CardHeader className="pb-3">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Plus className="h-4 w-4" /> Create Post
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div>
          <label className="text-xs font-medium mb-1 block">Caption</label>
          <textarea
            value={newCaption}
            onChange={(e) => setNewCaption(e.target.value)}
            placeholder="Write your caption..."
            rows={3}
            className="w-full text-sm border border-border rounded-md p-2 bg-background resize-none focus:outline-none focus:ring-1 focus:ring-ring"
          />
        </div>
        <div className="grid grid-cols-2 gap-3">
          <div>
            <label className="text-xs font-medium mb-1 block">Platforms</label>
            <div className="flex flex-wrap gap-1.5">
              {[
                'instagram',
                'linkedin',
                'twitter',
                'facebook',
                'youtube',
                'tiktok',
              ].map((p) => {
                const Icon = PLATFORM_ICONS[p] || Globe;
                const selected = newPlatforms.includes(p);
                return (
                  <button
                    key={p}
                    onClick={() =>
                      setNewPlatforms((prev) =>
                        selected ? prev.filter((x) => x !== p) : [...prev, p]
                      )
                    }
                    className={`flex items-center gap-1 px-2 py-1 rounded border text-xs transition-colors ${selected ? 'border-primary bg-primary/10 text-primary' : 'border-border text-muted-foreground hover:border-border/80'}`}
                  >
                    <Icon
                      className={`h-3 w-3 ${selected ? '' : PLATFORM_COLORS[p]}`}
                    />
                    <span className="capitalize">{p}</span>
                  </button>
                );
              })}
            </div>
          </div>
          <div className="space-y-2">
            <div>
              <label className="text-xs font-medium mb-1 block">Status</label>
              <select
                value={newStatus}
                onChange={(e) => setNewStatus(e.target.value)}
                className="w-full text-xs border border-border rounded-md px-2 py-1.5 bg-background"
              >
                <option value="draft">Draft</option>
                <option value="scheduled">Scheduled</option>
                <option value="pending_review">Pending Review</option>
              </select>
            </div>
            {orgId && (
              <div>
                <label className="text-xs font-medium mb-1 block">
                  Posting as
                </label>
                <div className="flex gap-1">
                  <button
                    onClick={() => setPostingAs('org')}
                    className={`flex-1 text-xs px-2 py-1.5 rounded-md border transition-colors ${postingAs === 'org' ? 'bg-primary/10 border-primary/30 text-primary' : 'border-border text-muted-foreground hover:text-foreground'}`}
                  >
                    Org brand
                  </button>
                  <button
                    onClick={() => setPostingAs('project')}
                    className={`flex-1 text-xs px-2 py-1.5 rounded-md border transition-colors ${postingAs === 'project' ? 'bg-primary/10 border-primary/30 text-primary' : 'border-border text-muted-foreground hover:text-foreground'}`}
                  >
                    Client project
                  </button>
                </div>
              </div>
            )}
            {(postingAs === 'project' || !orgId) &&
              projectEntries.length > 0 && (
                <div>
                  <label className="text-xs font-medium mb-1 block">
                    Project
                  </label>
                  <select
                    value={newProjectId}
                    onChange={(e) => setNewProjectId(e.target.value)}
                    className="w-full text-xs border border-border rounded-md px-2 py-1.5 bg-background"
                  >
                    {projectEntries.map((e) => (
                      <option key={e.id} value={e.id}>
                        {e.name}
                      </option>
                    ))}
                  </select>
                </div>
              )}
          </div>
        </div>
        <div>
          <label className="text-xs font-medium mb-1 block">
            Schedule for{' '}
            {newStatus !== 'scheduled' && (
              <span className="text-muted-foreground">(optional)</span>
            )}
          </label>
          <input
            type="datetime-local"
            value={newScheduled}
            onChange={(e) => setNewScheduled(e.target.value)}
            className="text-xs border border-border rounded-md px-2 py-1.5 bg-background"
          />
        </div>
        <div className="flex gap-2 pt-1">
          <Button
            size="sm"
            onClick={() => createMut.mutate()}
            disabled={
              createMut.isPending || !newCaption || newPlatforms.length === 0
            }
            className="text-xs gap-1 h-8"
          >
            {createMut.isPending ? 'Creating...' : 'Create Post'}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={() => setShowCreate(false)}
            className="text-xs h-8"
          >
            Cancel
          </Button>
        </div>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-4 relative">
      <PostDetailModal
        postId={selectedPostId}
        open={!!selectedPostId}
        onClose={() => setSelectedPostId(null)}
        onDeleted={() => {
          queryClient.invalidateQueries({
            queryKey: ['social', 'posts', 'org', orgId],
          });
          projectEntries.forEach((e) =>
            queryClient.invalidateQueries({ queryKey: socialKeys.posts(e.id) })
          );
        }}
      />
      {/* Top toolbar */}
      <div className="flex items-center justify-between gap-3 flex-wrap">
        {/* View toggle */}
        <div className="flex gap-0.5 p-0.5 bg-muted/60 rounded-lg border border-border/40">
          {(
            [
              ['list', 'List', List],
              ['week', 'Week', CalendarDays],
              ['month', 'Month', CalendarRange],
            ] as const
          ).map(([v, label, Icon]) => (
            <button
              key={v}
              onClick={() => {
                setCalView(v);
                setSelectedDay(null);
              }}
              className={`flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-all ${calView === v ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}
            >
              <Icon className="h-3.5 w-3.5" />
              {label}
            </button>
          ))}
        </div>

        <div className="flex items-center gap-2">
          {/* Calendar navigation (week/month only) */}
          {calView !== 'list' && (
            <div className="flex items-center gap-1">
              <button
                onClick={() => {
                  const d = new Date(calDate);
                  if (calView === 'month') d.setMonth(d.getMonth() - 1);
                  else d.setDate(d.getDate() - 7);
                  setCalDate(d);
                  setSelectedDay(null);
                }}
                className="p-1.5 rounded hover:bg-muted transition-colors"
              >
                <ChevronLeft className="h-4 w-4" />
              </button>
              <span className="text-sm font-medium min-w-[140px] text-center">
                {calView === 'month'
                  ? monthYear
                  : `${weekDays[0].toLocaleDateString('en-US', { month: 'short', day: 'numeric' })} – ${weekDays[6].toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}`}
              </span>
              <button
                onClick={() => {
                  const d = new Date(calDate);
                  if (calView === 'month') d.setMonth(d.getMonth() + 1);
                  else d.setDate(d.getDate() + 7);
                  setCalDate(d);
                  setSelectedDay(null);
                }}
                className="p-1.5 rounded hover:bg-muted transition-colors"
              >
                <ChevronRight className="h-4 w-4" />
              </button>
              <button
                onClick={() => {
                  setCalDate(new Date());
                  setSelectedDay(null);
                }}
                className="text-xs px-2 py-1 rounded border border-border hover:bg-muted transition-colors"
              >
                Today
              </button>
            </div>
          )}

          {/* List-only filters */}
          {calView === 'list' && (
            <>
              {/* Org view: show client content toggle + per-client filter */}
              {orgId && projectEntries.length > 0 && (
                <div className="flex items-center gap-2 flex-wrap">
                  <button
                    onClick={() => {
                      setShowClientContent((v) => !v);
                      setClientProjectFilter([]);
                    }}
                    className={`flex items-center gap-1.5 text-xs px-2.5 py-1.5 rounded-md border transition-colors ${
                      showClientContent
                        ? 'bg-primary/10 border-primary/30 text-primary'
                        : 'border-border text-muted-foreground hover:text-foreground'
                    }`}
                  >
                    {showClientContent
                      ? 'Hide client content'
                      : 'Show client content'}
                  </button>
                  {showClientContent && (
                    <div className="flex items-center gap-1 flex-wrap">
                      {projectEntries.map((e) => {
                        const active =
                          clientProjectFilter.length === 0 ||
                          clientProjectFilter.includes(e.id);
                        return (
                          <button
                            key={e.id}
                            onClick={() =>
                              setClientProjectFilter((prev) => {
                                if (prev.length === 0) {
                                  // All were active → deselect all except this one
                                  return projectEntries
                                    .filter((p) => p.id !== e.id)
                                    .map((p) => p.id);
                                }
                                const next = prev.includes(e.id)
                                  ? prev.filter((id) => id !== e.id)
                                  : [...prev, e.id];
                                // If all selected, reset to empty (= all)
                                return next.length === projectEntries.length
                                  ? []
                                  : next;
                              })
                            }
                            className={`text-xs px-2 py-1 rounded-full border transition-colors ${
                              active
                                ? 'bg-primary/10 border-primary/30 text-primary'
                                : 'border-border text-muted-foreground opacity-50'
                            }`}
                          >
                            {e.name}
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>
              )}
              {!orgId && projectEntries.length > 1 && (
                <select
                  value={projectFilter}
                  onChange={(e) => setProjectFilter(e.target.value)}
                  className="text-xs border border-border rounded-md px-2 py-1.5 bg-background"
                >
                  <option value="all">All projects</option>
                  {projectEntries.map((e) => (
                    <option key={e.id} value={e.id}>
                      {e.name}
                    </option>
                  ))}
                </select>
              )}
            </>
          )}

          <Button
            size="sm"
            className="gap-1.5 text-xs h-8"
            onClick={() => setShowCreate((v) => !v)}
          >
            <Plus className="h-3.5 w-3.5" /> New Post
          </Button>
        </div>
      </div>

      {/* Create form */}
      {showCreate && <CreateForm />}

      {/* -- LIST VIEW -- */}
      {calView === 'list' && (
        <>
          <div className="flex gap-1 p-1 bg-muted/50 rounded-lg flex-wrap">
            {STATUS_TABS.map((s) => (
              <button
                key={s}
                onClick={() => setStatusFilter(s)}
                className={`px-3 py-1 text-xs font-medium rounded-md transition-all capitalize ${statusFilter === s ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground hover:text-foreground'}`}
              >
                {s === 'all' ? 'All' : s.replace('_', ' ')}
                {counts[s] != null && counts[s] > 0 && (
                  <span className="ml-1.5 text-xs text-muted-foreground">
                    {counts[s]}
                  </span>
                )}
              </button>
            ))}
          </div>

          {filtered.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <FileText className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p>
                {statusFilter === 'all'
                  ? 'No posts yet'
                  : `No ${statusFilter.replace('_', ' ')} posts`}
              </p>
            </div>
          ) : (
            <div className="space-y-2">
              {filtered.map((post) => {
                const platforms = parsePlatforms(post.platforms);
                const hashtags: string[] = (() => {
                  try {
                    return post.hashtags ? JSON.parse(post.hashtags) : [];
                  } catch {
                    return [];
                  }
                })();
                return (
                  <Card
                    key={post.id}
                    className="bg-card/80 backdrop-blur-sm border-border/50 hover:border-primary/30 transition-colors cursor-pointer"
                    onPointerUp={(e) => {
                      if ((e.target as HTMLElement).closest('a,button')) return;
                      setSelectedPostId(post.id);
                    }}
                  >
                    <CardContent className="pt-4 pb-3">
                      <div className="flex items-start gap-3">
                        <div className="flex gap-1 mt-0.5 shrink-0">
                          {platforms.map((p) => {
                            const Icon = PLATFORM_ICONS[p] || Globe;
                            return (
                              <div
                                key={p}
                                className={`h-7 w-7 rounded-full flex items-center justify-center ${PLATFORM_BG[p] || 'bg-muted'}`}
                              >
                                <Icon
                                  className={`h-3.5 w-3.5 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`}
                                />
                              </div>
                            );
                          })}
                        </div>
                        <div className="min-w-0 flex-1">
                          <p className="text-sm line-clamp-2">
                            {post.caption || (
                              <span className="text-muted-foreground italic">
                                No caption
                              </span>
                            )}
                          </p>
                          {hashtags.length > 0 && (
                            <p className="text-xs text-blue-500 mt-1 line-clamp-1">
                              {hashtags.map((h) => `#${h}`).join(' ')}
                            </p>
                          )}
                          <div className="flex items-center gap-2 mt-2 flex-wrap">
                            <span
                              className={`text-xs px-1.5 py-0.5 rounded border ${STATUS_COLORS[post.status] || ''}`}
                            >
                              {post.status.replace('_', ' ')}
                            </span>
                            {post.scheduled_for && (
                              <span className="text-xs text-muted-foreground flex items-center gap-1">
                                <CalendarDays className="h-3 w-3" />
                                {formatDate(post.scheduled_for)}
                              </span>
                            )}
                            {post.published_at && (
                              <span className="text-xs text-muted-foreground">
                                Published {formatDate(post.published_at)}
                              </span>
                            )}
                            <span className="text-xs text-muted-foreground">
                              {post._project}
                            </span>
                          </div>
                          {post.status === 'published' &&
                            (post.impressions > 0 || post.likes > 0) && (
                              <div className="flex gap-3 mt-2 text-xs text-muted-foreground">
                                {post.impressions > 0 && (
                                  <span>
                                    &#128065;{' '}
                                    {formatCompactNumber(post.impressions)}
                                  </span>
                                )}
                                {post.reach > 0 && (
                                  <span>
                                    &#128225; {formatCompactNumber(post.reach)}
                                  </span>
                                )}
                                {post.likes > 0 && (
                                  <span>&#9829; {post.likes}</span>
                                )}
                                {post.comments > 0 && (
                                  <span>&#128172; {post.comments}</span>
                                )}
                                {post.engagement_rate > 0 && (
                                  <span>
                                    {(post.engagement_rate * 100).toFixed(1)}%
                                    eng
                                  </span>
                                )}
                              </div>
                            )}
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          {post.platform_url && (
                            <a
                              href={post.platform_url}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="p-1.5 rounded hover:bg-muted"
                            >
                              <ExternalLink className="h-3.5 w-3.5 text-muted-foreground" />
                            </a>
                          )}
                          <button
                            onClick={() => {
                              if (confirm('Delete this post?'))
                                deleteMut.mutate(post.id);
                            }}
                            className="p-1.5 rounded hover:bg-destructive/10"
                          >
                            <Trash2 className="h-3.5 w-3.5 text-muted-foreground hover:text-destructive" />
                          </button>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </>
      )}

      {/* -- WEEK VIEW -- */}
      {calView === 'week' && (
        <>
          <Card className="bg-card/80 backdrop-blur-sm border-border/50 overflow-hidden">
            <div className="grid grid-cols-7 border-b border-border/50">
              {weekDays.map((day, i) => {
                const isToday = isSameDay(day, today);
                const dayPosts = postsByDate[dateKey(day)] || [];
                return (
                  <div
                    key={i}
                    className={`border-r border-border/40 last:border-r-0 min-h-[520px] flex flex-col ${isToday ? 'bg-primary/5' : ''}`}
                  >
                    {/* Day header */}
                    <div
                      className={`px-2 py-2 border-b border-border/40 ${isToday ? 'bg-primary/10' : 'bg-muted/20'}`}
                    >
                      <p className="text-xs font-medium text-muted-foreground uppercase">
                        {DAY_NAMES[day.getDay()]}
                      </p>
                      <p
                        className={`text-lg font-semibold leading-none mt-0.5 ${isToday ? 'text-primary' : ''}`}
                      >
                        {day.getDate()}
                      </p>
                      {dayPosts.length > 0 && (
                        <p className="text-[9px] text-muted-foreground mt-1">
                          {dayPosts.length} post
                          {dayPosts.length !== 1 ? 's' : ''}
                        </p>
                      )}
                    </div>
                    {/* Posts */}
                    <div className="p-1.5 space-y-1 flex-1">
                      {dayPosts.map((post) => {
                        const platforms = parsePlatforms(post.platforms);
                        const Icon = PLATFORM_ICONS[platforms[0]] || Globe;
                        const timeStr =
                          post.scheduled_for || post.published_at
                            ? new Date(
                                post.scheduled_for || post.published_at!
                              ).toLocaleTimeString('en-US', {
                                hour: 'numeric',
                                minute: '2-digit',
                              })
                            : null;
                        const statusColor =
                          post.status === 'published'
                            ? 'bg-green-500/10 border-green-300/40 text-green-700 dark:text-green-400'
                            : post.status === 'scheduled'
                              ? 'bg-purple-500/10 border-purple-300/40 text-purple-700 dark:text-purple-400'
                              : post.status === 'failed'
                                ? 'bg-destructive/10 border-destructive/30 text-destructive'
                                : 'bg-muted/60 border-border/40 text-muted-foreground';
                        return (
                          <div
                            key={post.id}
                            className={`rounded border p-1.5 cursor-pointer group relative ${statusColor}`}
                            onPointerUp={(e) => {
                              if ((e.target as HTMLElement).closest('button'))
                                return;
                              setSelectedPostId(post.id);
                            }}
                          >
                            <div className="flex items-center gap-1 mb-0.5">
                              <Icon
                                className={`h-2.5 w-2.5 shrink-0 ${PLATFORM_COLORS[platforms[0]] || ''}`}
                              />
                              {timeStr && (
                                <span className="text-[9px] font-mono opacity-70">
                                  {timeStr}
                                </span>
                              )}
                            </div>
                            <p className="text-xs leading-tight line-clamp-3">
                              {post.caption || '(no caption)'}
                            </p>
                            {platforms.length > 1 && (
                              <div className="flex gap-0.5 mt-1">
                                {platforms.slice(1).map((p) => {
                                  const I = PLATFORM_ICONS[p] || Globe;
                                  return (
                                    <I
                                      key={p}
                                      className={`h-2 w-2 ${PLATFORM_COLORS[p] || ''} opacity-60`}
                                    />
                                  );
                                })}
                              </div>
                            )}
                            <button
                              onClick={() => {
                                if (confirm('Delete?'))
                                  deleteMut.mutate(post.id);
                              }}
                              className="absolute top-1 right-1 opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-destructive/20 transition-opacity"
                            >
                              <Trash2 className="h-2.5 w-2.5 text-destructive" />
                            </button>
                          </div>
                        );
                      })}
                      {dayPosts.length === 0 && (
                        <button
                          onClick={() => {
                            setNewScheduled(`${dateKey(day)}T09:00`);
                            setNewStatus('scheduled');
                            setShowCreate(true);
                          }}
                          className="w-full h-8 rounded border border-dashed border-border/30 text-xs text-muted-foreground/40 hover:border-border/60 hover:text-muted-foreground transition-colors flex items-center justify-center gap-1"
                        >
                          <Plus className="h-2.5 w-2.5" /> Add
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
            {/* Week summary bar */}
            <div className="px-4 py-2 bg-muted/20 border-t border-border/40 flex items-center gap-4 text-xs text-muted-foreground">
              <span>
                {weekDays.reduce(
                  (s, d) => s + (postsByDate[dateKey(d)]?.length || 0),
                  0
                )}{' '}
                posts this week
              </span>
              <span>&middot;</span>
              <span>
                {weekDays.reduce(
                  (s, d) =>
                    s +
                    (postsByDate[dateKey(d)]?.filter(
                      (p) => p.status === 'scheduled'
                    ).length || 0),
                  0
                )}{' '}
                scheduled
              </span>
              <span>&middot;</span>
              <span>
                {weekDays.reduce(
                  (s, d) =>
                    s +
                    (postsByDate[dateKey(d)]?.filter(
                      (p) => p.status === 'published'
                    ).length || 0),
                  0
                )}{' '}
                published
              </span>
            </div>
          </Card>
          {unscheduledPosts.length > 0 && (
            <UnscheduledDraftsStrip
              posts={unscheduledPosts}
              onSelect={setSelectedPostId}
            />
          )}
        </>
      )}

      {/* -- MONTH VIEW -- */}
      {calView === 'month' && (
        <>
          <Card className="bg-card/80 backdrop-blur-sm border-border/50 overflow-hidden">
            {/* Day-of-week headers */}
            <div className="grid grid-cols-7 bg-muted/30 border-b border-border/40">
              {DAY_NAMES.map((d) => (
                <div
                  key={d}
                  className="py-2 text-center text-xs font-semibold text-muted-foreground uppercase tracking-wide"
                >
                  {d}
                </div>
              ))}
            </div>
            {/* Calendar grid */}
            <div className="grid grid-cols-7">
              {monthGrid.map((cell, idx) => {
                const key = dateKey(cell.date);
                const dayPosts = postsByDate[key] || [];
                const isToday = isSameDay(cell.date, today);
                const isSelected = selectedDay
                  ? isSameDay(cell.date, selectedDay)
                  : false;
                const MAX_VISIBLE = 3;
                return (
                  <div
                    key={idx}
                    onClick={() =>
                      setSelectedDay(isSelected ? null : cell.date)
                    }
                    className={[
                      'min-h-[100px] p-1.5 border-b border-r border-border/30 cursor-pointer transition-colors',
                      !cell.inMonth && 'opacity-40',
                      isToday && 'bg-primary/5',
                      isSelected &&
                        'ring-2 ring-inset ring-primary/40 bg-primary/5',
                      'hover:bg-muted/30',
                    ]
                      .filter(Boolean)
                      .join(' ')}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span
                        className={[
                          'text-xs font-medium w-6 h-6 flex items-center justify-center rounded-full leading-none',
                          isToday ? 'bg-primary text-primary-foreground' : '',
                          !cell.inMonth ? 'text-muted-foreground/50' : '',
                        ]
                          .filter(Boolean)
                          .join(' ')}
                      >
                        {cell.date.getDate()}
                      </span>
                      {dayPosts.length > 0 && (
                        <span className="text-[9px] text-muted-foreground">
                          {dayPosts.length}
                        </span>
                      )}
                    </div>
                    <div className="space-y-0.5">
                      {dayPosts.slice(0, MAX_VISIBLE).map((post) => (
                        <PostPill
                          key={post.id}
                          post={post}
                          onClick={(e) => {
                            e.stopPropagation();
                            setSelectedPostId(post.id);
                          }}
                        />
                      ))}
                      {dayPosts.length > MAX_VISIBLE && (
                        <div className="text-[9px] text-muted-foreground pl-1">
                          +{dayPosts.length - MAX_VISIBLE} more
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
            {/* Month legend */}
            <div className="px-4 py-2 bg-muted/20 border-t border-border/40 flex items-center gap-4 text-xs">
              {[
                ['bg-purple-500/15 border-purple-300/50', 'scheduled'],
                ['bg-green-500/15 border-green-300/50', 'published'],
                ['bg-destructive/15 border-destructive/30', 'failed'],
                ['bg-muted border-border/50', 'draft'],
              ].map(([cls, lbl]) => (
                <span key={lbl} className="flex items-center gap-1.5">
                  <span className={`w-2.5 h-2.5 rounded border ${cls}`} />
                  <span className="text-muted-foreground capitalize">
                    {lbl}
                  </span>
                </span>
              ))}
            </div>
          </Card>

          {/* Day detail panel */}
          {selectedDay && <DayDetail day={selectedDay} />}
          {unscheduledPosts.length > 0 && (
            <UnscheduledDraftsStrip
              posts={unscheduledPosts}
              onSelect={setSelectedPostId}
            />
          )}
        </>
      )}
    </div>
  );
}
