import { useMemo, useState } from 'react';
import type { TaskWithAttemptStatus } from 'shared/types';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  ChevronLeft,
  ChevronRight,
  Calendar,
  Clock,
  Circle,
  CheckCircle2
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { format, startOfMonth, endOfMonth, eachDayOfInterval, isSameDay, addMonths, subMonths, parseISO } from 'date-fns';

interface TimelineViewProps {
  tasks: TaskWithAttemptStatus[];
  onTaskClick?: (task: TaskWithAttemptStatus) => void;
}

const STATUS_COLORS = {
  todo: 'bg-slate-500',
  inprogress: 'bg-blue-500',
  inreview: 'bg-yellow-500',
  done: 'bg-green-500',
  cancelled: 'bg-red-500',
};

const STATUS_LABELS = {
  todo: 'To Do',
  inprogress: 'In Progress',
  inreview: 'In Review',
  done: 'Done',
  cancelled: 'Cancelled',
};

export function TimelineView({ tasks, onTaskClick }: TimelineViewProps) {
  const [currentMonth, setCurrentMonth] = useState(new Date());

  // Get month boundaries and days
  const monthStart = startOfMonth(currentMonth);
  const monthEnd = endOfMonth(currentMonth);
  const days = eachDayOfInterval({ start: monthStart, end: monthEnd });

  // Group tasks by date (created or updated)
  const tasksByDate = useMemo(() => {
    const grouped: Record<string, TaskWithAttemptStatus[]> = {};

    tasks.forEach((task) => {
      const createdDate = format(parseISO(task.created_at), 'yyyy-MM-dd');
      const updatedDate = format(parseISO(task.updated_at), 'yyyy-MM-dd');

      // Add to created date
      if (!grouped[createdDate]) {
        grouped[createdDate] = [];
      }
      grouped[createdDate].push(task);

      // Also add to updated date if different
      if (createdDate !== updatedDate) {
        if (!grouped[updatedDate]) {
          grouped[updatedDate] = [];
        }
        // Only add if not already in that date
        if (!grouped[updatedDate].some(t => t.id === task.id)) {
          grouped[updatedDate].push(task);
        }
      }
    });

    return grouped;
  }, [tasks]);

  // Sort tasks by status (in progress first, then others)
  const sortedTasks = useMemo(() => {
    const statusOrder = { inprogress: 0, inreview: 1, todo: 2, done: 3, cancelled: 4 };
    return [...tasks].sort((a, b) => {
      const orderDiff = statusOrder[a.status] - statusOrder[b.status];
      if (orderDiff !== 0) return orderDiff;
      // If same status, sort by updated_at (most recent first)
      return new Date(b.updated_at).getTime() - new Date(a.updated_at).getTime();
    });
  }, [tasks]);

  const handlePreviousMonth = () => {
    setCurrentMonth(subMonths(currentMonth, 1));
  };

  const handleNextMonth = () => {
    setCurrentMonth(addMonths(currentMonth, 1));
  };

  const handleToday = () => {
    setCurrentMonth(new Date());
  };

  const today = new Date();

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center gap-2">
          <Clock className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Timeline View</h2>
        </div>

        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={handleToday}>
            Today
          </Button>
          <Button variant="ghost" size="icon" onClick={handlePreviousMonth} title="Previous month">
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <div className="min-w-[140px] text-center">
            <span className="text-sm font-medium">
              {format(currentMonth, 'MMMM yyyy')}
            </span>
          </div>
          <Button variant="ghost" size="icon" onClick={handleNextMonth} title="Next month">
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Timeline Content */}
      <div className="flex-1 overflow-auto p-4">
        <div className="grid grid-cols-2 gap-6">
          {/* Left: Calendar Timeline */}
          <div className="space-y-4">
            <div className="flex items-center gap-2 mb-4">
              <Calendar className="h-4 w-4 text-muted-foreground" />
              <h3 className="text-sm font-medium">Calendar</h3>
            </div>

            <div className="space-y-2">
              {days.map((day) => {
                const dateKey = format(day, 'yyyy-MM-dd');
                const dayTasks = tasksByDate[dateKey] || [];
                const isToday = isSameDay(day, today);

                return (
                  <div
                    key={dateKey}
                    className={cn(
                      'flex items-start gap-3 p-3 rounded-lg border transition-colors',
                      isToday && 'bg-accent border-accent-foreground/20',
                      dayTasks.length > 0 ? 'border-border' : 'border-transparent opacity-40'
                    )}
                  >
                    {/* Date indicator */}
                    <div className="flex flex-col items-center min-w-[60px] pt-1">
                      <span className={cn(
                        'text-xs font-medium text-muted-foreground uppercase',
                        isToday && 'text-accent-foreground'
                      )}>
                        {format(day, 'EEE')}
                      </span>
                      <span className={cn(
                        'text-2xl font-bold',
                        isToday ? 'text-accent-foreground' : 'text-foreground'
                      )}>
                        {format(day, 'd')}
                      </span>
                    </div>

                    {/* Tasks for this day */}
                    <div className="flex-1 space-y-1">
                      {dayTasks.length > 0 ? (
                        dayTasks.map((task) => (
                          <button
                            key={task.id}
                            onClick={() => onTaskClick?.(task)}
                            className={cn(
                              'w-full text-left px-2 py-1 rounded text-sm transition-colors',
                              'hover:bg-accent/50'
                            )}
                          >
                            <div className="flex items-center gap-2">
                              <div className={cn('w-2 h-2 rounded-full', STATUS_COLORS[task.status])} />
                              <span className="truncate flex-1">{task.title}</span>
                            </div>
                          </button>
                        ))
                      ) : (
                        <span className="text-xs text-muted-foreground italic">No tasks</span>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Right: Task List (sorted by status) */}
          <div className="space-y-4">
            <div className="flex items-center gap-2 mb-4">
              <Circle className="h-4 w-4 text-muted-foreground" />
              <h3 className="text-sm font-medium">All Tasks</h3>
              <Badge variant="secondary" className="ml-auto">
                {tasks.length} tasks
              </Badge>
            </div>

            <div className="space-y-2">
              {sortedTasks.map((task) => {
                const createdDate = parseISO(task.created_at);
                const updatedDate = parseISO(task.updated_at);
                const isInCurrentMonth =
                  (createdDate >= monthStart && createdDate <= monthEnd) ||
                  (updatedDate >= monthStart && updatedDate <= monthEnd);

                return (
                  <Card
                    key={task.id}
                    className={cn(
                      'p-4 cursor-pointer transition-all hover:shadow-md',
                      !isInCurrentMonth && 'opacity-40'
                    )}
                    onClick={() => onTaskClick?.(task)}
                  >
                    <div className="flex items-start gap-3">
                      {/* Status indicator */}
                      <div className={cn('w-3 h-3 rounded-full mt-1 flex-shrink-0', STATUS_COLORS[task.status])} />

                      {/* Task content */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 mb-1">
                          <h4 className="font-medium text-sm truncate">{task.title}</h4>
                          {task.status === 'done' && (
                            <CheckCircle2 className="h-4 w-4 text-green-500 flex-shrink-0" />
                          )}
                        </div>

                        {task.description && (
                          <p className="text-xs text-muted-foreground line-clamp-2 mb-2">
                            {task.description}
                          </p>
                        )}

                        <div className="flex items-center gap-3 text-xs text-muted-foreground">
                          <Badge variant="outline" className="text-xs">
                            {STATUS_LABELS[task.status]}
                          </Badge>
                          <span>Created {format(createdDate, 'MMM d')}</span>
                          {format(createdDate, 'yyyy-MM-dd') !== format(updatedDate, 'yyyy-MM-dd') && (
                            <span>Updated {format(updatedDate, 'MMM d')}</span>
                          )}
                        </div>
                      </div>
                    </div>
                  </Card>
                );
              })}

              {sortedTasks.length === 0 && (
                <Card className="p-8">
                  <div className="text-center text-muted-foreground">
                    <Calendar className="h-8 w-8 mx-auto mb-2 opacity-50" />
                    <p className="text-sm">No tasks in this project yet</p>
                  </div>
                </Card>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
