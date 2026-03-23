import * as React from 'react';

import { Button, type ButtonProps } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export interface IconButtonProps extends Omit<ButtonProps, 'children'> {
  icon: React.ElementType;
  label: string;
  iconClassName?: string;
}

export const IconButton = React.forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ icon: Icon, label, iconClassName, className, ...props }, ref) => (
    <Button
      ref={ref}
      size="icon"
      title={label}
      aria-label={label}
      className={cn(className)}
      {...props}
    >
      <Icon className={cn('h-4 w-4', iconClassName)} />
    </Button>
  )
);
IconButton.displayName = 'IconButton';
