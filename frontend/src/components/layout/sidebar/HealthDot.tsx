import { cn } from '@/lib/utils';

export function HealthDot({ status }: { status?: string }) {
  if (!status) return null;
  const color =
    status === 'critical'
      ? 'bg-destructive'
      : status === 'warning'
        ? 'bg-[hsl(var(--warning))]'
        : 'bg-[hsl(var(--success))]';
  return (
    <span
      className={cn('inline-block w-1.5 h-1.5 rounded-full shrink-0', color)}
      title={`Health: ${status}`}
    />
  );
}
