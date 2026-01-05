import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import {
  Brain,
  Cog,
  CheckCircle2,
  AlertTriangle,
  Pause,
  XCircle,
  ChevronRight,
  User,
  Bot,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AgentFlow, AgentPhase, FlowStatus } from 'shared/types';

interface AgentFlowCardProps {
  flow: AgentFlow;
  onClick?: () => void;
  onApprove?: () => void;
  onReject?: () => void;
  className?: string;
}

const phaseIcons: Record<AgentPhase, React.ReactNode> = {
  planning: <Brain className="h-4 w-4" />,
  execution: <Cog className="h-4 w-4 animate-spin" />,
  verification: <CheckCircle2 className="h-4 w-4" />,
};

const statusColors: Record<FlowStatus, string> = {
  planning: 'bg-blue-100 text-blue-800',
  executing: 'bg-yellow-100 text-yellow-800',
  verifying: 'bg-purple-100 text-purple-800',
  awaiting_approval: 'bg-orange-100 text-orange-800',
  completed: 'bg-green-100 text-green-800',
  failed: 'bg-red-100 text-red-800',
  paused: 'bg-gray-100 text-gray-800',
};

const statusIcons: Record<FlowStatus, React.ReactNode> = {
  planning: <Brain className="h-3 w-3" />,
  executing: <Cog className="h-3 w-3 animate-spin" />,
  verifying: <CheckCircle2 className="h-3 w-3" />,
  awaiting_approval: <AlertTriangle className="h-3 w-3" />,
  completed: <CheckCircle2 className="h-3 w-3" />,
  failed: <XCircle className="h-3 w-3" />,
  paused: <Pause className="h-3 w-3" />,
};

export function AgentFlowCard({
  flow,
  onClick,
  onApprove,
  onReject,
  className,
}: AgentFlowCardProps) {
  const getPhaseProgress = () => {
    switch (flow.current_phase) {
      case 'planning':
        return 33;
      case 'execution':
        return 66;
      case 'verification':
        return 100;
      default:
        return 0;
    }
  };

  return (
    <Card
      className={cn(
        'cursor-pointer transition-all hover:shadow-md',
        flow.status === 'awaiting_approval' && 'ring-2 ring-orange-400',
        className
      )}
      onClick={onClick}
    >
      <CardHeader className="p-3 pb-2">
        <div className="flex items-center justify-between">
          <Badge className={cn('text-xs', statusColors[flow.status])}>
            <span className="mr-1">{statusIcons[flow.status]}</span>
            {flow.status}
          </Badge>
          {flow.verification_score !== null && (
            <Badge variant="outline" className="text-xs">
              Score: {(flow.verification_score * 100).toFixed(0)}%
            </Badge>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-3 pt-0">
        {/* Flow Type */}
        <div className="mb-2">
          <span className="text-sm font-medium text-muted-foreground">
            {flow.flow_type}
          </span>
        </div>

        {/* Phase Progress */}
        <div className="mb-3">
          <div className="flex items-center gap-2 mb-1">
            {(['planning', 'execution', 'verification'] as AgentPhase[]).map(
              (phase, idx) => (
                <div key={phase} className="flex items-center gap-1">
                  <div
                    className={cn(
                      'p-1 rounded-full',
                      flow.current_phase === phase
                        ? 'bg-primary text-primary-foreground'
                        : getPhaseProgress() > idx * 33
                          ? 'bg-green-100 text-green-800'
                          : 'bg-gray-100 text-gray-400'
                    )}
                  >
                    {phaseIcons[phase]}
                  </div>
                  {idx < 2 && (
                    <ChevronRight className="h-3 w-3 text-muted-foreground" />
                  )}
                </div>
              )
            )}
          </div>
          <Progress value={getPhaseProgress()} className="h-1" />
        </div>

        {/* Agent Chain */}
        <div className="flex items-center gap-2 mb-2 text-xs text-muted-foreground">
          <div className="flex items-center gap-1" title="Planner">
            <Brain className="h-3 w-3" />
            {flow.planner_agent_id ? (
              <Bot className="h-3 w-3" />
            ) : (
              <span>-</span>
            )}
          </div>
          <ChevronRight className="h-3 w-3" />
          <div className="flex items-center gap-1" title="Executor">
            <Cog className="h-3 w-3" />
            {flow.executor_agent_id ? (
              <Bot className="h-3 w-3" />
            ) : (
              <span>-</span>
            )}
          </div>
          <ChevronRight className="h-3 w-3" />
          <div className="flex items-center gap-1" title="Verifier">
            <CheckCircle2 className="h-3 w-3" />
            {flow.verifier_agent_id ? (
              <Bot className="h-3 w-3" />
            ) : (
              <span>-</span>
            )}
          </div>
        </div>

        {/* Approval Actions */}
        {flow.status === 'awaiting_approval' && (
          <div className="flex gap-2 mt-3">
            <Button
              size="sm"
              variant="default"
              className="flex-1"
              onClick={(e) => {
                e.stopPropagation();
                onApprove?.();
              }}
            >
              <CheckCircle2 className="h-3 w-3 mr-1" />
              Approve
            </Button>
            <Button
              size="sm"
              variant="destructive"
              className="flex-1"
              onClick={(e) => {
                e.stopPropagation();
                onReject?.();
              }}
            >
              <XCircle className="h-3 w-3 mr-1" />
              Reject
            </Button>
          </div>
        )}

        {/* Approval Info */}
        {flow.approved_by && (
          <div className="flex items-center gap-1 mt-2 text-xs text-muted-foreground">
            <User className="h-3 w-3" />
            Approved by {flow.approved_by}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
