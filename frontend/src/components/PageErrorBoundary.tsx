import { AlertTriangle, RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import * as Sentry from '@sentry/react';

interface PageErrorFallbackProps {
  error: Error;
  resetError: () => void;
  label?: string;
}

function PageErrorFallback({ error, resetError, label }: PageErrorFallbackProps) {
  return (
    <div className="flex flex-col items-center justify-center gap-3 p-8 text-center">
      <div className="flex h-12 w-12 items-center justify-center rounded-full bg-destructive/10">
        <AlertTriangle className="h-6 w-6 text-destructive" />
      </div>
      <div className="space-y-1">
        <h3 className="text-sm font-semibold">
          {label ? `${label} failed to load` : 'Something went wrong'}
        </h3>
        <p className="text-xs text-muted-foreground max-w-sm">
          {error.message || 'An unexpected error occurred.'}
        </p>
      </div>
      <Button variant="outline" size="sm" onClick={resetError}>
        <RefreshCw className="h-3.5 w-3.5 mr-1.5" />
        Try Again
      </Button>
    </div>
  );
}

interface PageErrorBoundaryProps {
  children: React.ReactNode;
  label?: string;
}

export function PageErrorBoundary({ children, label }: PageErrorBoundaryProps) {
  return (
    <Sentry.ErrorBoundary
      fallback={({ error, resetError }) => (
        <PageErrorFallback
          error={error as Error}
          resetError={resetError}
          label={label}
        />
      )}
    >
      {children}
    </Sentry.ErrorBoundary>
  );
}
