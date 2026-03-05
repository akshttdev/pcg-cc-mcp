import * as React from 'react';
import { cn } from '@/lib/utils';

export interface InputProps
  extends React.InputHTMLAttributes<HTMLInputElement> {
  onCommandEnter?: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  onCommandShiftEnter?: (e: React.KeyboardEvent<HTMLInputElement>) => void;
}

const Input = React.forwardRef<HTMLInputElement, InputProps>(
  (
    {
      className,
      type,
      onKeyDown,
      onCommandEnter,
      onCommandShiftEnter,
      ...props
    },
    ref
  ) => {
    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Escape') {
        e.currentTarget.blur();
      }
      if (e.key === 'Enter') {
        if (e.metaKey && e.shiftKey) {
          onCommandShiftEnter?.(e);
        } else {
          onCommandEnter?.(e);
        }
      }
      onKeyDown?.(e);
    };

    return (
      <input
        ref={ref}
        type={type}
        onKeyDown={handleKeyDown}
        className={cn(
          'flex h-9 w-full rounded-lg border border-border/60 bg-transparent px-3 py-2 text-sm',
          'placeholder:text-muted-foreground/60',
          'hover:border-border',
          'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/30 focus-visible:border-primary/50',
          'file:border-0 file:bg-transparent file:text-sm file:font-medium',
          'disabled:cursor-not-allowed disabled:opacity-50',
          'transition-colors duration-150',
          className
        )}
        {...props}
      />
    );
  }
);

Input.displayName = 'Input';
export { Input };
