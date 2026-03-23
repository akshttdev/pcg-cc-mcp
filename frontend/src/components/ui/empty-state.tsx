import { LucideIcon } from 'lucide-react';

import { cn } from '@/lib/utils';

import { Button } from './button';

interface EmptyStateProps {
  icon?: LucideIcon;
  title: string;
  description?: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  secondaryAction?: {
    label: string;
    onClick: () => void;
  };
  className?: string;
  /** Visual variant: 'default' uses the standard layout; 'branded' adds a dashed border and tinted background */
  variant?: 'default' | 'branded';
  /** Border color for branded variant, e.g. 'pink-500'. Uses inline styles for dynamic Tailwind support. */
  borderColor?: string;
  /** Background tint for branded variant, e.g. 'pink-500'. Uses inline styles for dynamic Tailwind support. */
  bgTint?: string;
}

/** Map Tailwind color names to CSS color values for inline styles */
function tailwindColorToCss(color: string): string {
  const colorMap: Record<string, string> = {
    'pink-500': '#ec4899',
    'pink-400': '#f472b6',
    'pink-600': '#db2777',
    'purple-500': '#a855f7',
    'purple-400': '#c084fc',
    'purple-600': '#9333ea',
    'indigo-500': '#6366f1',
    'indigo-400': '#818cf8',
    'blue-500': '#3b82f6',
    'blue-400': '#60a5fa',
    'green-500': '#22c55e',
    'green-400': '#4ade80',
    'amber-500': '#f59e0b',
    'amber-400': '#fbbf24',
    'red-500': '#ef4444',
    'red-400': '#f87171',
    'cyan-500': '#06b6d4',
    'teal-500': '#14b8a6',
    'emerald-500': '#10b981',
    'violet-500': '#8b5cf6',
    'rose-500': '#f43f5e',
    'orange-500': '#f97316',
    'slate-500': '#64748b',
  };
  return colorMap[color] ?? color;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  secondaryAction,
  className,
  variant = 'default',
  borderColor,
  bgTint,
}: EmptyStateProps) {
  const isBranded = variant === 'branded';

  const brandedStyle: React.CSSProperties | undefined = isBranded
    ? {
        borderColor: borderColor
          ? `color-mix(in srgb, ${tailwindColorToCss(borderColor)} 30%, transparent)`
          : undefined,
        backgroundColor: bgTint
          ? `color-mix(in srgb, ${tailwindColorToCss(bgTint)} 5%, transparent)`
          : undefined,
      }
    : undefined;

  return (
    <div
      className={cn(
        isBranded
          ? 'rounded-xl border border-dashed p-6 text-center flex flex-col items-center justify-center'
          : 'flex flex-col items-center justify-center py-12 px-4 text-center',
        className
      )}
      style={brandedStyle}
    >
      {Icon && (
        <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-muted">
          <Icon className="h-8 w-8 text-muted-foreground" />
        </div>
      )}

      <h3 className="text-lg font-semibold mb-2">{title}</h3>

      {description && (
        <p className="text-sm text-muted-foreground mb-6 max-w-md">
          {description}
        </p>
      )}

      {(action || secondaryAction) && (
        <div className="flex gap-3">
          {action && (
            <Button onClick={action.onClick}>{action.label}</Button>
          )}
          {secondaryAction && (
            <Button variant="outline" onClick={secondaryAction.onClick}>
              {secondaryAction.label}
            </Button>
          )}
        </div>
      )}
    </div>
  );
}
