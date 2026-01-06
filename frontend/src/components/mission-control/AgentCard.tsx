import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { Bot, Clock, FileText, CheckCircle2, XCircle, Loader2, Pause } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import type { ActiveExecutionInfo, AgentTaskPlan } from '@/hooks/useMissionControl';
import { parsePlanSteps } from '@/hooks/useMissionControl';

interface AgentCardProps {
  execution: ActiveExecutionInfo;
  onClick?: () => void;
  isSelected?: boolean;
}

function getStatusIcon(status: string) {
  switch (status) {
    case 'running':
      return <Loader2 className="h-4 w-4 animate-spin text-blue-500" />;
    case 'completed':
      return <CheckCircle2 className="h-4 w-4 text-green-500" />;
    case 'failed':
      return <XCircle className="h-4 w-4 text-red-500" />;
    case 'paused':
      return <Pause className="h-4 w-4 text-yellow-500" />;
    default:
      return <Loader2 className="h-4 w-4 text-muted-foreground" />;
  }
}

function getStatusBadge(status: string) {
  switch (status) {
    case 'running':
      return <Badge variant="default" className="bg-blue-500">Running</Badge>;
    case 'completed':
      return <Badge variant="default" className="bg-green-500">Completed</Badge>;
    case 'failed':
      return <Badge variant="destructive">Failed</Badge>;
    case 'paused':
      return <Badge variant="secondary">Paused</Badge>;
    default:
      return <Badge variant="outline">{status}</Badge>;
  }
}

function PlanProgress({ plan }: { plan: AgentTaskPlan }) {
  const steps = parsePlanSteps(plan.plan_json);
  const currentStep = plan.current_step ?? 0;
  const totalSteps = plan.total_steps ?? steps.length;
  const progress = totalSteps > 0 ? (currentStep / totalSteps) * 100 : 0;

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-xs">
        <span className="text-muted-foreground">
          Step {currentStep} of {totalSteps}
        </span>
        <span className="text-muted-foreground">{Math.round(progress)}%</span>
      </div>
      <Progress value={progress} className="h-1.5" />
      {steps[currentStep] && (
        <p className="text-xs text-muted-foreground truncate">
          {steps[currentStep].title}
        </p>
      )}
    </div>
  );
}

export function AgentCard({ execution, onClick, isSelected }: AgentCardProps) {
  const startedAt = new Date(execution.process.started_at);

  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-md',
        isSelected && 'ring-2 ring-primary'
      )}
      onClick={onClick}
    >
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            <div className="rounded-full bg-muted p-1.5">
              <Bot className="h-4 w-4" />
            </div>
            <div>
              <CardTitle className="text-sm font-medium">
                {execution.executor}
              </CardTitle>
              <p className="text-xs text-muted-foreground">
                {execution.project_name}
              </p>
            </div>
          </div>
          {getStatusBadge(execution.process.status)}
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div>
          <p className="text-sm font-medium truncate" title={execution.task_title}>
            {execution.task_title}
          </p>
          <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
            <Clock className="h-3 w-3" />
            <span>Started {formatDistanceToNow(startedAt, { addSuffix: true })}</span>
          </div>
        </div>

        {execution.plan && (
          <PlanProgress plan={execution.plan} />
        )}

        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            {getStatusIcon(execution.process.status)}
            <span className="text-xs capitalize">
              {execution.process.run_reason.replace('_', ' ')}
            </span>
          </div>
          {execution.artifacts_count > 0 && (
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center gap-1 text-xs text-muted-foreground">
                  <FileText className="h-3 w-3" />
                  <span>{execution.artifacts_count}</span>
                </div>
              </TooltipTrigger>
              <TooltipContent>
                {execution.artifacts_count} artifact{execution.artifacts_count !== 1 ? 's' : ''}
              </TooltipContent>
            </Tooltip>
          )}
        </div>
      </CardContent>
    </Card>
  );
}

export function AgentCardSkeleton() {
  return (
    <Card>
      <CardHeader className="pb-2">
        <div className="flex items-start justify-between">
          <div className="flex items-center gap-2">
            <div className="h-8 w-8 rounded-full bg-muted animate-pulse" />
            <div className="space-y-1">
              <div className="h-4 w-24 bg-muted animate-pulse rounded" />
              <div className="h-3 w-16 bg-muted animate-pulse rounded" />
            </div>
          </div>
          <div className="h-5 w-16 bg-muted animate-pulse rounded-full" />
        </div>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="h-4 w-full bg-muted animate-pulse rounded" />
        <div className="h-3 w-2/3 bg-muted animate-pulse rounded" />
        <div className="h-1.5 w-full bg-muted animate-pulse rounded" />
      </CardContent>
    </Card>
  );
}
