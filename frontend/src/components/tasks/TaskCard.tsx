import { useCallback, useEffect, useRef } from 'react';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { KanbanCard } from '@/components/ui/shadcn-io/kanban';
import {
  Archive,
  CheckCircle,
  Copy,
  Edit,
  Loader2,
  MoreHorizontal,
  Trash2,
  XCircle,
  Bot,
  User,
  Zap,
  AlertTriangle,
  ArrowUp,
  ArrowDown,
  Calendar,
} from 'lucide-react';
import { TimeTrackerWidget } from '@/components/time-tracking/TimeTrackerWidget';
import { AgentFlowBadges } from './AgentFlowBadges';
import { ExecutionSummaryInline } from './ExecutionSummaryInline';
import { TagChips } from '@/components/ui/tag-chips';
import type { AgentFlow, UserListItem, TaskWithArchive } from '@/lib/api';

type Task = TaskWithArchive;

const PRIORITY_BORDER_COLORS: Record<string, string> = {
  critical: 'border-l-[3px] border-l-red-500',
  high: 'border-l-[3px] border-l-amber-500',
  medium: 'border-l-[3px] border-l-blue-400',
  low: 'border-l-[3px] border-l-slate-300 dark:border-l-slate-600',
};

interface TaskCardProps {
  task: Task;
  index: number;
  status: string;
  onEdit: (task: Task) => void;
  onDelete: (taskId: string) => void;
  onDuplicate?: (task: Task) => void;
  onArchive?: (task: Task) => void;
  onViewDetails: (task: Task) => void;
  isOpen?: boolean;
  selectionMode?: boolean;
  isSelected?: boolean;
  onToggleSelection?: (taskId: string) => void;
  agentFlow?: AgentFlow;
  dimmed?: boolean;
  usersMap?: Map<string, UserListItem>;
}

function PriorityBadge({ priority }: { priority: string }) {
  switch (priority) {
    case 'critical':
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium bg-red-100 text-red-700 dark:bg-red-950/50 dark:text-red-400 border border-red-200 dark:border-red-800" title="Critical">
          <AlertTriangle className="h-2.5 w-2.5" />
          <span>Critical</span>
        </span>
      );
    case 'high':
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium bg-orange-100 text-orange-700 dark:bg-orange-950/50 dark:text-orange-400 border border-orange-200 dark:border-orange-800" title="High">
          <ArrowUp className="h-2.5 w-2.5" />
          <span>High</span>
        </span>
      );
    case 'low':
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400 border border-slate-200 dark:border-slate-700" title="Low">
          <ArrowDown className="h-2.5 w-2.5" />
          <span>Low</span>
        </span>
      );
    default:
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium bg-blue-50 text-blue-600 dark:bg-blue-950/40 dark:text-blue-400 border border-blue-200 dark:border-blue-800" title="Medium">
          <span>Medium</span>
        </span>
      );
  }
}

