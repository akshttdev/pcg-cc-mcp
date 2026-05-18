import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  BrainCircuit,
  Calendar,
  CalendarDays,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  Circle,
  Clock,
  Facebook,
  Globe,
  Instagram,
  Linkedin,
  List,
  Plus,
  Twitter,
  Youtube,
} from 'lucide-react';
import { useCallback, useMemo, useRef, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import { MyIntelPanel } from '@/components/intel/MyIntelPanel';
import { ConnectAccountButton, PostDetailModal } from '@/components/social';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import { useProjectList } from '@/hooks/api/useProjectList';
import { socialApi, tasksApi } from '@/lib/api';
import { usersApi } from '@/lib/api/execution';
import { cn } from '@/lib/utils';

// ── Calendar helpers ──────────────────────────────────────────────────────────

const MONTH_NAMES = [
  'January',
  'February',
  'March',
  'April',
  'May',
  'June',
  'July',
  'August',
  'September',
  'October',
  'November',
  'December',
];
const DAY_NAMES = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

function buildMonthGrid(year: number, month: number): Date[][] {
  const first = new Date(year, month, 1);
  const last = new Date(year, month + 1, 0);
  const start = new Date(first);
  start.setDate(first.getDate() - first.getDay());
  const rows: Date[][] = [];
  const cur = new Date(start);
  while (rows.length < 6) {
    const row: Date[] = [];
    for (let d = 0; d < 7; d++) {
      row.push(new Date(cur));
      cur.setDate(cur.getDate() + 1);
    }
    rows.push(row);
    if (cur > last && rows.length >= 4) break;
  }
  return rows;
}

function buildWeekDays(anchor: Date): Date[] {
  const sunday = new Date(anchor);
  sunday.setDate(anchor.getDate() - anchor.getDay());
  return Array.from({ length: 7 }, (_, i) => {
    const d = new Date(sunday);
    d.setDate(sunday.getDate() + i);
    return d;
  });
}

function isSameDay(a: Date, b: Date) {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function dateKey(d: Date) {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

// ── Platform icon map ─────────────────────────────────────────────────────────

const PLATFORM_ICONS: Record<string, typeof Globe> = {
  instagram: Instagram,
  linkedin: Linkedin,
  twitter: Twitter,
  youtube: Youtube,
  facebook: Facebook,
};

const PLATFORM_COLORS: Record<string, string> = {
  instagram: 'text-pink-500',
  linkedin: 'text-blue-600',
  twitter: 'text-sky-400',
  youtube: 'text-red-500',
  facebook: 'text-blue-700',
  tiktok: 'text-foreground',
};

// ── Types ─────────────────────────────────────────────────────────────────────

type ViewMode = 'month' | 'week' | 'list';

interface CalEvent {
  id: string;
  rawId: string; // UUID of underlying record (without 'post-' / 'task-' prefix)
  title: string;
  date: Date;
  kind: 'task' | 'post';
  status: string;
  projectId?: string;
  platform?: string;
  _isPersonal?: boolean;
}

// ── Status colours ────────────────────────────────────────────────────────────

const TASK_STATUS_COLORS: Record<string, string> = {
  todo: 'bg-muted text-muted-foreground border-border/50',
  in_progress:
    'bg-blue-500/15 text-blue-700 dark:text-blue-400 border-blue-300/50',
  done: 'bg-green-500/15 text-green-700 dark:text-green-400 border-green-300/50',
  blocked: 'bg-destructive/15 text-destructive border-destructive/30',
};

const POST_STATUS_COLORS: Record<string, string> = {
  draft: 'bg-muted text-muted-foreground border-border/50',
  scheduled:
    'bg-purple-500/15 text-purple-700 dark:text-purple-400 border-purple-300/50',
  published:
    'bg-green-500/15 text-green-700 dark:text-green-400 border-green-300/50',
  pending_review:
    'bg-yellow-500/15 text-yellow-700 dark:text-yellow-400 border-yellow-300/50',
  failed: 'bg-destructive/15 text-destructive border-destructive/30',
};

// ── Main page ─────────────────────────────────────────────────────────────────

export default function CalendarPage() {
  const [view, setView] = useState<ViewMode>('month');
  const [anchor, setAnchor] = useState(new Date());
  const [selected, setSelected] = useState<Date | null>(null);
  const [showTasks, setShowTasks] = useState(true);
  const [showPosts, setShowPosts] = useState(true);
  const [newPostOpen, setNewPostOpen] = useState(false);
  const [newPostProjectId, setNewPostProjectId] = useState('');
  const [newPostCaption, setNewPostCaption] = useState('');
  const [newPostDate, setNewPostDate] = useState('');
  const [newPostPlatform, setNewPostPlatform] = useState('linkedin');
  const [newPostStatus, setNewPostStatus] = useState('scheduled');
  const [newPostCategory, setNewPostCategory] = useState('community');
  const [newPostAssigneeId, setNewPostAssigneeId] = useState('');
  const [newPostSubmitting, setNewPostSubmitting] = useState(false);
  const [newPostError, setNewPostError] = useState('');
  const [captionTopic, setCaptionTopic] = useState('');
  const [generatingCaptions, setGeneratingCaptions] = useState(false);
  const [captionOptions, setCaptionOptions] = useState<string[]>([]);
  const [selectedPostId, setSelectedPostId] = useState<string | null>(null);

  const queryClient = useQueryClient();
  const [searchParams] = useSearchParams();
  const [calTab, setCalTab] = useState<'calendar' | 'intel'>(
    searchParams.get('tab') === 'intel' ? 'intel' : 'calendar'
  );

  const { data: projects = [] } = useProjectList();
  const projectIds = projects.map((p: { id: string }) => p.id);

  const { data: users = [] } = useQuery({
    queryKey: ['users-for-calendar'],
    queryFn: () => usersApi.list(),
  });

  // ── Fetch tasks with due dates ──────────────────────────────────────────────
  const { data: tasksRaw = [] } = useQuery({
    queryKey: ['calendar-tasks', projectIds.join(',')],
    queryFn: async () => {
      if (!projectIds.length) return [];
      const all = await Promise.all(
        projectIds.map((id: string) => tasksApi.getAll(id).catch(() => []))
      );
      return all.flat();
    },
    enabled: projectIds.length > 0,
  });

  // ── Fetch social posts ──────────────────────────────────────────────────────
  const { data: postsRaw = [] } = useQuery({
    queryKey: ['calendar-posts', projectIds.join(',')],
    queryFn: async () => {
      if (!projectIds.length) return [];
      const all = await Promise.all(
        projectIds.map((id: string) =>
          socialApi
            .listPostsFiltered({ projectId: id, limit: 200 })
            .catch(() => [])
        )
      );
      return all.flat();
    },
    enabled: projectIds.length > 0,
  });

  // ── Fetch personal social posts ─────────────────────────────────────────────
  const { data: personalPostsRaw = [] } = useQuery({
    queryKey: ['calendar-personal-posts'],
    queryFn: () => socialApi.listPostsFiltered({ userId: 'me', limit: 200 }),
    staleTime: 60_000,
  });

  // ── Fetch personal social accounts ──────────────────────────────────────────
  const { data: personalAccounts = [] } = useQuery({
    queryKey: ['calendar-personal-accounts'],
    queryFn: () => socialApi.getPersonalAccounts(),
    staleTime: 120_000,
  });

  // ── Normalise to CalEvent ───────────────────────────────────────────────────
  const events = useMemo<CalEvent[]>(() => {
    const out: CalEvent[] = [];

    if (showTasks) {
      for (const t of tasksRaw as Array<Record<string, unknown>>) {
        const rawDate = (t.due_date ?? t.scheduled_start ?? t.scheduled_end) as
          | string
          | undefined;
        if (!rawDate) continue;
        const rawId = t.id as string;
        out.push({
          id: `task-${rawId}`,
          rawId,
          title: t.title as string,
          date: new Date(rawDate),
          kind: 'task',
          status: (t.status as string) ?? 'todo',
          projectId: t.project_id as string | undefined,
          _isPersonal: false,
        });
      }
    }

    function normalizePost(
      p: Record<string, unknown>,
      isPersonal: boolean
    ): CalEvent | null {
      const rawDate = (p.scheduled_for ?? p.published_at) as string | undefined;
      if (!rawDate) return null;
      let platform = 'globe';
      try {
        const arr = JSON.parse(p.platforms as string);
        platform = (arr as string[])[0] ?? 'globe';
      } catch {
        platform = (p.platforms as string) ?? 'globe';
      }
      const rawId = p.id as string;
      return {
        id: `post-${rawId}`,
        rawId,
        title:
          (p.caption as string | undefined)?.slice(0, 48) ||
          `(${platform} post)`,
        date: new Date(rawDate),
        kind: 'post',
        status: (p.status as string) ?? 'draft',
        platform,
        _isPersonal: isPersonal,
      };
    }

    if (showPosts) {
      for (const p of postsRaw as unknown as Array<Record<string, unknown>>) {
        const ev = normalizePost(p, false);
        if (ev) out.push(ev);
      }
      for (const p of personalPostsRaw as unknown as Array<
        Record<string, unknown>
      >) {
        const ev = normalizePost(p, true);
        if (ev) out.push(ev);
      }
    }

    return out;
  }, [tasksRaw, postsRaw, personalPostsRaw, showTasks, showPosts]);

  // ── Index by date ───────────────────────────────────────────────────────────
  const byDate = useMemo(() => {
    const map: Record<string, CalEvent[]> = {};
    for (const e of events) {
      const k = dateKey(e.date);
      if (!map[k]) map[k] = [];
      map[k].push(e);
    }
    return map;
  }, [events]);

  // ── Navigation ──────────────────────────────────────────────────────────────
  const today = new Date();

  function prev() {
    const d = new Date(anchor);
    if (view === 'month') d.setMonth(d.getMonth() - 1);
    else if (view === 'week') d.setDate(d.getDate() - 7);
    else d.setDate(d.getDate() - 1);
    setAnchor(d);
  }
  function next() {
    const d = new Date(anchor);
    if (view === 'month') d.setMonth(d.getMonth() + 1);
    else if (view === 'week') d.setDate(d.getDate() + 7);
    else d.setDate(d.getDate() + 1);
    setAnchor(d);
  }

  // ── Drag-and-drop rescheduling ──────────────────────────────────────────────
  const draggingId = useRef<string | null>(null);
  const [dragOver, setDragOver] = useState<string | null>(null);

  const handleDragStart = useCallback((e: React.DragEvent, event: CalEvent) => {
    if (event.kind !== 'post') {
      e.preventDefault();
      return;
    }
    draggingId.current = event.rawId;
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  const handleDrop = useCallback(
    async (day: Date) => {
      const id = draggingId.current;
      draggingId.current = null;
      setDragOver(null);
      if (!id) return;
      // Preserve original time if known; default to noon
      const scheduled = new Date(day);
      scheduled.setHours(12, 0, 0, 0);
      try {
        await socialApi.updatePost(id, {
          scheduled_for: scheduled.toISOString(),
        });
        await queryClient.invalidateQueries({ queryKey: ['calendar-posts'] });
      } catch (err) {
        console.error('Failed to reschedule post', err);
      }
    },
    [queryClient]
  );

  const monthGrid = buildMonthGrid(anchor.getFullYear(), anchor.getMonth());
  const weekDays = buildWeekDays(anchor);

  const headerLabel =
    view === 'month'
      ? `${MONTH_NAMES[anchor.getMonth()]} ${anchor.getFullYear()}`
      : view === 'week'
        ? `${weekDays[0].toLocaleDateString('en-US', { month: 'short', day: 'numeric' })} – ${weekDays[6].toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}`
        : anchor.toLocaleDateString('en-US', {
            weekday: 'long',
            month: 'long',
            day: 'numeric',
            year: 'numeric',
          });

  // ── Event pill ──────────────────────────────────────────────────────────────
  function EventPill({
    e,
    compact = false,
  }: {
    e: CalEvent;
    compact?: boolean;
  }) {
    const colorCls =
      e.kind === 'task'
        ? (TASK_STATUS_COLORS[e.status] ?? TASK_STATUS_COLORS.todo)
        : e._isPersonal
          ? 'bg-violet-500/15 text-violet-700 dark:text-violet-400 border-violet-300/50'
          : (POST_STATUS_COLORS[e.status] ?? POST_STATUS_COLORS.draft);

    if (e.kind === 'post') {
      const Icon = PLATFORM_ICONS[e.platform ?? ''] ?? Globe;
      return (
        <div
          draggable
          onDragStart={(ev) => handleDragStart(ev, e)}
          onPointerUp={(ev) => {
            // Use pointerUp instead of onClick to avoid drag-suppressed-click
            // bug in Chrome where a tiny mouse movement on draggable elements
            // prevents the click event from firing.
            ev.stopPropagation();
            if ((ev.target as HTMLElement).closest('[data-dragging]')) return;
            setSelectedPostId(e.rawId);
          }}
          className={`flex items-center gap-1 px-1.5 py-0.5 rounded border text-xs leading-tight truncate cursor-pointer hover:ring-1 hover:ring-primary/50 hover:brightness-105 active:opacity-70 select-none ${colorCls}`}
        >
          <Icon
            className={`h-2.5 w-2.5 shrink-0 ${PLATFORM_COLORS[e.platform ?? ''] ?? ''}`}
          />
          {!compact && <span className="truncate">{e.title}</span>}
        </div>
      );
    }
    return (
      <div
        className={`flex items-center gap-1 px-1.5 py-0.5 rounded border text-xs leading-tight truncate ${colorCls}`}
      >
        {e.status === 'done' ? (
          <CheckCircle2 className="h-2.5 w-2.5 shrink-0" />
        ) : (
          <Circle className="h-2.5 w-2.5 shrink-0" />
        )}
        {!compact && <span className="truncate">{e.title}</span>}
      </div>
    );
  }

  // ── Day detail sidebar ──────────────────────────────────────────────────────
  function DayDetail({ day }: { day: Date }) {
    const dayEvents = byDate[dateKey(day)] ?? [];
    return (
      <div className="w-72 shrink-0 border-l bg-card/60 flex flex-col">
        <div className="flex items-center justify-between px-4 py-3 border-b">
          <span className="text-sm font-medium">
            {day.toLocaleDateString('en-US', {
              weekday: 'short',
              month: 'short',
              day: 'numeric',
            })}
          </span>
          <button
            onClick={() => setSelected(null)}
            className="text-muted-foreground hover:text-foreground"
          >
            <ChevronRight className="h-4 w-4" />
          </button>
        </div>
        <ScrollArea className="flex-1">
          {dayEvents.length === 0 ? (
            <p className="text-xs text-muted-foreground p-4 text-center">
              No events
            </p>
          ) : (
            <div className="p-3 space-y-2">
              {dayEvents.map((e) => (
                <div
                  key={e.id}
                  onPointerUp={() => {
                    if (e.kind === 'post') setSelectedPostId(e.rawId);
                  }}
                  className={`flex items-start gap-2 p-2 rounded-lg border border-border/40 hover:bg-muted/30 ${e.kind === 'post' ? 'cursor-pointer hover:border-primary/30' : ''}`}
                >
                  {e.kind === 'task' ? (
                    e.status === 'done' ? (
                      <CheckCircle2 className="h-3.5 w-3.5 mt-0.5 shrink-0 text-green-500" />
                    ) : (
                      <Circle className="h-3.5 w-3.5 mt-0.5 shrink-0 text-muted-foreground" />
                    )
                  ) : (
                    (() => {
                      const Icon = PLATFORM_ICONS[e.platform ?? ''] ?? Globe;
                      return (
                        <Icon
                          className={`h-3.5 w-3.5 mt-0.5 shrink-0 ${PLATFORM_COLORS[e.platform ?? ''] ?? ''}`}
                        />
                      );
                    })()
                  )}
                  <div className="min-w-0">
                    <p className="text-xs font-medium line-clamp-2">
                      {e.title}
                    </p>
                    <div className="flex items-center gap-1.5 mt-0.5">
                      <span
                        className={`text-xs px-1 py-0.5 rounded border ${
                          e.kind === 'task'
                            ? (TASK_STATUS_COLORS[e.status] ?? '')
                            : (POST_STATUS_COLORS[e.status] ?? '')
                        }`}
                      >
                        {e.status.replace(/_/g, ' ')}
                      </span>
                      <span className="text-xs text-muted-foreground capitalize">
                        {e.kind}
                      </span>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </ScrollArea>
      </div>
    );
  }

  // ── List view ───────────────────────────────────────────────────────────────
  function ListView() {
    const sorted = [...events].sort(
      (a, b) => a.date.getTime() - b.date.getTime()
    );
    const groups: Record<string, CalEvent[]> = {};
    for (const e of sorted) {
      const k = dateKey(e.date);
      if (!groups[k]) groups[k] = [];
      groups[k].push(e);
    }
    if (!sorted.length)
      return (
        <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
          No scheduled events
        </div>
      );
    return (
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-4">
          {Object.entries(groups).map(([key, evts]) => {
            const day = new Date(key + 'T00:00:00');
            return (
              <div key={key}>
                <div
                  className={`text-xs font-semibold mb-2 flex items-center gap-2 ${isSameDay(day, today) ? 'text-primary' : 'text-muted-foreground'}`}
                >
                  <span>
                    {day.toLocaleDateString('en-US', {
                      weekday: 'short',
                      month: 'short',
                      day: 'numeric',
                    })}
                  </span>
                  {isSameDay(day, today) && (
                    <Badge variant="secondary" className="text-xs h-4">
                      Today
                    </Badge>
                  )}
                </div>
                <div className="space-y-1.5 pl-2 border-l-2 border-border/50">
                  {evts.map((e) => (
                    <div
                      key={e.id}
                      onPointerUp={() => {
                        if (e.kind === 'post') setSelectedPostId(e.rawId);
                      }}
                      className={`flex items-center gap-2 p-2 rounded-md hover:bg-muted/40 border border-transparent hover:border-border/40 ${e.kind === 'post' ? 'cursor-pointer hover:border-primary/30' : ''}`}
                    >
                      {e.kind === 'task' ? (
                        e.status === 'done' ? (
                          <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-500" />
                        ) : (
                          <Clock className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
                        )
                      ) : (
                        (() => {
                          const Icon =
                            PLATFORM_ICONS[e.platform ?? ''] ?? Globe;
                          return (
                            <Icon
                              className={`h-3.5 w-3.5 shrink-0 ${PLATFORM_COLORS[e.platform ?? ''] ?? ''}`}
                            />
                          );
                        })()
                      )}
                      <span className="text-sm flex-1 truncate">{e.title}</span>
                      <span
                        className={`text-xs px-1.5 py-0.5 rounded border shrink-0 ${
                          e.kind === 'task'
                            ? (TASK_STATUS_COLORS[e.status] ?? '')
                            : (POST_STATUS_COLORS[e.status] ?? '')
                        }`}
                      >
                        {e.status.replace(/_/g, ' ')}
                      </span>
                      <span className="text-xs text-muted-foreground capitalize shrink-0">
                        {e.kind}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            );
          })}
        </div>
      </ScrollArea>
    );
  }

  // ── Month grid ──────────────────────────────────────────────────────────────
  function MonthGrid() {
    return (
      <div className="flex-1 flex flex-col min-h-0">
        {/* Day headers */}
        <div className="grid grid-cols-7 border-b">
          {DAY_NAMES.map((d) => (
            <div
              key={d}
              className="py-2 text-center text-xs font-medium text-muted-foreground"
            >
              {d}
            </div>
          ))}
        </div>
        {/* Grid rows */}
        <div
          className="flex-1 grid"
          style={{ gridTemplateRows: `repeat(${monthGrid.length}, 1fr)` }}
        >
          {monthGrid.map((row, ri) => (
            <div key={ri} className="grid grid-cols-7 border-b last:border-b-0">
              {row.map((day, di) => {
                const key = dateKey(day);
                const dayEvts = byDate[key] ?? [];
                const isToday = isSameDay(day, today);
                const isCur = day.getMonth() === anchor.getMonth();
                const isSel = selected && isSameDay(day, selected);
                return (
                  <div
                    key={di}
                    onClick={() => setSelected(isSel ? null : day)}
                    onDragOver={(ev) => {
                      ev.preventDefault();
                      setDragOver(key);
                    }}
                    onDragLeave={() => setDragOver(null)}
                    onDrop={() => handleDrop(day)}
                    className={cn(
                      'border-r last:border-r-0 p-1 cursor-pointer hover:bg-muted/30 transition-colors min-h-[80px]',
                      !isCur && 'bg-muted/10',
                      isSel && 'bg-primary/5 ring-1 ring-inset ring-primary/30',
                      dragOver === key &&
                        'bg-primary/10 ring-1 ring-inset ring-primary/50'
                    )}
                  >
                    <div
                      className={cn(
                        'text-xs font-medium w-6 h-6 flex items-center justify-center rounded-full mb-1',
                        isToday && 'bg-primary text-primary-foreground',
                        !isToday && !isCur && 'text-muted-foreground/50',
                        !isToday && isCur && 'text-foreground'
                      )}
                    >
                      {day.getDate()}
                    </div>
                    <div className="space-y-0.5">
                      {dayEvts.slice(0, 3).map((e) => (
                        <EventPill key={e.id} e={e} />
                      ))}
                      {dayEvts.length > 3 && (
                        <div className="text-xs text-muted-foreground pl-1">
                          +{dayEvts.length - 3} more
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    );
  }

  // ── Week grid ───────────────────────────────────────────────────────────────
  function WeekGrid() {
    return (
      <div className="flex-1 flex flex-col min-h-0">
        <div className="grid grid-cols-7 border-b">
          {weekDays.map((day, i) => {
            const isToday = isSameDay(day, today);
            const isSel = selected && isSameDay(day, selected);
            return (
              <div
                key={i}
                onClick={() => setSelected(isSel ? null : day)}
                className={cn(
                  'py-2 px-1 text-center border-r last:border-r-0 cursor-pointer hover:bg-muted/30',
                  isSel && 'bg-primary/5'
                )}
              >
                <div className="text-xs text-muted-foreground">
                  {DAY_NAMES[day.getDay()]}
                </div>
                <div
                  className={cn(
                    'text-sm font-medium w-7 h-7 flex items-center justify-center rounded-full mx-auto mt-0.5',
                    isToday && 'bg-primary text-primary-foreground'
                  )}
                >
                  {day.getDate()}
                </div>
              </div>
            );
          })}
        </div>
        <ScrollArea className="flex-1">
          <div className="grid grid-cols-7 h-full">
            {weekDays.map((day, i) => {
              const dayEvts = byDate[dateKey(day)] ?? [];
              const isSel = selected && isSameDay(day, selected);
              const wk = dateKey(day);
              return (
                <div
                  key={i}
                  onDragOver={(ev) => {
                    ev.preventDefault();
                    setDragOver(wk);
                  }}
                  onDragLeave={() => setDragOver(null)}
                  onDrop={() => handleDrop(day)}
                  className={cn(
                    'border-r last:border-r-0 p-2 space-y-1 min-h-48',
                    isSel && 'bg-primary/5',
                    dragOver === wk &&
                      'bg-primary/10 ring-1 ring-inset ring-primary/50'
                  )}
                >
                  {dayEvts.map((e) => (
                    <EventPill key={e.id} e={e} />
                  ))}
                </div>
              );
            })}
          </div>
        </ScrollArea>
      </div>
    );
  }

  async function handleGenerateCaptions() {
    if (!captionTopic.trim()) return;
    setGeneratingCaptions(true);
    setCaptionOptions([]);
    try {
      const captions = await socialApi.generateCaptions({
        platform: newPostPlatform,
        topic: captionTopic.trim(),
      });
      setCaptionOptions(captions);
    } catch (err) {
      console.error('Caption generation failed', err);
    } finally {
      setGeneratingCaptions(false);
    }
  }

  async function handleCreatePost(e: React.FormEvent) {
    e.preventDefault();
    if (!newPostProjectId || !newPostCaption.trim()) return;
    setNewPostSubmitting(true);
    setNewPostError('');
    try {
      await socialApi.createPost({
        project_id: newPostProjectId,
        caption: newPostCaption.trim(),
        platforms: [newPostPlatform],
        status: newPostStatus,
        scheduled_for: newPostDate
          ? new Date(newPostDate).toISOString()
          : undefined,
        category: newPostCategory,
        assignee_id: newPostAssigneeId || undefined,
      });
      setNewPostOpen(false);
      setNewPostCaption('');
      setNewPostDate('');
      setNewPostAssigneeId('');
      await queryClient.invalidateQueries({ queryKey: ['calendar-posts'] });
    } catch (err) {
      setNewPostError(
        err instanceof Error ? err.message : 'Failed to create post'
      );
    } finally {
      setNewPostSubmitting(false);
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────────
  return (
    <div className="flex h-full bg-background">
      {/* Post detail modal */}
      <PostDetailModal
        postId={selectedPostId}
        open={!!selectedPostId}
        onClose={() => setSelectedPostId(null)}
        onDeleted={() =>
          queryClient.invalidateQueries({ queryKey: ['calendar-posts'] })
        }
      />

      {/* New Post dialog */}
      <Dialog open={newPostOpen} onOpenChange={setNewPostOpen}>
        <DialogContent className="max-w-lg">
          <DialogHeader>
            <DialogTitle>New Social Post</DialogTitle>
          </DialogHeader>
          <form onSubmit={handleCreatePost} className="space-y-4 pt-2">
            <div className="space-y-1.5">
              <Label>Project</Label>
              <Select
                value={newPostProjectId}
                onValueChange={setNewPostProjectId}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Select project" />
                </SelectTrigger>
                <SelectContent>
                  {(projects as Array<{ id: string; name: string }>).map(
                    (p) => (
                      <SelectItem key={p.id} value={p.id}>
                        {p.name}
                      </SelectItem>
                    )
                  )}
                </SelectContent>
              </Select>
            </div>
            {/* AI Caption Generator */}
            <div className="space-y-1.5 rounded-lg border border-dashed border-border p-3 bg-muted/20">
              <Label className="text-xs text-muted-foreground">
                AI Caption Generator
              </Label>
              <div className="flex gap-2">
                <Input
                  placeholder="Topic (e.g. 'NFT drop this Friday')"
                  value={captionTopic}
                  onChange={(e) => setCaptionTopic(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') {
                      e.preventDefault();
                      handleGenerateCaptions();
                    }
                  }}
                  className="h-8 text-sm"
                />
                <Button
                  type="button"
                  size="sm"
                  variant="secondary"
                  className="h-8 shrink-0"
                  onClick={handleGenerateCaptions}
                  disabled={generatingCaptions || !captionTopic.trim()}
                >
                  {generatingCaptions ? '...' : 'Generate'}
                </Button>
              </div>
              {captionOptions.length > 0 && (
                <div className="space-y-1.5 pt-1">
                  {captionOptions.map((c, i) => (
                    <button
                      key={i}
                      type="button"
                      onClick={() => {
                        setNewPostCaption(c);
                        setCaptionOptions([]);
                      }}
                      className="w-full text-left text-xs p-2 rounded border hover:bg-muted/60 line-clamp-3 border-border/50"
                    >
                      {c}
                    </button>
                  ))}
                </div>
              )}
            </div>
            <div className="space-y-1.5">
              <Label>Caption</Label>
              <Textarea
                value={newPostCaption}
                onChange={(e) => setNewPostCaption(e.target.value)}
                placeholder="What's the post about?"
                rows={4}
                required
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label>Platform</Label>
                <Select
                  value={newPostPlatform}
                  onValueChange={setNewPostPlatform}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="linkedin">LinkedIn</SelectItem>
                    <SelectItem value="instagram">Instagram</SelectItem>
                    <SelectItem value="twitter">Twitter / X</SelectItem>
                    <SelectItem value="facebook">Facebook</SelectItem>
                    <SelectItem value="tiktok">TikTok</SelectItem>
                    <SelectItem value="youtube">YouTube</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>Category</Label>
                <Select
                  value={newPostCategory}
                  onValueChange={setNewPostCategory}
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="community">Community</SelectItem>
                    <SelectItem value="events">Events</SelectItem>
                    <SelectItem value="vibe">Vibe</SelectItem>
                    <SelectItem value="drinks">Drinks</SelectItem>
                    <SelectItem value="promo">Promo</SelectItem>
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label>Status</Label>
                <Select value={newPostStatus} onValueChange={setNewPostStatus}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="draft">Draft</SelectItem>
                    <SelectItem value="scheduled">Scheduled</SelectItem>
                    <SelectItem value="pending_review">
                      Pending Review
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label>Schedule date</Label>
                <Input
                  type="datetime-local"
                  value={newPostDate}
                  onChange={(e) => setNewPostDate(e.target.value)}
                />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label>Assignee</Label>
              <Select
                value={newPostAssigneeId}
                onValueChange={setNewPostAssigneeId}
              >
                <SelectTrigger>
                  <SelectValue placeholder="Unassigned" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="">Unassigned</SelectItem>
                  {(
                    users as Array<{
                      id: string;
                      full_name?: string;
                      username?: string;
                    }>
                  ).map((u) => (
                    <SelectItem key={u.id} value={u.id}>
                      {u.full_name || u.username || u.id}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            {newPostError && (
              <p className="text-sm text-destructive">{newPostError}</p>
            )}
            <div className="flex justify-end gap-2 pt-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => setNewPostOpen(false)}
              >
                Cancel
              </Button>
              <Button
                type="submit"
                disabled={
                  newPostSubmitting ||
                  !newPostProjectId ||
                  !newPostCaption.trim()
                }
              >
                {newPostSubmitting ? 'Creating...' : 'Create Post'}
              </Button>
            </div>
          </form>
        </DialogContent>
      </Dialog>

      {/* Main panel */}
      <div className="flex-1 flex flex-col min-h-0">
        {/* Toolbar */}
        <div className="flex flex-wrap items-center justify-between px-4 py-2 border-b shrink-0 gap-2">
          <div className="flex items-center gap-2">
            <div className="rounded-lg bg-primary p-1.5 mr-1 shrink-0">
              <CalendarDays className="h-4 w-4 text-primary-foreground" />
            </div>
            <h1 className="text-sm font-bold mr-2 hidden sm:block">Calendar</h1>
            <button onClick={prev} className="p-1 rounded hover:bg-muted">
              <ChevronLeft className="h-4 w-4" />
            </button>
            <span className="text-sm font-semibold min-w-0 sm:min-w-48 text-center truncate">
              {headerLabel}
            </span>
            <button onClick={next} className="p-1 rounded hover:bg-muted">
              <ChevronRight className="h-4 w-4" />
            </button>
            <Button
              variant="outline"
              size="sm"
              className="ml-1 h-7 text-xs shrink-0"
              onClick={() => setAnchor(new Date())}
            >
              Today
            </Button>
          </div>

          <div className="flex items-center gap-2">
            {/* Filters */}
            <button
              onClick={() => setShowTasks((v) => !v)}
              className={cn(
                'flex items-center gap-1.5 px-2.5 py-1 rounded-full border text-xs font-medium transition-colors',
                showTasks
                  ? 'bg-blue-500/15 border-blue-300/50 text-blue-700 dark:text-blue-400'
                  : 'border-border text-muted-foreground'
              )}
            >
              <CheckCircle2 className="h-3 w-3" /> Tasks
            </button>
            <button
              onClick={() => setShowPosts((v) => !v)}
              className={cn(
                'flex items-center gap-1.5 px-2.5 py-1 rounded-full border text-xs font-medium transition-colors',
                showPosts
                  ? 'bg-purple-500/15 border-purple-300/50 text-purple-700 dark:text-purple-400'
                  : 'border-border text-muted-foreground'
              )}
            >
              <Globe className="h-3 w-3" /> Posts
            </button>

            <Button
              size="sm"
              className="h-7 gap-1 text-xs ml-2"
              onClick={() => {
                const firstId =
                  (projects as Array<{ id: string }>)[0]?.id ?? '';
                setNewPostProjectId(firstId);
                setNewPostOpen(true);
              }}
            >
              <Plus className="h-3.5 w-3.5" /> New Post
            </Button>

            {/* View switcher */}
            <div className="flex items-center border rounded-md overflow-hidden ml-2">
              {(
                [
                  ['month', Calendar],
                  ['week', CalendarDays],
                  ['list', List],
                ] as [ViewMode, typeof Calendar][]
              ).map(([v, Icon]) => (
                <button
                  key={v}
                  onClick={() => setView(v)}
                  className={cn(
                    'px-2.5 py-1.5 text-xs flex items-center gap-1 capitalize',
                    view === v
                      ? 'bg-primary text-primary-foreground'
                      : 'text-muted-foreground hover:text-foreground hover:bg-muted'
                  )}
                >
                  <Icon className="h-3.5 w-3.5" />
                  <span className="hidden sm:inline">{v}</span>
                </button>
              ))}
            </div>
          </div>
        </div>

        {/* Tab strip + personal social bar */}
        <div className="px-4 pt-3 shrink-0">
          <div className="flex gap-1 mb-3">
            <Button
              size="sm"
              variant={calTab === 'calendar' ? 'default' : 'ghost'}
              className="h-7 text-xs gap-1.5"
              onClick={() => setCalTab('calendar')}
            >
              <Calendar className="h-3.5 w-3.5" /> Calendar
            </Button>
            <Button
              size="sm"
              variant={calTab === 'intel' ? 'default' : 'ghost'}
              className="h-7 text-xs gap-1.5"
              onClick={() => setCalTab('intel')}
            >
              <BrainCircuit className="h-3.5 w-3.5" /> My Intel
            </Button>
          </div>

          {calTab === 'calendar' && (
            <div className="flex items-center gap-2 mb-3 flex-wrap">
              <span className="text-xs text-muted-foreground font-medium">
                My Social:
              </span>
              {(
                personalAccounts as Array<{
                  id: string;
                  platform: string;
                  username?: string | null;
                }>
              ).map((acc) => (
                <div
                  key={acc.id}
                  className="flex items-center gap-1 px-2 py-0.5 rounded-full bg-violet-500/10 border border-violet-500/20 text-xs text-violet-600 dark:text-violet-400"
                >
                  <span className="capitalize">{acc.platform}</span>
                  {acc.username && (
                    <span className="text-muted-foreground">
                      @{acc.username}
                    </span>
                  )}
                </div>
              ))}
              <ConnectAccountButton className="h-6 text-xs" />
            </div>
          )}
        </div>

        {/* Calendar body */}
        {calTab === 'intel' ? (
          <div className="flex-1 overflow-auto px-4 pb-4">
            <MyIntelPanel />
          </div>
        ) : (
          <div className="flex flex-1 min-h-0">
            <div className="flex-1 flex flex-col min-h-0">
              {view === 'month' && <MonthGrid />}
              {view === 'week' && <WeekGrid />}
              {view === 'list' && <ListView />}
            </div>

            {/* Day detail panel */}
            {selected && <DayDetail day={selected} />}
          </div>
        )}
      </div>
    </div>
  );
}
