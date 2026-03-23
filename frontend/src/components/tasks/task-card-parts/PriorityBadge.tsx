import { AlertTriangle, ArrowUp, ArrowDown } from 'lucide-react';

export const PRIORITY_BORDER_COLORS: Record<string, string> = {
  critical: 'border-l-[3px] border-l-red-500',
  high: 'border-l-[3px] border-l-amber-500',
  medium: 'border-l-[3px] border-l-blue-400',
  low: 'border-l-[3px] border-l-slate-300 dark:border-l-slate-600',
};

export function PriorityBadge({ priority }: { priority: string }) {
  switch (priority) {
    case 'critical':
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-xs font-medium bg-red-100 text-red-700 dark:bg-red-950/50 dark:text-red-400 border border-red-200 dark:border-red-800" title="Critical">
          <AlertTriangle className="h-2.5 w-2.5" />
          <span>Critical</span>
        </span>
      );
    case 'high':
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-xs font-medium bg-orange-100 text-orange-700 dark:bg-orange-950/50 dark:text-orange-400 border border-orange-200 dark:border-orange-800" title="High">
          <ArrowUp className="h-2.5 w-2.5" />
          <span>High</span>
        </span>
      );
    case 'low':
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-xs font-medium bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400 border border-slate-200 dark:border-slate-700" title="Low">
          <ArrowDown className="h-2.5 w-2.5" />
          <span>Low</span>
        </span>
      );
    default:
      return (
        <span className="inline-flex items-center gap-0.5 px-1 py-0.5 rounded text-xs font-medium bg-blue-50 text-blue-600 dark:bg-blue-950/40 dark:text-blue-400 border border-blue-200 dark:border-blue-800" title="Medium">
          <span>Medium</span>
        </span>
      );
  }
}
