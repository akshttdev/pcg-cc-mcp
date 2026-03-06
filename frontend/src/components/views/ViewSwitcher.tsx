import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { LayoutGrid, Table2, Calendar, GalleryVertical, Clock, BarChart3 } from 'lucide-react';
import { useViewStore, type ViewType } from '@/stores/useViewStore';
import { cn } from '@/lib/utils';

const VIEW_OPTIONS: Array<{
  type: ViewType;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  description: string;
}> = [
  {
    type: 'overview',
    label: 'Overview',
    icon: BarChart3,
    description: 'Project summary dashboard',
  },
  {
    type: 'board',
    label: 'Board',
    icon: LayoutGrid,
    description: 'Kanban-style board view',
  },
  {
    type: 'table',
    label: 'Table',
    icon: Table2,
    description: 'Spreadsheet-style list',
  },
  {
    type: 'gallery',
    label: 'Gallery',
    icon: GalleryVertical,
    description: 'Visual card grid',
  },
  {
    type: 'timeline',
    label: 'Timeline',
    icon: Clock,
    description: 'Chronological timeline',
  },
  {
    type: 'calendar',
    label: 'Calendar',
    icon: Calendar,
    description: 'Calendar view',
  },
];

export function ViewSwitcher() {
  const { currentViewType, setViewType } = useViewStore();

  const currentView = VIEW_OPTIONS.find((v) => v.type === currentViewType);
  const CurrentIcon = currentView?.icon || LayoutGrid;

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <CurrentIcon className="h-4 w-4" />
          <span>{currentView?.label || 'Board'}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel>View Type</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {VIEW_OPTIONS.map((view) => {
          const Icon = view.icon;
          const isDisabled = false; // All views are now enabled

          return (
            <DropdownMenuItem
              key={view.type}
              onClick={() => !isDisabled && setViewType(view.type)}
              disabled={isDisabled}
              className={cn(
                'gap-3 cursor-pointer',
                currentViewType === view.type && 'bg-accent',
                isDisabled && 'opacity-50 cursor-not-allowed'
              )}
            >
              <Icon className="h-4 w-4" />
              <div className="flex flex-col">
                <span className="text-sm font-medium">{view.label}</span>
                <span className="text-xs text-muted-foreground">{view.description}</span>
              </div>
            </DropdownMenuItem>
          );
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
