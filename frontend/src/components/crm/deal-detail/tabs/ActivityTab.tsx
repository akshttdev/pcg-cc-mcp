import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { taskKeys } from '@/lib/query-keys';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import {
  CheckCircle2,
  Clock,
  Circle,
  Zap,
  Bot,
  User2,
  Plus,
  ChevronDown,
  ChevronUp,
  FolderKanban,
  Activity,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { cn } from '@/lib/utils';
import { tasksApi } from '@/lib/api/tasks';
import { projectsApi } from '@/lib/api/projects';
import { useCrmActivities } from '@/hooks/useCrmActivities';
import { CrmActivityForm } from '../../CrmActivityForm';
import type { CrmDealWithContact } from '@/types/crm';
import type { TaskWithArchive } from '@/lib/api/client';

interface ActivityTabProps {
  deal: CrmDealWithContact;
  projectId?: string; // unused — derived from deal.project_id
}

// ── unified event type ────────────────────────────────────────────────────────

type EventKind = 'task' | 'vibe' | 'crm';

interface UnifiedEvent {
  id: string;
  kind: EventKind;
  timestamp: Date;
  // task fields
  task?: TaskWithArchive;
  // vibe fields
  vibeAmount?: number;
  vibeDesc?: string;
  vibeModel?: string;
  // crm fields
  crmType?: string;
  crmSubject?: string;
  crmDesc?: string;
}

function getStatusIcon(status: string) {
  if (status === 'completed' || status === 'done') return CheckCircle2;
  if (status === 'in_progress') return Activity;
  if (status === 'cancelled') return Circle;
  return Clock;
}

function getStatusColor(status: string) {
  if (status === 'completed' || status === 'done') return 'text-green-500';
  if (status === 'in_progress') return 'text-blue-500';
  if (status === 'cancelled') return 'text-muted-foreground';
  return 'text-amber-400';
}

function statusLabel(status: string) {
  const map: Record<string, string> = {
    todo: 'To Do',
    in_progress: 'In Progress',
    completed: 'Done',
    done: 'Done',
    cancelled: 'Cancelled',
    blocked: 'Blocked',
    in_review: 'In Review',
  };
  return map[status] ?? status;
}

// ── TaskEventCard ─────────────────────────────────────────────────────────────

function TaskEventCard({ task }: { task: TaskWithArchive }) {
  const [open, setOpen] = useState(false);
  const StatusIcon = getStatusIcon(task.status);
  const statusColor = getStatusColor(task.status);

  return (
    <div className="flex gap-3 items-start">
      <div className={cn('mt-0.5 shrink-0', statusColor)}>
        <StatusIcon className="h-4 w-4" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 flex-wrap">
          <span className="text-sm font-medium leading-tight">{task.title}</span>
          <Badge
            variant="outline"
            className={cn('text-xs px-1.5 py-0 shrink-0', statusColor)}
          >
            {statusLabel(task.status)}
          </Badge>
        </div>
        <div className="flex items-center gap-2 mt-0.5 text-xs text-muted-foreground">
          {task.assigned_agent ? (
            <span className="flex items-center gap-1">
              <Bot className="h-3 w-3" />
              {task.assigned_agent}
            </span>
          ) : task.assignee_id ? (
            <span className="flex items-center gap-1">
              <User2 className="h-3 w-3" />
              Team member
            </span>
          ) : null}
          <span className="ml-auto">
            {formatDistanceToNow(new Date(task.created_at), { addSuffix: true })}
          </span>
        </div>
        {task.description && (
          <div>
            <button
              className="text-xs text-muted-foreground hover:text-foreground mt-1 flex items-center gap-0.5"
              onClick={() => setOpen((v) => !v)}
            >
              {open ? <ChevronUp className="h-3 w-3" /> : <ChevronDown className="h-3 w-3" />}
              {open ? 'Hide' : 'Details'}
            </button>
            {open && (
              <p className="text-xs text-muted-foreground mt-1 bg-muted/40 rounded p-2 leading-relaxed whitespace-pre-wrap line-clamp-6">
                {task.description}
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ── VibeEventCard ─────────────────────────────────────────────────────────────

function VibeEventCard({ event }: { event: UnifiedEvent }) {
  const amount = event.vibeAmount ?? 0;
  const usd = amount > 0 ? (amount / 100).toFixed(2) : null;
  return (
    <div className="flex gap-3 items-start">
      <div className="mt-0.5 shrink-0 text-violet-500">
        <Zap className="h-4 w-4" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">
            {event.vibeDesc ?? 'LLM Usage'}
          </span>
          {amount > 0 && (
            <Badge variant="outline" className="text-xs px-1.5 py-0 text-violet-500 shrink-0">
              {amount.toLocaleString()} VIBE
              {usd && <span className="text-muted-foreground ml-1">(${usd})</span>}
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2 mt-0.5 text-xs text-muted-foreground">
          {event.vibeModel && <span>{event.vibeModel}</span>}
          <span className="ml-auto">
            {formatDistanceToNow(event.timestamp, { addSuffix: true })}
          </span>
        </div>
      </div>
    </div>
  );
}

// ── CrmEventCard ─────────────────────────────────────────────────────────────

function CrmEventCard({ event }: { event: UnifiedEvent }) {
  const label = (event.crmType ?? '')
    .split('_')
    .map((w: string) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(' ');
  return (
    <div className="flex gap-3 items-start">
      <div className="mt-0.5 shrink-0 text-blue-400">
        <Activity className="h-4 w-4" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{event.crmSubject ?? label}</span>
          <Badge variant="outline" className="text-xs px-1.5 py-0 shrink-0">
            {label}
          </Badge>
        </div>
        {event.crmDesc && (
          <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{event.crmDesc}</p>
        )}
        <div className="mt-0.5 text-xs text-muted-foreground text-right">
          {formatDistanceToNow(event.timestamp, { addSuffix: true })}
        </div>
      </div>
    </div>
  );
}

// ── ActivityTab ───────────────────────────────────────────────────────────────

export function ActivityTab({ deal }: ActivityTabProps) {
  const [formOpen, setFormOpen] = useState(false);
  const projectId = deal.project_id;

  const { data: tasks = [], isLoading: tasksLoading } = useQuery({
    queryKey: taskKeys.list(projectId!),
    queryFn: () => tasksApi.getAll(projectId!),
    enabled: !!projectId,
    staleTime: 30_000,
  });

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const { data: vibeTransactions = [], isLoading: vibeLoading } = useQuery<any[]>({
    queryKey: ['vibe', 'transactions', projectId],
    queryFn: () => projectsApi.getVibeTransactions(projectId!, 100),
    enabled: !!projectId,
    staleTime: 60_000,
  });

  const { data: crmActivities = [], isLoading: crmLoading } = useCrmActivities({
    dealId: deal.id,
    limit: 50,
  });

  const isLoading = tasksLoading || vibeLoading || crmLoading;

  // Merge all events into a single timeline
  const events = useMemo<UnifiedEvent[]>(() => {
    const result: UnifiedEvent[] = [];

    for (const task of tasks) {
      result.push({
        id: `task-${task.id}`,
        kind: 'task',
        timestamp: new Date(task.created_at),
        task: task as TaskWithArchive,
      });
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    for (const tx of vibeTransactions as any[]) {
      result.push({
        id: `vibe-${tx.id}`,
        kind: 'vibe',
        timestamp: new Date(tx.created_at),
        vibeAmount: tx.amount_vibe,
        vibeDesc: tx.description,
        vibeModel: tx.model,
      });
    }

    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    for (const act of crmActivities as any[]) {
      result.push({
        id: `crm-${act.id}`,
        kind: 'crm',
        timestamp: new Date(act.activity_at),
        crmType: act.activity_type,
        crmSubject: act.subject,
        crmDesc: act.description,
      });
    }

    return result.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
  }, [tasks, vibeTransactions, crmActivities]);

  // Group by date
  const grouped = useMemo(() => {
    const groups = new Map<string, UnifiedEvent[]>();
    for (const ev of events) {
      const key = ev.timestamp.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
      if (!groups.has(key)) groups.set(key, []);
      groups.get(key)!.push(ev);
    }
    return groups;
  }, [events]);

  // Aggregate VIBE cost
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const totalVibe = (vibeTransactions as any[]).reduce(
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (sum: number, tx: any) => sum + (tx.amount_vibe ?? 0),
    0
  );

  if (!projectId) {
    return (
      <div className="p-5 space-y-5">
        <div className="text-center py-8 text-sm text-muted-foreground">
          <FolderKanban className="h-8 w-8 mx-auto mb-2 opacity-30" />
          <p className="font-medium text-foreground">No project linked</p>
          <p className="text-xs mt-1">Convert this deal to a project to see workflow activity here.</p>
        </div>

        {/* CRM activities for unlinked deals */}
        {crmActivities.length > 0 && (
          <div className="space-y-3">
            <h3 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              CRM Activity
            </h3>
            {/* eslint-disable-next-line @typescript-eslint/no-explicit-any */}
            {(crmActivities as any[]).map((act) => (
              <CrmEventCard
                key={act.id}
                event={{
                  id: act.id,
                  kind: 'crm',
                  timestamp: new Date(act.activity_at),
                  crmType: act.activity_type,
                  crmSubject: act.subject,
                  crmDesc: act.description,
                }}
              />
            ))}
          </div>
        )}

        <Button
          variant="outline"
          size="sm"
          className="w-full gap-1.5"
          onClick={() => setFormOpen(true)}
        >
          <Plus className="h-3.5 w-3.5" />
          Log Activity
        </Button>
        <CrmActivityForm
          open={formOpen}
          onOpenChange={setFormOpen}
          projectId={deal.organization_id}
          dealId={deal.id}
        />
      </div>
    );
  }

  return (
    <div className="p-5 space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium">Workflow Activity</h3>
          {totalVibe > 0 && (
            <Badge variant="outline" className="text-xs px-1.5 py-0 text-violet-500 gap-0.5">
              <Zap className="h-2.5 w-2.5" />
              {totalVibe.toLocaleString()} VIBE spent
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" asChild>
            <Link to={`/projects/${projectId}/tasks`}>
              <FolderKanban className="h-3 w-3" />
              Board
            </Link>
          </Button>
          <Button variant="outline" size="sm" className="h-7 text-xs gap-1" onClick={() => setFormOpen(true)}>
            <Plus className="h-3 w-3" />
            Log
          </Button>
        </div>
      </div>

      {/* Summary cards */}
      {!isLoading && (tasks.length > 0 || totalVibe > 0) && (
        <Card className="bg-muted/30 border-border/60">
          <CardContent className="p-3">
            <div className="grid grid-cols-3 gap-3 text-center">
              <div>
                <p className="text-lg font-semibold">{tasks.length}</p>
                <p className="text-xs text-muted-foreground uppercase tracking-wide">Tasks</p>
              </div>
              <div>
                <p className="text-lg font-semibold">
                  {tasks.filter((t) => t.status === 'done').length}
                </p>
                <p className="text-xs text-muted-foreground uppercase tracking-wide">Done</p>
              </div>
              <div>
                <p className="text-lg font-semibold text-violet-500">
                  {totalVibe > 0 ? `$${(totalVibe / 100).toFixed(2)}` : '—'}
                </p>
                <p className="text-xs text-muted-foreground uppercase tracking-wide">VIBE Cost</p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {isLoading ? (
        <div className="space-y-3 animate-pulse">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="flex gap-3">
              <div className="h-4 w-4 rounded-full bg-muted shrink-0 mt-0.5" />
              <div className="flex-1 space-y-1.5">
                <div className="h-3.5 bg-muted rounded w-3/4" />
                <div className="h-3 bg-muted rounded w-1/2" />
              </div>
            </div>
          ))}
        </div>
      ) : events.length === 0 ? (
        <EmptyState
          icon={Clock}
          title="No activity yet on this project"
          className="py-8"
        />
      ) : (
        <div className="space-y-6">
          {Array.from(grouped.entries()).map(([date, dayEvents]) => (
            <div key={date}>
              <div className="text-xs font-medium text-muted-foreground mb-3 uppercase tracking-wide">
                {date}
              </div>
              <div className="space-y-4 border-l border-border/50 pl-3 ml-1.5">
                {dayEvents.map((ev) => (
                  <div key={ev.id}>
                    {ev.kind === 'task' && ev.task && <TaskEventCard task={ev.task} />}
                    {ev.kind === 'vibe' && <VibeEventCard event={ev} />}
                    {ev.kind === 'crm' && <CrmEventCard event={ev} />}
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <CrmActivityForm
        open={formOpen}
        onOpenChange={setFormOpen}
        projectId={deal.organization_id}
        dealId={deal.id}
      />
    </div>
  );
}
