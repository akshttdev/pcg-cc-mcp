import { Eye, X } from 'lucide-react';
import { useViewContext } from '@/contexts/view-context';
import { ROLE_LABELS } from '@/lib/roles';
import { Button } from '@/components/ui/button';

export function ViewAsBanner() {
  const { viewAsRole, isOverridden, clearViewAs } = useViewContext();

  if (!isOverridden || !viewAsRole) return null;

  return (
    <div data-testid="view-as-banner" className="bg-amber-500/10 border-b border-amber-500/20 px-4 py-1.5 flex items-center justify-between">
      <div className="flex items-center gap-2 text-sm text-amber-700 dark:text-amber-400">
        <Eye className="h-3.5 w-3.5" />
        <span>
          Viewing as <strong>{ROLE_LABELS[viewAsRole]}</strong>
        </span>
      </div>
      <Button
        variant="ghost"
        size="sm"
        onClick={clearViewAs}
        className="h-6 px-2 text-xs text-amber-700 dark:text-amber-400 hover:text-amber-900 dark:hover:text-amber-300 hover:bg-amber-500/10"
      >
        <X className="h-3 w-3 mr-1" />
        Reset
      </Button>
    </div>
  );
}
