import { cn } from '@/lib/utils';
import { Card, CardContent } from '@/components/ui/card';

interface MetricCardProps {
  label: string;
  value: string | number;
  icon: React.ElementType;
  accent?: string;
  sub?: React.ReactNode;
  className?: string;
}

export function MetricCard({
  label,
  value,
  icon: Icon,
  accent,
  sub,
  className,
}: MetricCardProps) {
  return (
    <Card className={cn('bg-muted/30 border-border/60', className)}>
      <CardContent className="p-3">
        <div className="flex items-center gap-1.5 mb-1">
          <Icon className="h-3.5 w-3.5" style={accent ? { color: accent } : undefined} />
          <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
            {label}
          </span>
        </div>
        <div className="text-sm font-semibold">{value}</div>
        {sub && <div className="mt-1">{sub}</div>}
      </CardContent>
    </Card>
  );
}
