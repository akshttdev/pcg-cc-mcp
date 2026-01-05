import { Button } from '@/components/ui/button';
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Calendar, LayoutGrid, List } from 'lucide-react';
import { cn } from '@/lib/utils';

export type BoardViewMode = 'kanban' | 'calendar' | 'list';

interface CalendarViewToggleProps {
  currentView: BoardViewMode;
  onViewChange: (view: BoardViewMode) => void;
  className?: string;
}

export function CalendarViewToggle({
  currentView,
  onViewChange,
  className,
}: CalendarViewToggleProps) {
  return (
    <Tabs value={currentView} onValueChange={(v) => onViewChange(v as BoardViewMode)}>
      <TabsList className={cn('grid grid-cols-3 w-[200px]', className)}>
        <TabsTrigger value="kanban" className="flex items-center gap-1.5">
          <LayoutGrid className="h-3.5 w-3.5" />
          <span className="text-xs">Board</span>
        </TabsTrigger>
        <TabsTrigger value="calendar" className="flex items-center gap-1.5">
          <Calendar className="h-3.5 w-3.5" />
          <span className="text-xs">Calendar</span>
        </TabsTrigger>
        <TabsTrigger value="list" className="flex items-center gap-1.5">
          <List className="h-3.5 w-3.5" />
          <span className="text-xs">List</span>
        </TabsTrigger>
      </TabsList>
    </Tabs>
  );
}

// Alternative button-based toggle
export function ViewModeButtons({
  currentView,
  onViewChange,
  className,
}: CalendarViewToggleProps) {
  return (
    <div className={cn('flex items-center gap-1 p-1 bg-muted rounded-lg', className)}>
      <Button
        variant={currentView === 'kanban' ? 'secondary' : 'ghost'}
        size="sm"
        className="h-7 px-2"
        onClick={() => onViewChange('kanban')}
      >
        <LayoutGrid className="h-3.5 w-3.5 mr-1" />
        Board
      </Button>
      <Button
        variant={currentView === 'calendar' ? 'secondary' : 'ghost'}
        size="sm"
        className="h-7 px-2"
        onClick={() => onViewChange('calendar')}
      >
        <Calendar className="h-3.5 w-3.5 mr-1" />
        Calendar
      </Button>
      <Button
        variant={currentView === 'list' ? 'secondary' : 'ghost'}
        size="sm"
        className="h-7 px-2"
        onClick={() => onViewChange('list')}
      >
        <List className="h-3.5 w-3.5 mr-1" />
        List
      </Button>
    </div>
  );
}
