import { useState, useMemo, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { TaskWithAttemptStatus } from 'shared/types';
import { CalendarCell } from './CalendarCell';

interface CalendarViewProps {
  tasks: TaskWithAttemptStatus[];
  onTaskClick?: (task: TaskWithAttemptStatus) => void;
  onDateClick?: (date: Date) => void;
  onTaskDrop?: (task: TaskWithAttemptStatus, newDate: Date) => void;
  className?: string;
}

const DAYS_OF_WEEK = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December'
];

interface CalendarDay {
  date: Date;
  isCurrentMonth: boolean;
  isToday: boolean;
  tasks: TaskWithAttemptStatus[];
}

function getCalendarDays(year: number, month: number, tasks: TaskWithAttemptStatus[]): CalendarDay[] {
  const firstDay = new Date(year, month, 1);
  const lastDay = new Date(year, month + 1, 0);
  const today = new Date();
  today.setHours(0, 0, 0, 0);

  const days: CalendarDay[] = [];

  // Add days from previous month
  const firstDayOfWeek = firstDay.getDay();
  for (let i = firstDayOfWeek - 1; i >= 0; i--) {
    const date = new Date(year, month, -i);
    days.push({
      date,
      isCurrentMonth: false,
      isToday: date.getTime() === today.getTime(),
      tasks: getTasksForDate(tasks, date),
    });
  }

  // Add days of current month
  for (let day = 1; day <= lastDay.getDate(); day++) {
    const date = new Date(year, month, day);
    days.push({
      date,
      isCurrentMonth: true,
      isToday: date.getTime() === today.getTime(),
      tasks: getTasksForDate(tasks, date),
    });
  }

  // Add days from next month to complete the grid
  const remainingDays = 42 - days.length; // 6 weeks * 7 days
  for (let day = 1; day <= remainingDays; day++) {
    const date = new Date(year, month + 1, day);
    days.push({
      date,
      isCurrentMonth: false,
      isToday: date.getTime() === today.getTime(),
      tasks: getTasksForDate(tasks, date),
    });
  }

  return days;
}

function getTasksForDate(tasks: TaskWithAttemptStatus[], date: Date): TaskWithAttemptStatus[] {
  const dateStr = date.toISOString().split('T')[0];
  return tasks.filter((task) => {
    if (!task.due_date) return false;
    const taskDate = new Date(task.due_date).toISOString().split('T')[0];
    return taskDate === dateStr;
  });
}

function getCategoryColor(category?: string): string {
  switch (category?.toLowerCase()) {
    case 'events':
      return 'bg-purple-500';
    case 'drinks':
      return 'bg-amber-500';
    case 'community':
      return 'bg-blue-500';
    case 'vibe':
      return 'bg-pink-500';
    case 'promotion':
      return 'bg-green-500';
    default:
      return 'bg-gray-500';
  }
}

export function CalendarView({
  tasks,
  onTaskClick,
  onDateClick,
  onTaskDrop,
  className,
}: CalendarViewProps) {
  const [currentDate, setCurrentDate] = useState(new Date());
  const [draggedTask, setDraggedTask] = useState<TaskWithAttemptStatus | null>(null);

  const year = currentDate.getFullYear();
  const month = currentDate.getMonth();

  const calendarDays = useMemo(
    () => getCalendarDays(year, month, tasks),
    [year, month, tasks]
  );

  const goToPreviousMonth = useCallback(() => {
    setCurrentDate(new Date(year, month - 1, 1));
  }, [year, month]);

  const goToNextMonth = useCallback(() => {
    setCurrentDate(new Date(year, month + 1, 1));
  }, [year, month]);

  const goToToday = useCallback(() => {
    setCurrentDate(new Date());
  }, []);

  const handleDragStart = useCallback((task: TaskWithAttemptStatus) => {
    setDraggedTask(task);
  }, []);

  const handleDrop = useCallback(
    (date: Date) => {
      if (draggedTask && onTaskDrop) {
        onTaskDrop(draggedTask, date);
      }
      setDraggedTask(null);
    },
    [draggedTask, onTaskDrop]
  );

  // Calculate category distribution for legend
  const categoryStats = useMemo(() => {
    const stats: Record<string, number> = {};
    tasks.forEach((task) => {
      const category = (task.custom_properties as any)?.category || 'other';
      stats[category] = (stats[category] || 0) + 1;
    });
    return stats;
  }, [tasks]);

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Button variant="outline" size="icon" onClick={goToPreviousMonth}>
              <ChevronLeft className="h-4 w-4" />
            </Button>
            <CardTitle className="text-lg font-semibold min-w-[160px] text-center">
              {MONTHS[month]} {year}
            </CardTitle>
            <Button variant="outline" size="icon" onClick={goToNextMonth}>
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={goToToday}>
              Today
            </Button>
          </div>
        </div>

        {/* Category legend */}
        <div className="flex items-center gap-3 mt-3 flex-wrap">
          {Object.entries(categoryStats).map(([category, count]) => (
            <Badge
              key={category}
              variant="outline"
              className="text-xs flex items-center gap-1"
            >
              <div className={cn('h-2 w-2 rounded-full', getCategoryColor(category))} />
              {category}: {count}
            </Badge>
          ))}
        </div>
      </CardHeader>
      <CardContent>
        {/* Day headers */}
        <div className="grid grid-cols-7 mb-2">
          {DAYS_OF_WEEK.map((day) => (
            <div
              key={day}
              className="text-center text-xs font-medium text-muted-foreground py-2"
            >
              {day}
            </div>
          ))}
        </div>

        {/* Calendar grid */}
        <div className="grid grid-cols-7 gap-px bg-border rounded-lg overflow-hidden">
          {calendarDays.map((day, index) => (
            <CalendarCell
              key={index}
              date={day.date}
              isCurrentMonth={day.isCurrentMonth}
              isToday={day.isToday}
              tasks={day.tasks}
              onTaskClick={onTaskClick}
              onDateClick={onDateClick}
              onDragStart={handleDragStart}
              onDrop={handleDrop}
              getCategoryColor={getCategoryColor}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