function DueDateBadge({ dueDate }: { dueDate: string }) {
  const due = new Date(dueDate);
  const now = new Date();
  const diffDays = Math.ceil((due.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

  let colorClasses: string;
  if (diffDays < 0) {
    colorClasses = 'bg-red-100 text-red-700 dark:bg-red-950/50 dark:text-red-400 border-red-200 dark:border-red-800';
  } else if (diffDays <= 2) {
    colorClasses = 'bg-amber-100 text-amber-700 dark:bg-amber-950/50 dark:text-amber-400 border-amber-200 dark:border-amber-800';
  } else {
    colorClasses = 'bg-blue-50 text-blue-600 dark:bg-blue-950/40 dark:text-blue-400 border-blue-200 dark:border-blue-800';
  }

  const label = diffDays < 0
    ? `${Math.abs(diffDays)}d overdue`
    : diffDays === 0
    ? 'Today'
    : diffDays === 1
    ? 'Tomorrow'
    : `${due.toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}`;

  return (
    <span
      className={`inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-[10px] font-medium border ${colorClasses}`}
      title={`Due: ${due.toLocaleDateString()}`}
    >
      <Calendar className="h-2.5 w-2.5" />
      <span>{label}</span>
    </span>
  );
}

export function TaskCard({
  task,
  index,
  status,
  onEdit,
  onDelete,
  onDuplicate,
  onArchive,
  onViewDetails,
  isOpen,
  selectionMode,
  isSelected,
  onToggleSelection,
  agentFlow,
  dimmed,
  usersMap,
}: TaskCardProps) {
  const handleClick = useCallback(() => {
    if (selectionMode && onToggleSelection) {
      onToggleSelection(task.id);
    } else {
      onViewDetails(task);
    }
  }, [task, onViewDetails, selectionMode, onToggleSelection]);

  const localRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!isOpen || !localRef.current) return;
    const el = localRef.current;
    requestAnimationFrame(() => {
      el.scrollIntoView({
        block: 'center',
        inline: 'nearest',
        behavior: 'smooth',
      });
    });
  }, [isOpen]);

  return (
    <KanbanCard
      key={task.id}
      id={task.id}
      name={task.title}
      index={index}
      parent={status}
      onClick={handleClick}
      isOpen={isOpen}
      forwardedRef={localRef}
      className={[
        dimmed ? 'opacity-60' : '',
        (task.priority && PRIORITY_BORDER_COLORS[task.priority]) || '',
      ].filter(Boolean).join(' ') || undefined}
    >
      <div className="flex flex-col gap-1.5 min-w-0">
        <div className="flex items-start gap-2 min-w-0">
          {/* Checkbox for selection mode */}
          {selectionMode && (
            <div
              className="pt-0.5"
              onClick={(e) => {
                e.stopPropagation();
                onToggleSelection?.(task.id);
              }}
            >
              <Checkbox
                checked={isSelected}
                onCheckedChange={() => onToggleSelection?.(task.id)}
              />
            </div>
          )}
          <h4 className="flex-1 min-w-0 line-clamp-2 font-light text-sm">
            {task.title}
          </h4>
          {/* Actions Menu */}
          <div
            onPointerDown={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            onClick={(e) => e.stopPropagation()}
          >
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 w-6 p-0 hover:bg-muted shrink-0"
                >
                  <MoreHorizontal className="h-3 w-3" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onClick={() => onEdit(task)}>
                  <Edit className="h-4 w-4 mr-2" />
                  Edit
                </DropdownMenuItem>
                {onDuplicate && (
                  <DropdownMenuItem onClick={() => onDuplicate(task)}>
                    <Copy className="h-4 w-4 mr-2" />
                    Duplicate
                  </DropdownMenuItem>
                )}
                {onArchive && (
                  <DropdownMenuItem onClick={() => onArchive(task)}>
                    <Archive className="h-4 w-4 mr-2" />
                    {task.archived_at ? 'Unarchive' : 'Archive'}
                  </DropdownMenuItem>
                )}
                <DropdownMenuItem
                  onClick={() => onDelete(task.id)}
                  className="text-destructive"
                >
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
        <div className="flex items-center flex-wrap gap-1">
          {/* Priority Badge */}
          <PriorityBadge priority={task.priority} />
          {/* In Progress Spinner */}
          {task.has_in_progress_attempt && (
            <Loader2 className="h-3 w-3 animate-spin text-blue-500" />
          )}
          {/* Merged Indicator */}
          {task.has_merged_attempt && (
            <CheckCircle className="h-3 w-3 text-green-500" />
          )}
          {/* Failed Indicator */}
          {task.last_attempt_failed && !task.has_merged_attempt && (
            <XCircle className="h-3 w-3 text-destructive" />
          )}
          {/* Agent Flow Status - compact */}
          {agentFlow && (
            <AgentFlowBadges flow={agentFlow} compact />
          )}
          {/* Execution Summary - compact */}
          {task.last_execution_summary && (
            <ExecutionSummaryInline summary={task.last_execution_summary} compact />
          )}
          {/* Collaborator Avatars */}
          {task.collaborators && task.collaborators.length > 0 && (
            <div className="flex -space-x-1" title={task.collaborators.map(c => `${c.actor_id} (${c.actor_type})`).join(', ')}>
              {task.collaborators.slice(0, 3).map((collaborator, idx) => (
                <div
                  key={`${collaborator.actor_id}-${idx}`}
                  className={`h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-medium border border-background ${
                    collaborator.actor_type === 'agent'
                      ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                      : 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                  }`}
                >
                  {collaborator.actor_type === 'agent'
                    ? <Bot className="h-2.5 w-2.5" />
                    : <User className="h-2.5 w-2.5" />
                  }
                </div>
              ))}
              {task.collaborators.length > 3 && (
                <div className="h-4 w-4 rounded-full flex items-center justify-center text-[8px] font-medium border border-background bg-muted text-muted-foreground">
                  +{task.collaborators.length - 3}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
      {/* Tag chips */}
      {task.tags && <TagChips tags={task.tags} maxVisible={2} size="xs" />}
      {/* Meta row: due date + description preview */}
      {(task.due_date || task.description) && (
        <div className="mt-1">
          {task.due_date && status !== 'done' && status !== 'cancelled' && (
            <div className="mb-1">
              <DueDateBadge dueDate={task.due_date} />
            </div>
          )}
          {task.description && (
            <p className="text-sm text-secondary-foreground break-words">
              {task.description.length > 130
                ? `${task.description.substring(0, 130)}...`
                : task.description}
            </p>
          )}
        </div>
      )}
      {task.screenshot && (
        <div className="mt-2">
          <img
            src={task.screenshot}
            alt="Task screenshot"
            className="max-h-32 rounded-md border object-contain w-full bg-muted"
          />
        </div>
      )}
      {!selectionMode && (
        <div className="mt-2 pt-2 border-t flex items-center justify-between gap-2">
          <div className="flex items-center gap-2 min-w-0">
            <TimeTrackerWidget taskId={task.id} compact />
            {task.assignee_id && (() => {
              const assignee = usersMap?.get(task.assignee_id);
              const displayName = assignee?.full_name || assignee?.username || assignee?.email || task.assignee_id;
              const initials = assignee?.full_name
                ? assignee.full_name.split(' ').map(n => n[0]).join('').toUpperCase().slice(0, 2)
                : (assignee?.username?.[0] || displayName[0] || '?').toUpperCase();
              return (
                <div className="flex items-center gap-1.5 min-w-0">
                  <div
                    className="h-5 w-5 rounded-full flex items-center justify-center text-[10px] font-medium bg-violet-100 text-violet-700 dark:bg-violet-900 dark:text-violet-300 border border-violet-200 dark:border-violet-800 shrink-0"
                    title={displayName}
                  >
                    {initials}
                  </div>
                  <span className="text-xs text-muted-foreground truncate">
                    {assignee?.full_name || assignee?.username || assignee?.email || 'Unassigned'}
                  </span>
                </div>
              );
            })()}
            {task.assigned_agent && (
              <div className="flex items-center gap-1 text-xs text-muted-foreground" title={`Agent: ${task.assigned_agent}`}>
                <Bot className="h-3 w-3 text-blue-500" />
                <span className="truncate max-w-[80px]">{task.assigned_agent}</span>
              </div>
            )}
          </div>
          {task.vibe_cost != null && task.vibe_cost > 0 && (
            <div
              className="flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800 text-amber-700 dark:text-amber-400 text-[10px] font-medium shrink-0"
              title={`${task.vibe_cost} VIBE${task.vibe_model ? ` (${task.vibe_model})` : ''}`}
            >
              <Zap className="h-2.5 w-2.5" />
              <span>{Number(task.vibe_cost)}</span>
              <span className="text-amber-500 dark:text-amber-500/70">VIBE</span>
            </div>
          )}
        </div>
      )}
    </KanbanCard>
  );
}
