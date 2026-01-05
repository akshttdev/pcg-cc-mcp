import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import {
  Brain,
  Cog,
  CheckCircle2,
  AlertTriangle,
  Pause,
  XCircle,
  Clock,
  User,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AgentFlow } from '@/lib/api';

interface AgentFlowBadgesProps {
  flow: AgentFlow;
  compact?: boolean;
  className?: string;
}

type FlowStatus = 'planning' | 'executing' | 'verifying' | 'awaiting_approval' | 'completed' | 'failed' | 'paused';
type AgentPhase = 'planning' | 'execution' | 'verification';

const statusConfig: Record<FlowStatus, {
  label: string;
  icon: React.ReactNode;
  color: string;
  bgColor: string;
}> = {
  planning: {
    label: 'Planning',
    icon: <Brain className="h-3 w-3" />,
    color: 'text-blue-700',
    bgColor: 'bg-blue-100',
  },
  executing: {
    label: 'Executing',
    icon: <Cog className="h-3 w-3 animate-spin" />,
    color: 'text-yellow-700',
    bgColor: 'bg-yellow-100',
  },
  verifying: {
    label: 'Verifying',
    icon: <CheckCircle2 className="h-3 w-3" />,
    color: 'text-purple-700',
    bgColor: 'bg-purple-100',
  },
  awaiting_approval: {
    label: 'Needs Approval',
    icon: <AlertTriangle className="h-3 w-3" />,
    color: 'text-orange-700',
    bgColor: 'bg-orange-100',
  },
  completed: {
    label: 'Completed',
    icon: <CheckCircle2 className="h-3 w-3" />,
    color: 'text-green-700',
    bgColor: 'bg-green-100',
  },
  failed: {
    label: 'Failed',
    icon: <XCircle className="h-3 w-3" />,
    color: 'text-red-700',
    bgColor: 'bg-red-100',
  },
  paused: {
    label: 'Paused',
    icon: <Pause className="h-3 w-3" />,
    color: 'text-gray-700',
    bgColor: 'bg-gray-100',
  },
};

const phaseIcons: Record<AgentPhase, React.ReactNode> = {
  planning: <Brain className="h-2.5 w-2.5" />,
  execution: <Cog className="h-2.5 w-2.5" />,
  verification: <CheckCircle2 className="h-2.5 w-2.5" />,
};

function getPhaseProgress(phase: string): number {
  switch (phase) {
    case 'planning':
      return 33;
    case 'execution':
      return 66;
    case 'verification':
      return 100;
    default:
      return 0;
  }
}

export function AgentFlowBadges({ flow, compact = false, className }: AgentFlowBadgesProps) {
  const status = (flow.status as FlowStatus) || 'planning';
  const phase = (flow.current_phase as AgentPhase) || 'planning';
  const config = statusConfig[status] || statusConfig.planning;
  const progress = getPhaseProgress(phase);

  if (compact) {
    return (
      <TooltipProvider>
        <div className={cn('flex items-center gap-1', className)}>
          {/* Status Badge - super compact */}
          <Tooltip>
            <TooltipTrigger asChild>
              <div
                className={cn(
                  'flex items-center justify-center h-4 w-4 rounded-full',
                  config.bgColor,
                  config.color
                )}
              >
                {config.icon}
              </div>
            </TooltipTrigger>
            <TooltipContent side="top" className="text-xs">
              <p>{config.label}</p>
              {flow.verification_score !== undefined && flow.verification_score !== null && (
                <p className="text-muted-foreground">
                  Score: {(flow.verification_score * 100).toFixed(0)}%
                </p>
              )}
            </TooltipContent>
          </Tooltip>

          {/* Awaiting Approval indicator */}
          {status === 'awaiting_approval' && (
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center justify-center h-4 w-4 rounded-full bg-orange-100 text-orange-700 animate-pulse">
                  <Clock className="h-2.5 w-2.5" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                Waiting for human approval
              </TooltipContent>
            </Tooltip>
          )}

          {/* Approved indicator */}
          {flow.approved_by && (
            <Tooltip>
              <TooltipTrigger asChild>
                <div className="flex items-center justify-center h-4 w-4 rounded-full bg-green-100 text-green-700">
                  <User className="h-2.5 w-2.5" />
                </div>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                Approved by {flow.approved_by}
              </TooltipContent>
            </Tooltip>
          )}
        </div>
      </TooltipProvider>
    );
  }

  // Full view
  return (
    <div className={cn('space-y-1.5', className)}>
      <div className="flex items-center gap-1.5">
        {/* Status badge */}
        <Badge
          variant="outline"
          className={cn('text-[10px] h-5 px-1.5 gap-1', config.bgColor, config.color, 'border-0')}
        >
          {config.icon}
          {config.label}
        </Badge>

        {/* Verification score */}
        {flow.verification_score !== undefined && flow.verification_score !== null && (
          <Badge variant="outline" className="text-[10px] h-5 px-1.5">
            {(flow.verification_score * 100).toFixed(0)}%
          </Badge>
        )}

        {/* Approved by */}
        {flow.approved_by && (
          <TooltipProvider>
            <Tooltip>
              <TooltipTrigger asChild>
                <Badge variant="outline" className="text-[10px] h-5 px-1.5 gap-1 bg-green-50 text-green-700 border-green-200">
                  <User className="h-2.5 w-2.5" />
                  Approved
                </Badge>
              </TooltipTrigger>
              <TooltipContent side="top" className="text-xs">
                Approved by {flow.approved_by}
              </TooltipContent>
            </Tooltip>
          </TooltipProvider>
        )}
      </div>

      {/* Phase progress */}
      {status !== 'completed' && status !== 'failed' && (
        <div className="flex items-center gap-1">
          {(['planning', 'execution', 'verification'] as AgentPhase[]).map((p, idx) => {
            const isActive = phase === p;
            const isCompleted = progress > idx * 33;

            return (
              <div
                key={p}
                className={cn(
                  'flex items-center justify-center h-4 w-4 rounded-full transition-all',
                  isActive
                    ? 'bg-primary text-primary-foreground'
                    : isCompleted
                      ? 'bg-green-100 text-green-700'
                      : 'bg-muted text-muted-foreground'
                )}
              >
                {phaseIcons[p]}
              </div>
            );
          })}
          <Progress value={progress} className="h-1 flex-1 ml-1" />
        </div>
      )}
    </div>
  );
}
