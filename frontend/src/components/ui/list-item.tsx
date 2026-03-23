import * as React from 'react';
import { cn } from '@/lib/utils';

interface ListItemProps {
  icon?: React.ElementType;
  iconClassName?: string;
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  onClick?: () => void;
  className?: string;
  size?: 'sm' | 'md' | 'lg';
}

const sizeStyles = {
  sm: 'py-1.5 px-3 text-xs gap-2',
  md: 'py-2 px-4 text-sm gap-2.5',
  lg: 'py-3 px-4 text-sm gap-3',
} as const;

export function ListItem({
  icon: Icon,
  iconClassName,
  title,
  subtitle,
  actions,
  onClick,
  className,
  size = 'md',
}: ListItemProps) {
  const Comp = onClick ? 'button' : 'div';

  return (
    <Comp
      className={cn(
        'flex items-center justify-between hover:bg-muted/50 rounded-md transition-colors',
        sizeStyles[size],
        onClick && 'cursor-pointer w-full text-left',
        className
      )}
      onClick={onClick}
    >
      <div className="flex items-center gap-2 min-w-0">
        {Icon && (
          <Icon
            className={cn(
              'shrink-0',
              size === 'sm' ? 'h-3.5 w-3.5' : 'h-4 w-4',
              'text-muted-foreground',
              iconClassName
            )}
          />
        )}
        <div className="min-w-0">
          <span className="font-medium truncate block">{title}</span>
          {subtitle && (
            <span className="text-muted-foreground truncate block text-xs">
              {subtitle}
            </span>
          )}
        </div>
      </div>

      {actions && (
        <div className="flex items-center gap-1 shrink-0 ml-2">{actions}</div>
      )}
    </Comp>
  );
}
