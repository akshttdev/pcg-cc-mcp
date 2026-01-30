/**
 * Ralph Loop Progress Component
 *
 * Displays real-time progress of Ralph Wiggum loop executions.
 * Shows iteration progress, validation results, and status.
 */

import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Separator } from '@/components/ui/separator';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Skeleton } from '@/components/ui/skeleton';
import {
  RefreshCw,
  CheckCircle2,
  XCircle,
  Clock,
  Play,
  Pause,
  AlertTriangle,
  Loader2,
  ChevronRight,
  Terminal,
} from 'lucide-react';
import {
  ralphApi,
  type RalphLoopState,
  type RalphIteration,
  type RalphLoopStatus,
} from '@/lib/api';

interface RalphLoopProgressProps {
  loopId?: string;
  taskAttemptId?: string;
  onCancel?: () => void;
  compact?: boolean;
}

const STATUS_CONFIG: Record<
  RalphLoopStatus,
  { label: string; icon: typeof CheckCircle2; color: string; bgColor: string }
> = {
  initializing: {
    label: 'Initializing',
    icon: Loader2,
    color: 'text-blue-600',
    bgColor: 'bg-blue-100',
  },
  running: {
    label: 'Running',
    icon: Play,
    color: 'text-emerald-600',
    bgColor: 'bg-emerald-100',
  },
  validating: {
    label: 'Validating',
    icon: RefreshCw,
    color: 'text-amber-600',
    bgColor: 'bg-amber-100',
  },
  complete: {
    label: 'Complete',
    icon: CheckCircle2,
    color: 'text-emerald-600',
    bgColor: 'bg-emerald-100',
  },
  maxreached: {
    label: 'Max Iterations',
    icon: AlertTriangle,
    color: 'text-amber-600',
    bgColor: 'bg-amber-100',
  },
  failed: {
    label: 'Failed',
    icon: XCircle,
    color: 'text-red-600',
    bgColor: 'bg-red-100',
  },
  cancelled: {
    label: 'Cancelled',
    icon: Pause,
    color: 'text-gray-600',
    bgColor: 'bg-gray-100',
  },
};

