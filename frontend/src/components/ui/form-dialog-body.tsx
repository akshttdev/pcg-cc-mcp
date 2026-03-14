import { cn } from '@/lib/utils';
import { ScrollArea } from '@/components/ui/scroll-area';

interface FormDialogBodyProps {
  children: React.ReactNode;
  footer: React.ReactNode;
  className?: string;
  footerClassName?: string;
}

export function FormDialogBody({ children, footer, className, footerClassName }: FormDialogBodyProps) {
  return (
    <>
      <ScrollArea className={cn('flex-1 -mx-6 px-6', className)}>
        {children}
      </ScrollArea>
      <div className={cn('border-t bg-muted/50 pt-4 -mx-6 px-6 pb-1 flex flex-col-reverse sm:flex-row sm:justify-end sm:space-x-2', footerClassName)}>
        {footer}
      </div>
    </>
  );
}
