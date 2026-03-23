import { useMemo, useState } from 'react';
import type { TaskWithAttemptStatus } from 'shared/types';
import { Card } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  ChevronLeft,
  ChevronRight,
  Calendar as CalendarIcon,
  Plus,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  format,
  startOfMonth,
  endOfMonth,
  startOfWeek,
  endOfWeek,
  eachDayOfInterval,
  isSameMonth,
  isToday,
  addMonths,
  subMonths,
  parseISO,
} from 'date-fns';

interface CalendarViewProps {
  tasks: TaskWithAttemptStatus[];
  onTaskClick?: (task: TaskWithAttemptStatus) => void;
  onCreateTask?: (date: Date) => void;
}

const STATUS_COLORS = {
  todo: 'bg-slate-500 hover:bg-slate-600',
  inprogress: 'bg-blue-500 hover:bg-blue-600',
  inreview: 'bg-yellow-500 hover:bg-yellow-600',
  done: 'bg-green-500 hover:bg-green-600',
  cancelled: 'bg-red-500 hover:bg-red-600',
};

const STATUS_DOT_COLORS = {
  todo: 'bg-slate-500',
  inprogress: 'bg-blue-500',
  inreview: 'bg-yellow-500',
  done: 'bg-green-500',
  cancelled: 'bg-red-500',
};

export function CalendarView({ tasks, onTaskClick, onCreateTask }: CalendarViewProps) {
  const [currentMonth, setCurrentMonth] = useState(new Date());

  // Get calendar boundaries
  const monthStart = startOfMonth(currentMonth);
  const monthEnd = endOfMonth(currentMonth);
  const calendarStart = startOfWeek(monthStart, { weekStartsOn: 0 }); // Sunday
  const calendarEnd = endOfWeek(monthEnd, { weekStartsOn: 0 });
  const calendarDays = eachDayOfInterval({ start: calendarStart, end: calendarEnd });

  // Group tasks by date
  const tasksByDate = useMemo(() => {
    const grouped: Record<string, TaskWithAttemptStatus[]> = {};

    tasks.forEach((task) => {
      // Use created_at as the task's date
      const taskDate = format(parseISO(task.created_at), 'yyyy-MM-dd');

      if (!grouped[taskDate]) {
        grouped[taskDate] = [];
      }
      grouped[taskDate].push(task);
    });

    // Sort tasks within each date by status priority
    Object.keys(grouped).forEach((date) => {
      grouped[date].sort((a, b) => {
        const statusOrder = {
          inprogress: 0,
          inreview: 1,
          todo: 2,
          done: 3,
          cancelled: 4,
        };
        return statusOrder[a.status] - statusOrder[b.status];
      });
    });

    return grouped;
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

  const weekDays = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b">
        <div className="flex items-center gap-2">
          <CalendarIcon className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Calendar View</h2>
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

      {/* Calendar Grid */}
      <div className="flex-1 overflow-auto p-4">
        <div className="min-w-[900px]">
          {/* Week day headers */}
          <div className="grid grid-cols-7 gap-2 mb-2">
            {weekDays.map((day) => (
              <div
                key={day}
                className="text-center text-xs font-medium text-muted-foreground py-2"
              >
                {day}
              </div>
            ))}
          </div>

          {/* Calendar days */}
          <div className="grid grid-cols-7 gap-2 auto-rows-fr">
            {calendarDays.map((day) => {
              const dateKey = format(day, 'yyyy-MM-dd');
              const dayTasks = tasksByDate[dateKey] || [];
              const isCurrentMonth = isSameMonth(day, currentMonth);
              const isDayToday = isToday(day);
              const dayNumber = format(day, 'd');

              return (
                <Card
                  key={dateKey}
                  className={cn(
                    'min-h-[120px] p-2 transition-all',
                    !isCurrentMonth && 'opacity-40 bg-muted/20',
                    isDayToday && 'ring-2 ring-primary',
                    dayTasks.length > 0 && 'hover:shadow-md'
                  )}
                >
                  {/* Day header */}
                  <div className="flex items-center justify-between mb-2">
                    <span
                      className={cn(
                        'text-sm font-medium',
                        isDayToday
                          ? 'bg-primary text-primary-foreground rounded-full w-6 h-6 flex items-center justify-center'
                          : 'text-muted-foreground'
                      )}
                    >
                      {dayNumber}
                    </span>
                    {onCreateTask && (
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-5 w-5 opacity-0 hover:opacity-100 transition-opacity"
                        onClick={() => onCreateTask(day)}
                        title="Add task"
                      >
                        <Plus className="h-3 w-3" />
                      </Button>
                    )}
                  </div>

                  {/* Tasks for this day */}
                  <div className="space-y-1">
                    {dayTasks.slice(0, 3).map((task) => (
                      <button
                        key={task.id}
                        onClick={() => onTaskClick?.(task)}
                        className={cn(
                          'w-full text-left px-2 py-1 rounded text-xs transition-colors',
                          STATUS_COLORS[task.status],
                          'text-white truncate'
                        )}
                        title={task.title}
                      >
                        {task.title}
                      </button>
                    ))}

                    {/* Show "+X more" if there are more than 3 tasks */}
                    {dayTasks.length > 3 && (
                      <div className="text-xs text-muted-foreground pl-2">
                        +{dayTasks.length - 3} more
                      </div>
                    )}

                    {/* Status dots for remaining tasks */}
                    {dayTasks.length > 3 && (
                      <div className="flex gap-1 pl-2 pt-1">
                        {dayTasks.slice(3, 8).map((task) => (
                          <div
                            key={task.id}
                            className={cn(
                              'w-2 h-2 rounded-full',
                              STATUS_DOT_COLORS[task.status]
                            )}
                            title={task.title}
                          />
                        ))}
                        {dayTasks.length > 8 && (
                          <span className="text-xs text-muted-foreground">
                            +{dayTasks.length - 8}
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                </Card>
              );
            })}
          </div>
        </div>
      </div>

      {/* Legend */}
      <div className="flex items-center justify-center gap-4 p-4 border-t text-xs">
        <span className="text-muted-foreground font-medium">Status:</span>
        {Object.entries(STATUS_COLORS).map(([status, colorClass]) => (
          <div key={status} className="flex items-center gap-1">
            <div className={cn('w-3 h-3 rounded', colorClass)} />
            <span className="capitalize">{status.replace('inprogress', 'in progress').replace('inreview', 'in review')}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
