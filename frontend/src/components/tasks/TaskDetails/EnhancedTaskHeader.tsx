import { useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeftFromLine,
  ArrowRightFromLine,
  Bot,
  Calendar,
  CheckCircle,
  Code,
  Copy,
  Edit,
  FileText,
  Image,
  Loader2,
  MoreHorizontal,
  Terminal,
  Trash2,
  User,
  Video,
  X,
  XCircle,
} from 'lucide-react';
import { useMemo, useState } from 'react';
import type { TaskStatus, TaskWithAttemptStatus } from 'shared/types';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { tasksApi } from '@/lib/api';
import { taskKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';

import type { TaskCardMode } from '../EnhancedTaskCard';

interface EnhancedTaskHeaderProps {
  task: TaskWithAttemptStatus;
  mode: TaskCardMode;
  onEdit?: () => void;
  onDelete?: () => void;
  onDuplicate?: () => void;
  onClose?: () => void;
  onToggleExpand?: () => void;
  isExpanded?: boolean;
  hideClose?: boolean;
  onStatusChange?: (newStatus: string) => void;
}

// Mode display configuration
const modeConfig: Record<
  TaskCardMode,
  { icon: React.ReactNode; label: string; color: string }
> = {
  terminal: {
    icon: <Terminal className="h-4 w-4" />,
    label: 'Code',
    color:
      'bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300',
  },
  visual: {
    icon: <Image className="h-4 w-4" />,
    label: 'Visual',
    color: 'bg-pink-100 text-pink-700 dark:bg-pink-900 dark:text-pink-300',
  },
  document: {
    icon: <FileText className="h-4 w-4" />,
    label: 'Document',
    color: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300',
  },
  media: {
    icon: <Video className="h-4 w-4" />,
    label: 'Media',
    color:
      'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
  },
  compact: {
    icon: <Code className="h-4 w-4" />,
    label: 'Quick',
    color: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
  },
};

// Status configuration
const statusConfig: Record<
  string,
  { color: string; label: string; dot: string }
> = {
  todo: {
    color: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
    label: 'To Do',
    dot: 'bg-gray-400 dark:bg-gray-500',
  },
  inprogress: {
    color: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300',
    label: 'In Progress',
    dot: 'bg-blue-500 dark:bg-blue-400',
  },
  inreview: {
    color:
      'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300',
    label: 'In Review',
    dot: 'bg-yellow-500 dark:bg-yellow-400',
  },
  done: {
    color: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
    label: 'Done',
    dot: 'bg-green-500 dark:bg-green-400',
  },
  cancelled: {
    color: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300',
    label: 'Cancelled',
    dot: 'bg-red-500 dark:bg-red-400',
  },
};

// Priority configuration
const priorityConfig: Record<string, { color: string; label: string }> = {
  critical: { color: 'bg-red-500 text-white', label: 'Critical' },
  high: { color: 'bg-orange-500 text-white', label: 'High' },
  medium: { color: 'bg-yellow-500 text-black', label: 'Medium' },
  low: { color: 'bg-gray-400 text-white', label: 'Low' },
};

// Status options for dropdown
const STATUS_OPTIONS = [
  { value: 'todo', label: 'To Do' },
  { value: 'inprogress', label: 'In Progress' },
  { value: 'inreview', label: 'In Review' },
  { value: 'done', label: 'Done' },
  { value: 'cancelled', label: 'Cancelled' },
];

export function EnhancedTaskHeader({
  task,
  mode,
  onEdit,
  onDelete,
  onDuplicate,
  onClose,
  onToggleExpand,
  isExpanded,
  hideClose,
  onStatusChange,
}: EnhancedTaskHeaderProps) {
  const queryClient = useQueryClient();
  const [updatingStatus, setUpdatingStatus] = useState(false);
  const modeInfo = modeConfig[mode];
  const statusInfo = statusConfig[task.status] || statusConfig.todo;
  const priorityInfo = priorityConfig[task.priority] || priorityConfig.medium;

  // Format due date
  const dueDateDisplay = useMemo(() => {
    if (!task.due_date) return null;
    const date = new Date(task.due_date);
    const now = new Date();
    const diffDays = Math.ceil(
      (date.getTime() - now.getTime()) / (1000 * 60 * 60 * 24)
    );

    if (diffDays < 0) return { text: 'Overdue', color: 'text-red-500' };
    if (diffDays === 0) return { text: 'Due today', color: 'text-orange-500' };
    if (diffDays === 1)
      return { text: 'Due tomorrow', color: 'text-yellow-500' };
    if (diffDays <= 7)
      return { text: `Due in ${diffDays} days`, color: 'text-blue-500' };
    return { text: date.toLocaleDateString(), color: 'text-muted-foreground' };
  }, [task.due_date]);

  // Count collaborators by type
  const collaboratorCounts = useMemo(() => {
    const counts = { agents: 0, humans: 0 };
    if (task.parsed_collaborators) {
      for (const c of task.parsed_collaborators) {
        if (c.actor_type === 'agent') counts.agents++;
        else counts.humans++;
      }
    }
    return counts;
  }, [task.parsed_collaborators]);

  return (
    <TooltipProvider>
      <div className="border-b bg-background/95 backdrop-blur sticky top-0 z-10">
        {/* Top row: Title and actions */}
        <div className="flex items-start gap-3 p-4">
          {/* Mode badge */}
          <Badge
            variant="outline"
            className={cn('shrink-0 gap-1', modeInfo.color)}
          >
            {modeInfo.icon}
            <span className="text-xs">{modeInfo.label}</span>
          </Badge>

          {/* Title and description */}
          <div className="flex-1 min-w-0">
            <h2 className="text-lg font-semibold truncate">{task.title}</h2>
            {task.description && (
              <p className="text-sm text-muted-foreground line-clamp-2 mt-0.5">
                {task.description}
              </p>
            )}
          </div>

          {/* Action buttons */}
          <div className="flex items-center gap-1 shrink-0">
            {onToggleExpand && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={onToggleExpand}
                    className="h-8 w-8"
                  >
                    {isExpanded ? (
                      <ArrowRightFromLine className="h-4 w-4" />
                    ) : (
                      <ArrowLeftFromLine className="h-4 w-4" />
                    )}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  {isExpanded ? 'Collapse panel' : 'Expand panel'}
                </TooltipContent>
              </Tooltip>
            )}

            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <IconButton
                  variant="ghost" className="h-8 w-8"
                  icon={MoreHorizontal}
                  label="More options"
                />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {onEdit && (
                  <DropdownMenuItem onClick={onEdit}>
                    <Edit className="h-4 w-4 mr-2" />
                    Edit
                  </DropdownMenuItem>
                )}
                {onDuplicate && (
                  <DropdownMenuItem onClick={onDuplicate}>
                    <Copy className="h-4 w-4 mr-2" />
                    Duplicate
                  </DropdownMenuItem>
                )}
                {(onEdit || onDuplicate) && onDelete && (
                  <DropdownMenuSeparator />
                )}
                {onDelete && (
                  <DropdownMenuItem
                    onClick={onDelete}
                    className="text-destructive"
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    Delete
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>

            {!hideClose && onClose && (
              <Tooltip>
                <TooltipTrigger asChild>
                  <IconButton
                    variant="ghost" onClick={onClose}
                    className="h-8 w-8"
                    icon={X}
                    label="Close"
                  />
                </TooltipTrigger>
                <TooltipContent>Close (Esc)</TooltipContent>
              </Tooltip>
            )}
          </div>
        </div>

        {/* Bottom row: Metadata badges */}
        <div className="flex items-center gap-2 px-4 pb-3 flex-wrap">
          {/* Status dropdown */}
          <Select
            value={task.status}
            onValueChange={async (newStatus) => {
              if (newStatus === task.status) return;
              setUpdatingStatus(true);
              try {
                await tasksApi.update(task.id, {
                  status: newStatus as TaskStatus,
                });
                queryClient.invalidateQueries({ queryKey: taskKeys.all });
                toast.success(
                  `Status changed to ${statusConfig[newStatus]?.label || newStatus}`
                );
                onStatusChange?.(newStatus);
              } catch (err) {
                toast.error('Failed to update status');
              } finally {
                setUpdatingStatus(false);
              }
            }}
            disabled={updatingStatus}
          >
            <SelectTrigger
              className={cn(
                'h-7 text-xs gap-1 border-0 w-auto',
                statusInfo.color
              )}
            >
              {updatingStatus ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : task.has_in_progress_attempt ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : task.has_merged_attempt ? (
                <CheckCircle className="h-3 w-3" />
              ) : task.last_attempt_failed ? (
                <XCircle className="h-3 w-3" />
              ) : null}
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STATUS_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  <span className="flex items-center gap-1.5">
                    <span
                      className={cn(
                        'h-2 w-2 rounded-full',
                        statusConfig[opt.value]?.dot || 'bg-gray-300'
                      )}
                    />
                    {opt.label}
                  </span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Priority */}
          <Badge className={cn('text-xs', priorityInfo.color)}>
            {priorityInfo.label}
          </Badge>

          {/* Assigned agent */}
          {task.assigned_agent && (
            <Badge variant="outline" className="text-xs gap-1">
              <Bot className="h-3 w-3" />
              {task.assigned_agent}
            </Badge>
          )}

          {/* Collaborators */}
          {(collaboratorCounts.agents > 0 || collaboratorCounts.humans > 0) && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Badge variant="outline" className="text-xs gap-1.5">
                  {collaboratorCounts.agents > 0 && (
                    <span className="flex items-center gap-0.5">
                      <Bot className="h-3 w-3 text-blue-500" />
                      {collaboratorCounts.agents}
                    </span>
                  )}
                  {collaboratorCounts.humans > 0 && (
                    <span className="flex items-center gap-0.5">
                      <User className="h-3 w-3 text-green-500" />
                      {collaboratorCounts.humans}
                    </span>
                  )}
                </Badge>
              </TooltipTrigger>
              <TooltipContent>
                {collaboratorCounts.agents > 0 &&
                  `${collaboratorCounts.agents} agent${collaboratorCounts.agents > 1 ? 's' : ''}`}
                {collaboratorCounts.agents > 0 &&
                  collaboratorCounts.humans > 0 &&
                  ', '}
                {collaboratorCounts.humans > 0 &&
                  `${collaboratorCounts.humans} human${collaboratorCounts.humans > 1 ? 's' : ''}`}
              </TooltipContent>
            </Tooltip>
          )}

          {/* Due date */}
          {dueDateDisplay && (
            <Badge
              variant="outline"
              className={cn('text-xs gap-1', dueDateDisplay.color)}
            >
              <Calendar className="h-3 w-3" />
              {dueDateDisplay.text}
            </Badge>
          )}

          {/* Tags */}
          {task.tags &&
            (() => {
              try {
                const parsed = JSON.parse(task.tags) as string[];
                return parsed.slice(0, 3).map((tag, i) => (
                  <Badge key={i} variant="secondary" className="text-xs">
                    {tag.trim()}
                  </Badge>
                ));
              } catch {
                return task.tags
                  .split(',')
                  .slice(0, 3)
                  .map((tag, i) => (
                    <Badge key={i} variant="secondary" className="text-xs">
                      {tag.trim()}
                    </Badge>
                  ));
              }
            })()}
        </div>
      </div>
    </TooltipProvider>
  );
}
