import { useCallback } from 'react';
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
  Zap,
} from 'lucide-react';
import { TimeTrackerWidget } from '@/components/time-tracking/TimeTrackerWidget';
import { AgentFlowBadges } from './AgentFlowBadges';
import { ExecutionSummaryInline } from './ExecutionSummaryInline';
import { TagChips } from '@/components/ui/tag-chips';
import {
  PriorityBadge,
  PRIORITY_BORDER_COLORS,
  DueDateBadge,
  CollaboratorAvatars,
  useResolvedAssignee,
  useResolvedAgent,
  useScrollIntoView,
} from './task-card-parts';
import type { AgentFlow, UserListItem, TaskWithArchive } from '@/lib/api';
import type { AgentWithParsedFields } from 'shared/types';

type Task = TaskWithArchive;

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
  agentsMap?: Map<string, AgentWithParsedFields>;
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
  agentsMap,
}: TaskCardProps) {
  const handleClick = useCallback(() => {
    if (selectionMode && onToggleSelection) {
      onToggleSelection(task.id);
    } else {
      onViewDetails(task);
    }
  }, [task, onViewDetails, selectionMode, onToggleSelection]);

  const localRef = useScrollIntoView(isOpen);
  const assignee = useResolvedAssignee(task.assignee_id, usersMap);
  const resolvedAgent = useResolvedAgent(task.assigned_agent, agentsMap);

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
          <PriorityBadge priority={task.priority} />
          {task.has_in_progress_attempt && (
            <Loader2 className="h-3 w-3 animate-spin text-blue-500" />
          )}
          {task.has_merged_attempt && (
            <CheckCircle className="h-3 w-3 text-green-500" />
          )}
          {task.last_attempt_failed && !task.has_merged_attempt && (
            <XCircle className="h-3 w-3 text-destructive" />
          )}
          {agentFlow && (
            <AgentFlowBadges flow={agentFlow} compact />
          )}
          {task.last_execution_summary && (
            <ExecutionSummaryInline summary={task.last_execution_summary} compact />
          )}
          {task.collaborators && task.collaborators.length > 0 && (
            <CollaboratorAvatars
              collaborators={task.collaborators}
              usersMap={usersMap}
              agentsMap={agentsMap}
            />
          )}
        </div>
      </div>
      {task.tags && <TagChips tags={task.tags} maxVisible={2} size="xs" />}
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
            {assignee && (
              <div className="flex items-center gap-1.5 min-w-0">
                <div
                  className="h-5 w-5 rounded-full flex items-center justify-center text-[10px] font-medium bg-violet-100 text-violet-700 dark:bg-violet-900 dark:text-violet-300 border border-violet-200 dark:border-violet-800 shrink-0"
                  title={assignee.displayName}
                >
                  {assignee.initials}
                </div>
                <span className="text-xs text-muted-foreground truncate">
                  {assignee.displayName}
                </span>
              </div>
            )}
            {resolvedAgent && (
              <div className="flex items-center gap-1 text-xs text-muted-foreground" title={resolvedAgent.tooltip}>
                <Bot className="h-3 w-3 text-blue-500" />
                <span className="truncate max-w-[80px]">{resolvedAgent.displayName}</span>
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
