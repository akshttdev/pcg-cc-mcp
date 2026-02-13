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
} from 'lucide-react';
import { TimeTrackerWidget } from '@/components/time-tracking/TimeTrackerWidget';
import { AgentFlowBadges } from './AgentFlowBadges';
import { ExecutionSummaryInline } from './ExecutionSummaryInline';
import type { TaskWithAttemptStatus } from 'shared/types';
import type { AgentFlow } from '@/lib/api';

type Task = TaskWithAttemptStatus;

interface TaskCardProps {
  task: Task;
  index: number;
  status: string;
  onEdit: (task: Task) => void;
  onDelete: (taskId: string) => void;
  onDuplicate?: (task: Task) => void;
  onViewDetails: (task: Task) => void;
  isOpen?: boolean;
  selectionMode?: boolean;
  isSelected?: boolean;
  onToggleSelection?: (taskId: string) => void;
  agentFlow?: AgentFlow;
}

export function TaskCard({
  task,
  index,
  status,
  onEdit,
  onDelete,
  onDuplicate,
  onViewDetails,
  isOpen,
  selectionMode,
  isSelected,
  onToggleSelection,
  agentFlow,
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
    >
      <div className="flex flex-1 gap-2 items-center min-w-0">
        {/* Checkbox for selection mode */}
        {selectionMode && (
          <div
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
        <div className="flex items-center space-x-1">
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
                  className="h-6 w-6 p-0 hover:bg-muted"
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
      </div>
      {task.description && (
        <p className="flex-1 text-sm text-secondary-foreground break-words">
          {task.description.length > 130
            ? `${task.description.substring(0, 130)}...`
            : task.description}
        </p>
      )}
      {!selectionMode && (
        <div className="mt-2 pt-2 border-t flex items-center justify-between gap-2">
          <TimeTrackerWidget taskId={task.id} compact />
          {task.vibe_cost != null && task.vibe_cost > 0 && (
            <div
              className="flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-amber-50 dark:bg-amber-950/40 border border-amber-200 dark:border-amber-800 text-amber-700 dark:text-amber-400 text-[10px] font-medium shrink-0"
              title={`${task.vibe_cost} VIBE${task.vibe_model ? ` (${task.vibe_model})` : ''}`}
            >
              <Zap className="h-2.5 w-2.5" />
              <span>{task.vibe_cost}</span>
              <span className="text-amber-500 dark:text-amber-500/70">VIBE</span>
            </div>
          )}
        </div>
      )}
    </KanbanCard>
  );
}
