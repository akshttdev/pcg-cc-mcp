import { Box, Square } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export interface ViewToggleProps {
  mode: '2d' | '3d';
  onChange: (mode: '2d' | '3d') => void;
}

export function ViewToggle({ mode, onChange }: ViewToggleProps) {
  return (
    <div className="inline-flex items-center rounded-md border border-slate-700 bg-slate-900 p-0.5">
      <Button
        variant="ghost"
        size="sm"
        className={cn(
          'h-7 gap-1 rounded-sm px-2 text-xs',
          mode === '2d' && 'bg-slate-700 text-slate-50'
        )}
        onClick={() => onChange('2d')}
        data-testid="topology-view-toggle-2d"
      >
        <Square className="h-3 w-3" />
        2D
      </Button>
      <Button
        variant="ghost"
        size="sm"
        className={cn(
          'h-7 gap-1 rounded-sm px-2 text-xs',
          mode === '3d' && 'bg-slate-700 text-slate-50'
        )}
        onClick={() => onChange('3d')}
        data-testid="topology-view-toggle-3d"
      >
        <Box className="h-3 w-3" />
        3D
      </Button>
    </div>
  );
}
