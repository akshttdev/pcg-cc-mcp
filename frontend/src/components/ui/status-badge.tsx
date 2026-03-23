import * as React from 'react';

import { cn } from '@/lib/utils';

export type StatusVariant =
  | 'success'
  | 'warning'
  | 'error'
  | 'info'
  | 'muted'
  | 'pending';

interface StatusBadgeProps {
  status: StatusVariant;
  label: string;
  icon?: React.ElementType;
  pulse?: boolean;
  size?: 'sm' | 'md';
  className?: string;
}

const variantStyles: Record<StatusVariant, string> = {
  success: 'bg-green-500/10 text-green-600 border-green-500/20',
  warning: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
  error: 'bg-red-500/10 text-red-600 border-red-500/20',
  info: 'bg-blue-500/10 text-blue-600 border-blue-500/20',
  muted: 'bg-muted text-muted-foreground border-border',
  pending: 'bg-amber-500/10 text-amber-600 border-amber-500/20',
};

const sizeStyles = {
  sm: 'text-[9px] px-1.5 py-0.5 gap-0.5',
  md: 'text-[10px] px-2 py-0.5 gap-1',
} as const;

export function StatusBadge({
  status,
  label,
  icon: Icon,
  pulse = false,
  size = 'md',
  className,
}: StatusBadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full border font-medium',
        variantStyles[status],
        sizeStyles[size],
        pulse && 'animate-pulse',
        className
      )}
    >
      {Icon && (
        <Icon
          className={cn(
            'shrink-0',
            size === 'sm' ? 'h-2.5 w-2.5' : 'h-3 w-3'
          )}
        />
      )}
      {label}
    </span>
  );
}