export function RalphLoopProgress({
  loopId,
  taskAttemptId,
  onCancel,
  compact = false,
}: RalphLoopProgressProps) {
  // Fetch loop state
  const {
    data: loopState,
    isLoading: stateLoading,
    error: stateError,
  } = useQuery({
    queryKey: ['ralph-loop', loopId || taskAttemptId],
    queryFn: async () => {
      if (loopId) {
        return ralphApi.getLoopState(loopId);
      } else if (taskAttemptId) {
        const response = await ralphApi.getByAttempt(taskAttemptId);
        if (response.type === 'Found') {
          return response as RalphLoopState;
        }
        return null;
      }
      return null;
    },
    enabled: !!(loopId || taskAttemptId),
    refetchInterval: (query) => {
      // Poll while running
      const data = query.state.data;
      if (data && (data.status === 'running' || data.status === 'validating' || data.status === 'initializing')) {
        return 3000;
      }
      return false;
    },
  });

  // Fetch iterations
  const {
    data: iterations = [],
  } = useQuery({
    queryKey: ['ralph-iterations', loopState?.id],
    queryFn: () => ralphApi.getIterations(loopState!.id),
    enabled: !!loopState?.id,
    refetchInterval: () => {
      if (loopState && (loopState.status === 'running' || loopState.status === 'validating')) {
        return 3000;
      }
      return false;
    },
  });

  // Calculate progress
  const progress = useMemo(() => {
    if (!loopState) return 0;
    return Math.min((loopState.current_iteration / loopState.max_iterations) * 100, 100);
  }, [loopState]);

  // Status config
  const statusConfig = useMemo(() => {
    if (!loopState) return null;
    return STATUS_CONFIG[loopState.status];
  }, [loopState]);

  // Is loop active
  const isActive = useMemo(() => {
    if (!loopState) return false;
    return ['initializing', 'running', 'validating'].includes(loopState.status);
  }, [loopState]);

  // Format duration
  const formatDuration = (ms: number) => {
    const seconds = Math.floor(ms / 1000);
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes > 0) {
      return `${minutes}m ${remainingSeconds}s`;
    }
    return `${seconds}s`;
  };

  // Calculate total duration
  const totalDuration = useMemo(() => {
    if (!loopState) return null;
    const start = new Date(loopState.started_at).getTime();
    const end = loopState.completed_at
      ? new Date(loopState.completed_at).getTime()
      : Date.now();
    return end - start;
  }, [loopState]);

  if (stateLoading) {
    return (
      <Card>
        <CardContent className="py-4">
          <div className="flex items-center gap-3">
            <Skeleton className="h-8 w-8 rounded-full" />
            <div className="flex-1">
              <Skeleton className="h-4 w-32 mb-2" />
              <Skeleton className="h-2 w-full" />
            </div>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (stateError || !loopState) {
    // No Ralph loop for this attempt (normal case for standard execution)
    if (!loopId && taskAttemptId) {
      return null;
    }
    return (
      <Alert variant="destructive">
        <AlertDescription>
          {stateError instanceof Error ? stateError.message : 'Failed to load Ralph loop state'}
        </AlertDescription>
      </Alert>
    );
  }

  if (compact) {
    return (
      <div className="flex items-center gap-3 p-3 rounded-lg border bg-card">
        <div className={`p-2 rounded-full ${statusConfig?.bgColor}`}>
          {statusConfig && (
            <statusConfig.icon
              className={`h-4 w-4 ${statusConfig.color} ${isActive ? 'animate-spin' : ''}`}
            />
          )}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-sm font-medium">Ralph Loop</span>
            <Badge variant="outline" className={`text-xs ${statusConfig?.color}`}>
              {statusConfig?.label}
            </Badge>
          </div>
          <div className="flex items-center gap-2">
            <Progress value={progress} className="h-1.5 flex-1" />
            <span className="text-xs text-muted-foreground whitespace-nowrap">
              {loopState.current_iteration}/{loopState.max_iterations}
            </span>
          </div>
        </div>
        {isActive && onCancel && (
          <Button variant="ghost" size="sm" onClick={onCancel}>
            Cancel
          </Button>
        )}
      </div>
    );
  }

  return (
    <Card>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className={`p-2 rounded-full ${statusConfig?.bgColor}`}>
              {statusConfig && (
                <statusConfig.icon
                  className={`h-5 w-5 ${statusConfig.color} ${
                    isActive && statusConfig.icon === Play ? '' : ''
                  } ${statusConfig.icon === Loader2 || (isActive && statusConfig.icon === RefreshCw) ? 'animate-spin' : ''}`}
                />
              )}
            </div>
            <div>
              <CardTitle className="text-lg">Ralph Loop Execution</CardTitle>
              <CardDescription>
                Iteration {loopState.current_iteration} of {loopState.max_iterations}
              </CardDescription>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className={`${statusConfig?.color} border-current`}
            >
              {statusConfig?.label}
            </Badge>
            {isActive && onCancel && (
              <Button variant="outline" size="sm" onClick={onCancel}>
                Cancel
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Progress Bar */}
        <div className="space-y-2">
          <div className="flex items-center justify-between text-sm">
            <span className="text-muted-foreground">Progress</span>
            <span className="font-medium">{Math.round(progress)}%</span>
          </div>
          <Progress value={progress} className="h-2" />
        </div>

        {/* Stats */}
        <div className="grid grid-cols-3 gap-4">
          <div className="text-center p-3 rounded-lg bg-muted/50">
            <div className="text-2xl font-bold">{loopState.current_iteration}</div>
            <div className="text-xs text-muted-foreground">Iterations</div>
          </div>
          <div className="text-center p-3 rounded-lg bg-muted/50">
            <div className="text-2xl font-bold">
              {totalDuration ? formatDuration(totalDuration) : '-'}
            </div>
            <div className="text-xs text-muted-foreground">Duration</div>
          </div>
          <div className="text-center p-3 rounded-lg bg-muted/50">
            <div className="text-2xl font-bold">
              {loopState.total_tokens_used?.toLocaleString() || '-'}
            </div>
            <div className="text-xs text-muted-foreground">Tokens</div>
          </div>
        </div>

        {/* Error Display */}
        {loopState.last_error && (
          <Alert variant="destructive">
            <AlertTriangle className="h-4 w-4" />
            <AlertTitle>Last Error</AlertTitle>
            <AlertDescription className="font-mono text-xs">
              {loopState.last_error}
            </AlertDescription>
          </Alert>
        )}

        {/* Iterations Timeline */}
        {iterations.length > 0 && (
          <>
            <Separator />
            <div className="space-y-2">
              <h4 className="text-sm font-medium flex items-center gap-2">
                <Terminal className="h-4 w-4" />
                Iteration History
              </h4>
              <div className="space-y-2 max-h-48 overflow-y-auto">
                {iterations.slice().reverse().map((iteration) => (
                  <IterationRow key={iteration.id} iteration={iteration} />
                ))}
              </div>
            </div>
          </>
        )}

        {/* Completion Info */}
        {loopState.status === 'complete' && (
          <Alert className="border-emerald-200 bg-emerald-50">
            <CheckCircle2 className="h-4 w-4 text-emerald-600" />
            <AlertTitle className="text-emerald-800">Task Completed Successfully</AlertTitle>
            <AlertDescription className="text-emerald-700">
              Completed in {loopState.current_iteration} iterations
              {loopState.final_validation_passed && ' with all validations passing'}.
            </AlertDescription>
          </Alert>
        )}

        {/* Max Reached Warning */}
        {loopState.status === 'maxreached' && (
          <Alert className="border-amber-200 bg-amber-50">
            <AlertTriangle className="h-4 w-4 text-amber-600" />
            <AlertTitle className="text-amber-800">Maximum Iterations Reached</AlertTitle>
            <AlertDescription className="text-amber-700">
              The loop reached {loopState.max_iterations} iterations without detecting completion.
              Consider reviewing the task complexity or increasing max iterations.
            </AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
}

// Iteration Row Component
function IterationRow({ iteration }: { iteration: RalphIteration }) {
  const statusIcon = useMemo(() => {
    switch (iteration.status) {
      case 'completed':
        return iteration.all_backpressure_passed ? (
          <CheckCircle2 className="h-4 w-4 text-emerald-500" />
        ) : (
          <AlertTriangle className="h-4 w-4 text-amber-500" />
        );
      case 'failed':
        return <XCircle className="h-4 w-4 text-red-500" />;
      case 'running':
        return <Loader2 className="h-4 w-4 text-blue-500 animate-spin" />;
      case 'timeout':
        return <Clock className="h-4 w-4 text-orange-500" />;
      default:
        return <ChevronRight className="h-4 w-4 text-gray-400" />;
    }
  }, [iteration]);

  return (
    <div className="flex items-center gap-3 p-2 rounded-lg hover:bg-muted/50 transition-colors">
      {statusIcon}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">Iteration {iteration.iteration_number}</span>
          {iteration.completion_signal_found && (
            <Badge variant="outline" className="text-xs text-emerald-600">
              Completion detected
            </Badge>
          )}
        </div>
        {iteration.output_summary && (
          <p className="text-xs text-muted-foreground truncate">{iteration.output_summary}</p>
        )}
      </div>
      <div className="text-right text-xs text-muted-foreground">
        {iteration.duration_ms && (
          <span>{Math.round(iteration.duration_ms / 1000)}s</span>
        )}
      </div>
    </div>
  );
}

export default RalphLoopProgress;
