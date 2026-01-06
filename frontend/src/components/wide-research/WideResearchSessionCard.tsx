import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Clock,
  CheckCircle2,
  XCircle,
  PlayCircle,
  Loader2,
  Bot,
  FileText,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { WideResearchSession, ResearchSessionStatus } from 'shared/types';

interface WideResearchSessionCardProps {
  session: WideResearchSession;
  progressPercent: number;
  onClick?: () => void;
  className?: string;
}

const statusColors: Record<ResearchSessionStatus, string> = {
  spawning: 'bg-blue-100 text-blue-800',
  in_progress: 'bg-yellow-100 text-yellow-800',
  aggregating: 'bg-purple-100 text-purple-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
  cancelled: 'bg-gray-100 text-gray-800',
};

const statusIcons: Record<ResearchSessionStatus, React.ReactNode> = {
  spawning: <Loader2 className="h-3 w-3 animate-spin" />,
  in_progress: <PlayCircle className="h-3 w-3" />,
  aggregating: <Loader2 className="h-3 w-3 animate-spin" />,
  completed: <CheckCircle2 className="h-3 w-3" />,
  failed: <XCircle className="h-3 w-3" />,
  cancelled: <Clock className="h-3 w-3" />,
};

export function WideResearchSessionCard({
  session,
  progressPercent,
  onClick,
  className,
}: WideResearchSessionCardProps) {
  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-md',
        className
      )}
      onClick={onClick}
    >
      <CardHeader className="p-3 pb-2">
        <div className="flex items-center justify-between">
          <Badge className={cn('text-xs', statusColors[session.status])}>
            <span className="mr-1">{statusIcons[session.status]}</span>
            {session.status.replace('_', ' ')}
          </Badge>
          <div className="flex items-center gap-1 text-xs text-muted-foreground">
            <Bot className="h-3 w-3" />
            {session.total_subagents} subagents
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-3 pt-0">
        {/* Task Description */}
        <p className="text-sm font-medium line-clamp-2 mb-2">
          {session.task_description}
        </p>

        {/* Progress */}
        <div className="mb-2">
          <div className="flex justify-between text-xs text-muted-foreground mb-1">
            <span>
              {session.completed_subagents} completed / {session.failed_subagents}{' '}
              failed
            </span>
            <span>{progressPercent.toFixed(0)}%</span>
          </div>
          <Progress value={progressPercent} className="h-1.5" />
        </div>

        {/* Mini Grid Preview */}
        <div className="flex gap-0.5 mb-2">
          {Array.from({ length: Math.min(20, session.total_subagents) }).map(
            (_, i) => {
              const isCompleted = i < session.completed_subagents;
              const isFailed =
                i >= session.completed_subagents &&
                i < session.completed_subagents + session.failed_subagents;
              return (
                <div
                  key={i}
                  className={cn(
                    'w-2 h-2 rounded-sm',
                    isCompleted
                      ? 'bg-green-500'
                      : isFailed
                        ? 'bg-red-500'
                        : 'bg-gray-200'
                  )}
                />
              );
            }
          )}
          {session.total_subagents > 20 && (
            <span className="text-xs text-muted-foreground ml-1">
              +{session.total_subagents - 20}
            </span>
          )}
        </div>

        {/* Aggregated Result Badge */}
        {session.aggregated_result_artifact_id && (
          <div className="flex items-center gap-1 text-xs text-green-600">
            <FileText className="h-3 w-3" />
            Result available
          </div>
        )}

        {/* Timestamps */}
        <div className="flex items-center gap-1 mt-2 text-xs text-muted-foreground">
          <Clock className="h-3 w-3" />
          {formatDate(session.created_at)}
        </div>
      </CardContent>
    </Card>
  );
}
