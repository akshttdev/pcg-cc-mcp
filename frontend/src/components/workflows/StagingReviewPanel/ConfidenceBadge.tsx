import { Badge } from '@/components/ui/badge';
import { cn } from '@/lib/utils';

export const ConfidenceBadge = ({ value }: { value: number | null }) => {
  if (value === null || value === undefined) return null;
  const pct = Math.round(value * 100);
  const color = value >= 0.8 ? 'text-green-600 border-green-200'
    : value >= 0.5 ? 'text-amber-600 border-amber-200'
    : 'text-red-600 border-red-200';
  return (
    <Badge variant="outline" className={cn('text-xs', color)}>
      {pct}%
    </Badge>
  );
};
