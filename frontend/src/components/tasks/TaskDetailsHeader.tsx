import { Copy, Edit, Maximize2, Minimize2, MoreHorizontal,Trash2, X } from 'lucide-react';
import { memo } from 'react';
import type { TaskWithAttemptStatus } from 'shared/types';

import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { IconButton } from '@/components/ui/icon-button';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import { statusBoardColors, statusLabels } from '@/utils/status-labels';

import { Card } from '../ui/card';
import { TaskTitleDescription } from './TaskDetails/TaskTitleDescription';

interface TaskDetailsHeaderProps {
  task: TaskWithAttemptStatus;
  onClose: () => void;
  onEditTask?: (task: TaskWithAttemptStatus) => void;
  onDeleteTask?: (taskId: string) => void;
  onDuplicateTask?: (task: TaskWithAttemptStatus) => void;
  hideCloseButton?: boolean;
  isFullScreen?: boolean;
}

// backgroundColor: `hsl(var(${statusBoardColors[task.status]}) / 0.03)`,

function TaskDetailsHeader({
  task,
  onClose,
  onEditTask,
  onDeleteTask,
  onDuplicateTask,
  hideCloseButton = false,
  isFullScreen,
}: TaskDetailsHeaderProps) {
  const { toggleFullscreen } = useTaskViewManager();
  return (
    <div>
      <Card
        className="flex shrink-0 items-center gap-2 border-b border-dashed bg-background"
        style={{}}
      >
        <div className="p-3 flex flex-1 items-center truncate">
          <div
            className="h-2 w-2 rounded-full inline-block"
            style={{
              backgroundColor: `hsl(var(${statusBoardColors[task.status]}))`,
            }}
          />
          <p className="ml-2 text-sm">{statusLabels[task.status]}</p>
        </div>
        <div className="mr-3 flex items-center gap-1">
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => toggleFullscreen(!isFullScreen)}
                  aria-label={isFullScreen ? 'Exit fullscreen' : 'Expand to fullscreen'}
                >
                  {isFullScreen ? (
                    <Minimize2 className="h-4 w-4" />
                  ) : (
                    <Maximize2 className="h-4 w-4" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent>
                <p>
                  {isFullScreen
                    ? 'Exit fullscreen (f)'
                    : 'Fullscreen (f)'}
                </p>
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
          {(onEditTask || onDuplicateTask || onDeleteTask) && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <IconButton
                  variant="ghost"
                  icon={MoreHorizontal}
                  label="More options"
                />
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {onEditTask && (
                  <DropdownMenuItem onClick={() => onEditTask(task)}>
                    <Edit className="h-4 w-4 mr-2" />
                    Edit
                  </DropdownMenuItem>
                )}
                {onDuplicateTask && (
                  <DropdownMenuItem onClick={() => onDuplicateTask(task)}>
                    <Copy className="h-4 w-4 mr-2" />
                    Duplicate
                  </DropdownMenuItem>
                )}
                {onDeleteTask && (
                  <DropdownMenuItem
                    onClick={() => onDeleteTask(task.id)}
                    className="text-destructive"
                  >
                    <Trash2 className="h-4 w-4 mr-2" />
                    Delete
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          {!hideCloseButton && (
            <TooltipProvider>
              <Tooltip>
                <TooltipTrigger asChild>
                  <IconButton
                    variant="ghost" onClick={onClose}
                    icon={X}
                    label="Close"
                  />
                </TooltipTrigger>
                <TooltipContent>
                  <p>Close (Esc)</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          )}
        </div>
      </Card>

      {/* Title and Task Actions */}
      {!isFullScreen && (
        <div className="p-3 border-b border-dashed max-h-96 overflow-y-auto">
          <TaskTitleDescription task={task} />
        </div>
      )}
    </div>
  );
}

export default memo(TaskDetailsHeader);
