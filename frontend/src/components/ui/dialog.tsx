import * as React from 'react';
import { createPortal } from 'react-dom';
import { X } from 'lucide-react';

import { cn } from '@/lib/utils';
import { useHotkeysContext } from 'react-hotkeys-hook';
import { useKeyExit, useKeySubmit, Scope } from '@/keyboard';

const Dialog = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & {
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
    uncloseable?: boolean;
  }
>(({ className, open, onOpenChange, children, uncloseable, ...props }, ref) => {
  const { enableScope, disableScope } = useHotkeysContext();

  // Manage dialog scope when open/closed
  React.useEffect(() => {
    if (open) {
      enableScope(Scope.DIALOG);
      disableScope(Scope.KANBAN);
      disableScope(Scope.PROJECTS);
    } else {
      disableScope(Scope.DIALOG);
      enableScope(Scope.KANBAN);
      enableScope(Scope.PROJECTS);
    }
  }, [open, enableScope, disableScope]);

  // Dialog keyboard shortcuts using semantic hooks (callbacks memoized to avoid re-registration conflicts)
  const handleEscapeKey = React.useCallback(
    (e: KeyboardEvent | undefined) => {
      if (uncloseable) return;

      // Two-step Esc behavior:
      // 1. If input/textarea is focused, blur it first
      const activeElement = document.activeElement as HTMLElement;
      if (
        activeElement &&
        (activeElement.tagName === 'INPUT' ||
          activeElement.tagName === 'TEXTAREA' ||
          activeElement.isContentEditable)
      ) {
        activeElement.blur();
        e?.preventDefault();
        return;
      }

      // 2. Otherwise close the dialog
      onOpenChange?.(false);
    },
    [uncloseable, onOpenChange]
  );

  const handleSubmitKey = React.useCallback(
    (e: KeyboardEvent | undefined) => {
      // Don't interfere if user is typing in textarea (allow new lines)
      const activeElement = document.activeElement as HTMLElement;
      if (activeElement?.tagName === 'TEXTAREA') {
        return;
      }

      // Look for submit button or primary action button within this dialog
      if (ref && typeof ref === 'object' && ref.current) {
        // First try to find a submit button
        const submitButton = ref.current.querySelector(
          'button[type="submit"]'
        ) as HTMLButtonElement;
        if (submitButton && !submitButton.disabled) {
          e?.preventDefault();
          submitButton.click();
          return;
        }

        // If no submit button, look for primary action button
        const buttons = Array.from(
          ref.current.querySelectorAll('button')
        ) as HTMLButtonElement[];
        const primaryButton = buttons.find(
          (btn) =>
            !btn.disabled &&
            !btn.textContent?.toLowerCase().includes('cancel') &&
            !btn.textContent?.toLowerCase().includes('close') &&
            btn.type !== 'button'
        );

        if (primaryButton) {
          e?.preventDefault();
          primaryButton.click();
        }
      }
    },
    [ref]
  );

  useKeyExit(handleEscapeKey, {
    scope: Scope.DIALOG,
    when: () => !!open,
  });

  useKeySubmit(handleSubmitKey, {
    scope: Scope.DIALOG,
    when: () => !!open,
  });

  if (!open) return null;

  // Provide onOpenChange and uncloseable to DialogContent children via context
  const enrichedChildren = React.Children.map(children, (child) => {
    if (React.isValidElement(child) && (child.type as any)?.displayName === 'DialogContent') {
      return React.cloneElement(child as React.ReactElement<any>, {
        _onClose: uncloseable ? undefined : () => onOpenChange?.(false),
        _uncloseable: uncloseable,
      });
    }
    return child;
  });

  const dialogContent = (
    <div className="fixed inset-0 z-[9999] flex items-center justify-center p-4 overflow-y-auto">
      <div
        className="fixed inset-0 bg-black/50 z-[9998]"
        onClick={() => (uncloseable ? {} : onOpenChange?.(false))}
      />
      <div
        ref={ref}
className={cn('relative z-[10000] w-full flex justify-center', className)}
        {...props}
      >
        {enrichedChildren}
      </div>
    </div>
  );

  return createPortal(dialogContent, document.body);
});
Dialog.displayName = 'Dialog';

const DialogHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      'flex flex-col space-y-1.5 text-center sm:text-left',
      className
    )}
    {...props}
  />
);
DialogHeader.displayName = 'DialogHeader';

const DialogTitle = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className, ...props }, ref) => (
  <h3
    ref={ref}
    className={cn(
      'text-lg font-semibold leading-none tracking-tight',
      className
    )}
    {...props}
  />
));
DialogTitle.displayName = 'DialogTitle';

const DialogDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className, ...props }, ref) => (
  <p
    ref={ref}
    className={cn('text-sm text-muted-foreground', className)}
    {...props}
  />
));
DialogDescription.displayName = 'DialogDescription';

const DialogContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & {
    _onClose?: () => void;
    _uncloseable?: boolean;
  }
>(({ className, children, _onClose, _uncloseable, ...props }, ref) => (
  <div
    ref={ref}
    className={cn(
      'relative w-full max-w-lg bg-background border p-6 shadow-lg duration-200 sm:rounded-lg grid gap-4',
      className
    )}
    {...props}
  >
    {!_uncloseable && _onClose && (
      <button
        className="absolute right-4 top-4 z-10 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
        onClick={_onClose}
      >
        <X className="h-4 w-4" />
        <span className="sr-only">Close</span>
      </button>
    )}
    {children}
  </div>
));
DialogContent.displayName = 'DialogContent';

const DialogFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      'flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2',
      className
    )}
    {...props}
  />
);
DialogFooter.displayName = 'DialogFooter';

const DialogTrigger = React.forwardRef<
  HTMLButtonElement,
  React.ButtonHTMLAttributes<HTMLButtonElement> & { asChild?: boolean }
>(({ asChild, children, ...props }, ref) => {
  if (asChild && React.isValidElement(children)) {
    return React.cloneElement(children as React.ReactElement<React.HTMLAttributes<HTMLElement>>, props as React.HTMLAttributes<HTMLElement>);
  }
  return <button ref={ref} {...props}>{children}</button>;
});
DialogTrigger.displayName = 'DialogTrigger';

export {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
};
