import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ArrowUpDown, Check } from 'lucide-react';
import { useViewStore, type SortField, type SortDirection } from '@/stores/useViewStore';

const sortOptions: { field: SortField; label: string; defaultDir: SortDirection }[] = [
  { field: 'priority', label: 'Priority', defaultDir: 'asc' },
  { field: 'due_date', label: 'Due Date', defaultDir: 'asc' },
  { field: 'updated_at', label: 'Recently Updated', defaultDir: 'desc' },
  { field: 'created_at', label: 'Recently Created', defaultDir: 'desc' },
  { field: 'assignee_id', label: 'Assignee', defaultDir: 'asc' },
  { field: 'title', label: 'Title A-Z', defaultDir: 'asc' },
];

export function SortMenu() {
  const { sortOption, setSortOption } = useViewStore();

  const activeLabel = sortOptions.find((o) => o.field === sortOption.field)?.label || 'Sort';

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="outline" size="sm" className="gap-2">
          <ArrowUpDown className="h-4 w-4" />
          {activeLabel}
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-48">
        {sortOptions.map((opt) => (
          <DropdownMenuItem
            key={opt.field}
            onClick={() => {
              if (sortOption.field === opt.field) {
                // Toggle direction
                setSortOption({
                  field: opt.field,
                  direction: sortOption.direction === 'asc' ? 'desc' : 'asc',
                });
              } else {
                setSortOption({ field: opt.field, direction: opt.defaultDir });
              }
            }}
          >
            <span className="flex-1">{opt.label}</span>
            {sortOption.field === opt.field && (
              <span className="text-xs text-muted-foreground ml-2">
                {sortOption.direction === 'asc' ? '↑' : '↓'}
              </span>
            )}
            {sortOption.field === opt.field && (
              <Check className="h-3.5 w-3.5 ml-1" />
            )}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
