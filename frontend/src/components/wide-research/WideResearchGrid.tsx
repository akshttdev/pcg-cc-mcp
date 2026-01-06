import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { Clock, CheckCircle2, XCircle, Loader2, PauseCircle, FileText } from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  WideResearchSession,
  WideResearchSubagent,
  SubagentStatus,
} from 'shared/types';

interface WideResearchGridProps {
  session: WideResearchSession;
  subagents: WideResearchSubagent[];
  onSubagentClick?: (subagent: WideResearchSubagent) => void;
  className?: string;
}

const statusColors: Record<SubagentStatus, string> = {
  pending: 'bg-gray-200',
  running: 'bg-blue-400 animate-pulse',
  completed: 'bg-green-500',
  failed: 'bg-red-500',
  cancelled: 'bg-gray-400',
  timeout: 'bg-orange-500',
};

const statusIcons: Record<SubagentStatus, React.ReactNode> = {
  pending: <Clock className="h-3 w-3" />,
  running: <Loader2 className="h-3 w-3 animate-spin" />,
  completed: <CheckCircle2 className="h-3 w-3" />,
  failed: <XCircle className="h-3 w-3" />,
  cancelled: <PauseCircle className="h-3 w-3" />,
  timeout: <Clock className="h-3 w-3" />,
};

export function WideResearchGrid({
  session,
  subagents,
  onSubagentClick,
  className,
}: WideResearchGridProps) {
  // Calculate grid dimensions
  const gridSize = useMemo(() => {
    const total = subagents.length;
    if (total <= 10) return { cols: 5, rows: 2 };
    if (total <= 25) return { cols: 5, rows: 5 };
    if (total <= 50) return { cols: 10, rows: 5 };
    return { cols: 10, rows: Math.ceil(total / 10) };
  }, [subagents.length]);

  // Status counts
  const statusCounts = useMemo(() => {
    const counts: Record<SubagentStatus, number> = {
      pending: 0,
      running: 0,
      completed: 0,
      failed: 0,
      cancelled: 0,
      timeout: 0,
    };

    subagents.forEach((s) => {
      counts[s.status]++;
    });

    return counts;
  }, [subagents]);

  const completedCount = session.completed_subagents ?? 0;
  const failedCount = session.failed_subagents ?? 0;
  const progressPercent = session.total_subagents
    ? ((completedCount + failedCount) / session.total_subagents) * 100
    : 0;

  return (
    <Card className={cn('w-full', className)}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg">Wide Research Session</CardTitle>
          <Badge
            variant={session.status === 'completed' ? 'default' : 'secondary'}
          >
            {session.status.replace('_', ' ')}
          </Badge>
        </div>

        {/* Task Description */}
        <p className="text-sm text-muted-foreground mt-1">
          {session.task_description}
        </p>

        {/* Progress Bar */}
        <div className="mt-3">
          <div className="flex justify-between text-xs text-muted-foreground mb-1">
            <span>
              {completedCount + failedCount} / {session.total_subagents} completed
            </span>
            <span>{progressPercent.toFixed(0)}%</span>
          </div>
          <Progress value={progressPercent} className="h-2" />
        </div>

        {/* Status Legend */}
        <div className="flex flex-wrap gap-3 mt-3">
          {Object.entries(statusCounts)
            .filter(([_, count]) => count > 0)
            .map(([status, count]) => (
              <div
                key={status}
                className="flex items-center gap-1.5 text-xs text-muted-foreground"
              >
                <div
                  className={cn(
                    'w-3 h-3 rounded',
                    statusColors[status as SubagentStatus]
                  )}
                />
                <span>
                  {status}: {count}
                </span>
              </div>
            ))}
        </div>
      </CardHeader>

      <CardContent>
        <TooltipProvider>
          {/* Subagent Grid */}
          <div
            className="grid gap-1"
            style={{
              gridTemplateColumns: `repeat(${gridSize.cols}, minmax(0, 1fr))`,
            }}
          >
            {subagents.map((subagent) => (
              <Tooltip key={subagent.id}>
                <TooltipTrigger asChild>
                  <button
                    className={cn(
                      'aspect-square rounded-sm transition-all hover:scale-110 hover:z-10 flex items-center justify-center',
                      statusColors[subagent.status],
                      subagent.result_artifact_id && 'ring-2 ring-white/50',
                      onSubagentClick && 'cursor-pointer'
                    )}
                    onClick={() => onSubagentClick?.(subagent)}
                  >
                    <span className="text-[8px] text-white font-bold">
                      {subagent.subagent_index + 1}
                    </span>
                  </button>
                </TooltipTrigger>
                <TooltipContent side="top" className="max-w-xs">
                  <div className="space-y-1">
                    <div className="flex items-center gap-2">
                      {statusIcons[subagent.status]}
                      <span className="font-medium">{subagent.status}</span>
                    </div>
                    <p className="text-xs">{subagent.target_item}</p>
                    {subagent.error_message && (
                      <p className="text-xs text-red-400">
                        Error: {subagent.error_message}
                      </p>
                    )}
                    {subagent.result_artifact_id && (
                      <div className="flex items-center gap-1 text-xs text-green-400">
                        <FileText className="h-3 w-3" />
                        Has result artifact
                      </div>
                    )}
                    {subagent.started_at && (
                      <p className="text-xs text-muted-foreground">
                        Started:{' '}
                        {new Date(subagent.started_at).toLocaleTimeString()}
                      </p>
                    )}
                    {subagent.completed_at && (
                      <p className="text-xs text-muted-foreground">
                        Completed:{' '}
                        {new Date(subagent.completed_at).toLocaleTimeString()}
                      </p>
                    )}
                  </div>
                </TooltipContent>
              </Tooltip>
            ))}
          </div>
        </TooltipProvider>

        {/* Session Stats */}
        <div className="mt-4 pt-4 border-t grid grid-cols-3 gap-4 text-center">
          <div>
            <div className="text-2xl font-bold text-green-600">
              {completedCount}
            </div>
            <div className="text-xs text-muted-foreground">Completed</div>
          </div>
            <div>
              <div className="text-2xl font-bold text-red-600">
                {failedCount}
              </div>
              <div className="text-xs text-muted-foreground">Failed</div>
            </div>
            <div>
              <div className="text-2xl font-bold text-blue-600">
                {statusCounts.running}
              </div>
              <div className="text-xs text-muted-foreground">Running</div>
            </div>
        </div>

        {/* Aggregated Result */}
        {session.aggregated_result_artifact_id && (
          <div className="mt-4 p-3 bg-green-50 rounded-lg flex items-center gap-2">
            <CheckCircle2 className="h-5 w-5 text-green-600" />
            <div>
              <div className="font-medium text-sm text-green-800">
                Aggregated Result Available
              </div>
              <div className="text-xs text-green-600">
                Research synthesis complete
              </div>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
