import { useCallback, useState } from 'react';
import { Plus } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TaskWithAttemptStatus } from 'shared/types';

interface CalendarCellProps {
  date: Date;
  isCurrentMonth: boolean;
  isToday: boolean;
  tasks: TaskWithAttemptStatus[];
  onTaskClick?: (task: TaskWithAttemptStatus) => void;
  onDateClick?: (date: Date) => void;
  onDragStart?: (task: TaskWithAttemptStatus) => void;
  onDrop?: (date: Date) => void;
  getCategoryColor: (category?: string) => string;
}

interface TaskPillProps {
  task: TaskWithAttemptStatus;
  getCategoryColor: (category?: string) => string;
  onClick?: () => void;
  onDragStart?: () => void;
}

function TaskPill({ task, getCategoryColor, onClick, onDragStart }: TaskPillProps) {
  const category = (task.custom_properties as any)?.category;
  const platforms = (task.custom_properties as any)?.platforms as string[] | undefined;

  return (
    <div
      className={cn(
        'group flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] cursor-pointer',
        'bg-muted hover:bg-muted/80 transition-colors',
        'truncate max-w-full'
      )}
      onClick={onClick}
      draggable
      onDragStart={(e) => {
        e.dataTransfer.effectAllowed = 'move';
        onDragStart?.();
      }}
    >
      <div className={cn('h-1.5 w-1.5 rounded-full shrink-0', getCategoryColor(category))} />
      <span className="truncate">{task.title}</span>
      {platforms && platforms.length > 0 && (
        <span className="text-muted-foreground ml-auto shrink-0">
          {platforms.map((p) => getPlatformEmoji(p)).join('')}
        </span>
      )}
    </div>
  );
}

function getPlatformEmoji(platform: string): string {
  switch (platform.toLowerCase()) {
    case 'instagram':
      return 'IG';
    case 'linkedin':
      return 'LI';
    case 'twitter':
      return 'X';
    case 'tiktok':
      return 'TT';
    case 'facebook':
      return 'FB';
    default:
      return '';
  }
}

export function CalendarCell({
  date,
  isCurrentMonth,
  isToday,
  tasks,
  onTaskClick,
  onDateClick,
  onDragStart,
  onDrop,
  getCategoryColor,
}: CalendarCellProps) {
  const [isDragOver, setIsDragOver] = useState(false);
  const dayNumber = date.getDate();
  const maxVisibleTasks = 3;

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback(() => {
    setIsDragOver(false);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragOver(false);
      onDrop?.(date);
    },
    [date, onDrop]
  );

  return (
    <div
      className={cn(
        'min-h-[100px] p-1 bg-background',
        !isCurrentMonth && 'bg-muted/30',
        isToday && 'bg-primary/5',
        isDragOver && 'bg-primary/10 ring-1 ring-primary'
      )}
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      {/* Day number */}
      <div className="flex items-center justify-between mb-1">
        <button
          onClick={() => onDateClick?.(date)}
          className={cn(
            'h-6 w-6 flex items-center justify-center rounded-full text-xs font-medium',
            'hover:bg-muted transition-colors',
            !isCurrentMonth && 'text-muted-foreground',
            isToday && 'bg-primary text-primary-foreground hover:bg-primary/90'
          )}
        >
          {dayNumber}
        </button>
        {isCurrentMonth && (
          <button
            onClick={() => onDateClick?.(date)}
            className="opacity-0 group-hover:opacity-100 h-5 w-5 flex items-center justify-center rounded hover:bg-muted transition-all"
          >
            <Plus className="h-3 w-3 text-muted-foreground" />
          </button>
        )}
      </div>

      {/* Tasks */}
      <div className="space-y-0.5">
        {tasks.slice(0, maxVisibleTasks).map((task) => (
          <TaskPill
            key={task.id}
            task={task}
            getCategoryColor={getCategoryColor}
            onClick={() => onTaskClick?.(task)}
            onDragStart={() => onDragStart?.(task)}
          />
        ))}
        {tasks.length > maxVisibleTasks && (
          <button
            onClick={() => onDateClick?.(date)}
            className="text-[10px] text-muted-foreground hover:text-foreground px-1.5"
          >
            +{tasks.length - maxVisibleTasks} more
          </button>
        )}
      </div>
    </div>
  );
}
