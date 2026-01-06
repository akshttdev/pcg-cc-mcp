import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Globe,
  Loader2,
  ExternalLink,
  X,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import type { BrowserSession, BrowserSessionDetails } from '@/hooks/useBowser';
import { useSessionDetails, useCloseSession } from '@/hooks/useBowser';

interface BrowserSessionViewerProps {
  sessionId: string;
  className?: string;
  onClose?: () => void;
}

function getStatusBadge(status: BrowserSession['status']) {
  switch (status) {
    case 'starting':
      return <Badge variant="secondary">Starting</Badge>;
    case 'active':
      return <Badge className="bg-green-500">Active</Badge>;
    case 'idle':
      return <Badge variant="outline">Idle</Badge>;
    case 'closed':
      return <Badge variant="secondary">Closed</Badge>;
    case 'error':
      return <Badge variant="destructive">Error</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

function ActionItem({ action }: { action: BrowserSessionDetails['actions'][0] }) {
  const isSuccess = action.result === 'success';
  const isBlocked = action.result === 'blocked';
  const isFailed = action.result === 'failed' || action.result === 'timeout';

  return (
    <div
      className={cn(
        'flex items-center gap-2 py-1 px-2 rounded text-xs',
        isSuccess && 'bg-green-50 dark:bg-green-900/20',
        isBlocked && 'bg-yellow-50 dark:bg-yellow-900/20',
        isFailed && 'bg-red-50 dark:bg-red-900/20',
        !action.result && 'bg-muted/50'
      )}
    >
      <span className="font-mono text-muted-foreground w-16">
        {action.action_type}
      </span>
      {action.target_selector && (
        <span className="text-muted-foreground truncate max-w-[200px]">
          {action.target_selector}
        </span>
      )}
      {action.duration_ms && (
        <span className="text-muted-foreground ml-auto">
          {action.duration_ms}ms
        </span>
      )}
      {action.result && (
        <Badge
          variant={isSuccess ? 'default' : isBlocked ? 'outline' : 'destructive'}
          className="text-[10px] px-1 py-0"
        >
          {action.result}
        </Badge>
      )}
    </div>
  );
}

export function BrowserSessionViewer({
  sessionId,
  className,
  onClose,
}: BrowserSessionViewerProps) {
  const { data: details, isLoading } = useSessionDetails(sessionId);
  const { mutate: closeSession, isPending: isClosing } = useCloseSession();

  if (isLoading || !details) {
    return (
      <Card className={cn('h-full flex flex-col', className)}>
        <CardContent className="flex-1 flex items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </CardContent>
      </Card>
    );
  }

  const { session, screenshots, actions } = details;
  const successCount = actions.filter((a) => a.result === 'success').length;
  const failedCount = actions.filter(
    (a) => a.result === 'failed' || a.result === 'timeout'
  ).length;
  const blockedCount = actions.filter((a) => a.result === 'blocked').length;

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            <div className="rounded-full bg-muted p-2">
              <Globe className="h-5 w-5" />
            </div>
            <div>
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                Bowser Session
                {getStatusBadge(session.status)}
              </CardTitle>
              <p className="text-xs text-muted-foreground">
                {session.browser_type} &bull; {session.viewport_width}x{session.viewport_height}
                {session.headless && ' (headless)'}
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            {session.status !== 'closed' && session.status !== 'error' && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => closeSession(sessionId)}
                disabled={isClosing}
              >
                {isClosing ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  'Close'
                )}
              </Button>
            )}
            {onClose && (
              <Button variant="ghost" size="sm" onClick={onClose}>
                <X className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden flex flex-col gap-4">
        {/* Current URL */}
        {session.current_url && (
          <div className="flex items-center gap-2 p-2 bg-muted/50 rounded text-sm">
            <Globe className="h-4 w-4 text-muted-foreground" />
            <span className="truncate flex-1">{session.current_url}</span>
            <a
              href={session.current_url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground"
            >
              <ExternalLink className="h-4 w-4" />
            </a>
          </div>
        )}

        {/* Error message */}
        {session.error_message && (
          <div className="p-2 bg-destructive/10 border border-destructive/20 rounded text-sm text-destructive">
            {session.error_message}
          </div>
        )}

        {/* Stats */}
        <div className="grid grid-cols-4 gap-2 text-center text-xs">
          <div className="p-2 bg-muted/50 rounded">
            <div className="font-medium">{actions.length}</div>
            <div className="text-muted-foreground">Actions</div>
          </div>
          <div className="p-2 bg-green-50 dark:bg-green-900/20 rounded">
            <div className="font-medium text-green-600">{successCount}</div>
            <div className="text-muted-foreground">Success</div>
          </div>
          <div className="p-2 bg-red-50 dark:bg-red-900/20 rounded">
            <div className="font-medium text-red-600">{failedCount}</div>
            <div className="text-muted-foreground">Failed</div>
          </div>
          <div className="p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded">
            <div className="font-medium text-yellow-600">{blockedCount}</div>
            <div className="text-muted-foreground">Blocked</div>
          </div>
        </div>

        {/* Screenshots count */}
        <div className="flex items-center justify-between text-sm">
          <span className="text-muted-foreground">Screenshots</span>
          <Badge variant="secondary">{screenshots.length}</Badge>
        </div>

        {/* Recent actions */}
        <div className="flex-1 min-h-0">
          <div className="text-xs font-medium text-muted-foreground mb-2">
            Recent Actions
          </div>
          <ScrollArea className="h-full">
            <div className="space-y-1">
              {actions.slice(-20).reverse().map((action) => (
                <ActionItem key={action.id} action={action} />
              ))}
              {actions.length === 0 && (
                <div className="text-center text-muted-foreground py-4 text-sm">
                  No actions yet
                </div>
              )}
            </div>
          </ScrollArea>
        </div>

        {/* Session time */}
        <div className="text-xs text-muted-foreground text-center">
          Started {formatDistanceToNow(new Date(session.started_at), { addSuffix: true })}
        </div>
      </CardContent>
    </Card>
  );
}
