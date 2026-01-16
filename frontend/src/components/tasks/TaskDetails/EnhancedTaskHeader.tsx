import { useMemo } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Tooltip, TooltipContent, TooltipTrigger, TooltipProvider } from '@/components/ui/tooltip';
import {
  Terminal,
  FileText,
  Image,
  Video,
  Code,
  MoreHorizontal,
  Edit,
  Copy,
  Trash2,
  Maximize2,
  Minimize2,
  X,
  Bot,
  User,
  CheckCircle,
  XCircle,
  Loader2,
  Calendar,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TaskWithAttemptStatus } from 'shared/types';
import type { TaskCardMode } from '../EnhancedTaskCard';

interface EnhancedTaskHeaderProps {
  task: TaskWithAttemptStatus;
  mode: TaskCardMode;
  onEdit?: () => void;
  onDelete?: () => void;
  onDuplicate?: () => void;
  onClose?: () => void;
  onToggleFullscreen?: () => void;
  isFullscreen?: boolean;
  hideClose?: boolean;
}

// Mode display configuration
const modeConfig: Record<TaskCardMode, { icon: React.ReactNode; label: string; color: string }> = {
  terminal: {
    icon: <Terminal className="h-4 w-4" />,
    label: 'Code',
    color: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900 dark:text-emerald-300',
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
    color: 'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
  },
  compact: {
    icon: <Code className="h-4 w-4" />,
    label: 'Quick',
    color: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
  },
};

// Status configuration
const statusConfig: Record<string, { color: string; label: string }> = {
  todo: { color: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300', label: 'To Do' },
  inprogress: { color: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300', label: 'In Progress' },
  inreview: { color: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300', label: 'In Review' },
  done: { color: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300', label: 'Done' },
  cancelled: { color: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300', label: 'Cancelled' },
};

// Priority configuration
const priorityConfig: Record<string, { color: string; label: string }> = {
  critical: { color: 'bg-red-500 text-white', label: 'Critical' },
  high: { color: 'bg-orange-500 text-white', label: 'High' },
  medium: { color: 'bg-yellow-500 text-black', label: 'Medium' },
  low: { color: 'bg-gray-400 text-white', label: 'Low' },
};

export function EnhancedTaskHeader({
  task,
  mode,
  onEdit,
  onDelete,
  onDuplicate,
  onClose,
  onToggleFullscreen,
  isFullscreen,
  hideClose,
}: EnhancedTaskHeaderProps) {
  const modeInfo = modeConfig[mode];
  const statusInfo = statusConfig[task.status] || statusConfig.todo;
  const priorityInfo = priorityConfig[task.priority] || priorityConfig.medium;

  // Format due date
  const dueDateDisplay = useMemo(() => {
    if (!task.due_date) return null;
    const date = new Date(task.due_date);
    const now = new Date();
    const diffDays = Math.ceil((date.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

    if (diffDays < 0) return { text: 'Overdue', color: 'text-red-500' };
    if (diffDays === 0) return { text: 'Due today', color: 'text-orange-500' };
    if (diffDays === 1) return { text: 'Due tomorrow', color: 'text-yellow-500' };
    if (diffDays <= 7) return { text: `Due in ${diffDays} days`, color: 'text-blue-500' };
    return { text: date.toLocaleDateString(), color: 'text-muted-foreground' };
  }, [task.due_date]);

  // Count collaborators by type
  const collaboratorCounts = useMemo(() => {
    const counts = { agents: 0, humans: 0 };
    if (task.collaborators) {
      for (const c of task.collaborators) {
        if (c.actor_type === 'agent') counts.agents++;
        else counts.humans++;
      }
    }
    return counts;
  }, [task.collaborators]);

  return (
    <TooltipProvider>
    <div className="border-b bg-background/95 backdrop-blur sticky top-0 z-10">
      {/* Top row: Title and actions */}
      <div className="flex items-start gap-3 p-4">
        {/* Mode badge */}
        <Badge variant="outline" className={cn('shrink-0 gap-1', modeInfo.color)}>
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
          {onToggleFullscreen && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button variant="ghost" size="icon" onClick={onToggleFullscreen} className="h-8 w-8">
                  {isFullscreen ? (
                    <Minimize2 className="h-4 w-4" />
                  ) : (
                    <Maximize2 className="h-4 w-4" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                {isFullscreen ? 'Exit fullscreen' : 'Fullscreen'}
              </TooltipContent>
            </Tooltip>
          )}

          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon" className="h-8 w-8">
                <MoreHorizontal className="h-4 w-4" />
              </Button>
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
              {(onEdit || onDuplicate) && onDelete && <DropdownMenuSeparator />}
              {onDelete && (
                <DropdownMenuItem onClick={onDelete} className="text-destructive">
                  <Trash2 className="h-4 w-4 mr-2" />
                  Delete
                </DropdownMenuItem>
              )}
            </DropdownMenuContent>
          </DropdownMenu>

          {!hideClose && onClose && (
            <Button variant="ghost" size="icon" onClick={onClose} className="h-8 w-8">
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Bottom row: Metadata badges */}
      <div className="flex items-center gap-2 px-4 pb-3 flex-wrap">
        {/* Status */}
        <Badge variant="outline" className={cn('text-xs', statusInfo.color)}>
          {task.has_in_progress_attempt ? (
            <Loader2 className="h-3 w-3 mr-1 animate-spin" />
          ) : task.has_merged_attempt ? (
            <CheckCircle className="h-3 w-3 mr-1" />
          ) : task.last_attempt_failed ? (
            <XCircle className="h-3 w-3 mr-1" />
          ) : null}
          {statusInfo.label}
        </Badge>

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
              {collaboratorCounts.agents > 0 && `${collaboratorCounts.agents} agent${collaboratorCounts.agents > 1 ? 's' : ''}`}
              {collaboratorCounts.agents > 0 && collaboratorCounts.humans > 0 && ', '}
              {collaboratorCounts.humans > 0 && `${collaboratorCounts.humans} human${collaboratorCounts.humans > 1 ? 's' : ''}`}
            </TooltipContent>
          </Tooltip>
        )}

        {/* Due date */}
        {dueDateDisplay && (
          <Badge variant="outline" className={cn('text-xs gap-1', dueDateDisplay.color)}>
            <Calendar className="h-3 w-3" />
            {dueDateDisplay.text}
          </Badge>
        )}

        {/* Tags */}
        {task.tags && task.tags.split(',').slice(0, 3).map((tag, i) => (
          <Badge key={i} variant="secondary" className="text-xs">
            {tag.trim()}
          </Badge>
        ))}
      </div>
    </div>
    </TooltipProvider>
  );
}
