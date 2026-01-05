import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  AlertTriangle,
  Clock,
  Shield,
  RefreshCw,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { usePendingApprovalsSummary } from '@/hooks/useAutonomy';

interface PendingApprovalsDashboardProps {
  className?: string;
  onNavigateToCheckpoints?: () => void;
  onNavigateToGates?: () => void;
}

export function PendingApprovalsDashboard({
  className,
  onNavigateToCheckpoints,
  onNavigateToGates,
}: PendingApprovalsDashboardProps) {
  const { data: summary, isLoading, refetch } = usePendingApprovalsSummary();

  if (isLoading) {
    return (
      <Card className={className}>
        <CardHeader>
          <CardTitle className="text-base">Pending Approvals</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="animate-pulse space-y-3">
            <div className="h-20 bg-muted rounded-lg" />
            <div className="h-20 bg-muted rounded-lg" />
          </div>
        </CardContent>
      </Card>
    );
  }

  const hasPending = summary && summary.total_pending > 0;

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <Shield className="h-4 w-4" />
            Pending Approvals
            {hasPending && (
              <Badge variant="destructive">{summary.total_pending}</Badge>
            )}
          </CardTitle>
          <Button variant="ghost" size="sm" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4" />
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {!hasPending ? (
          <div className="text-center py-8 text-muted-foreground">
            <Shield className="h-8 w-8 mx-auto mb-2 opacity-50" />
            <p className="text-sm">No pending approvals</p>
            <p className="text-xs mt-1">All checkpoints and gates are clear</p>
          </div>
        ) : (
          <div className="space-y-4">
            {/* Checkpoints */}
            {summary.pending_checkpoints > 0 && (
              <div
                className={`p-4 rounded-lg border ${
                  summary.pending_checkpoints > 0
                    ? 'border-yellow-500 bg-yellow-500/10'
                    : 'border-border'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <AlertTriangle className="h-5 w-5 text-yellow-600" />
                    <div>
                      <p className="font-medium">Checkpoints</p>
                      <p className="text-xs text-muted-foreground">
                        Execution checkpoints awaiting review
                      </p>
                    </div>
                  </div>
                  <Badge variant="secondary" className="text-lg">
                    {summary.pending_checkpoints}
                  </Badge>
                </div>
                {onNavigateToCheckpoints && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-3 w-full"
                    onClick={onNavigateToCheckpoints}
                  >
                    Review Checkpoints
                  </Button>
                )}
              </div>
            )}

            {/* Gates */}
            {summary.pending_gates > 0 && (
              <div
                className={`p-4 rounded-lg border ${
                  summary.pending_gates > 0
                    ? 'border-blue-500 bg-blue-500/10'
                    : 'border-border'
                }`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Shield className="h-5 w-5 text-blue-600" />
                    <div>
                      <p className="font-medium">Approval Gates</p>
                      <p className="text-xs text-muted-foreground">
                        Gates requiring approval to proceed
                      </p>
                    </div>
                  </div>
                  <Badge variant="secondary" className="text-lg">
                    {summary.pending_gates}
                  </Badge>
                </div>
                {onNavigateToGates && (
                  <Button
                    variant="outline"
                    size="sm"
                    className="mt-3 w-full"
                    onClick={onNavigateToGates}
                  >
                    Review Gates
                  </Button>
                )}
              </div>
            )}

            {/* Oldest pending */}
            {summary.oldest_pending_at && (
              <div className="flex items-center gap-2 text-xs text-muted-foreground pt-2 border-t">
                <Clock className="h-3 w-3" />
                <span>
                  Oldest pending:{' '}
                  {formatDistanceToNow(new Date(summary.oldest_pending_at), {
                    addSuffix: true,
                  })}
                </span>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

// Compact version for sidebars or headers
export function PendingApprovalsIndicator({ className }: { className?: string }) {
  const { data: summary } = usePendingApprovalsSummary();

  if (!summary || summary.total_pending === 0) {
    return null;
  }

  return (
    <div className={`flex items-center gap-2 ${className}`}>
      <Badge variant="destructive" className="animate-pulse">
        <AlertTriangle className="h-3 w-3 mr-1" />
        {summary.total_pending} pending
      </Badge>
    </div>
  );
}
