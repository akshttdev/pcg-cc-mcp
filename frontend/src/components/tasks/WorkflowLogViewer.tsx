import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Clock,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Loader2,
  Bot,
  ArrowRight,
  FileText,
  GitBranch,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AgentFlowEvent, FlowEventType } from 'shared/types';

interface WorkflowLogViewerProps {
  events: AgentFlowEvent[];
  className?: string;
}

const eventTypeConfig: Record<FlowEventType, {
  icon: React.ReactNode;
  color: string;
  label: string;
}> = {
  phase_started: {
    icon: <Loader2 className="h-4 w-4" />,
    color: 'bg-blue-100 text-blue-700 border-blue-200',
    label: 'Phase Started',
  },
  phase_completed: {
    icon: <CheckCircle2 className="h-4 w-4" />,
    color: 'bg-green-100 text-green-700 border-green-200',
    label: 'Phase Completed',
  },
  artifact_created: {
    icon: <FileText className="h-4 w-4" />,
    color: 'bg-purple-100 text-purple-700 border-purple-200',
    label: 'Artifact Created',
  },
  artifact_updated: {
    icon: <FileText className="h-4 w-4" />,
    color: 'bg-amber-100 text-amber-700 border-amber-200',
    label: 'Artifact Updated',
  },
  approval_requested: {
    icon: <AlertTriangle className="h-4 w-4" />,
    color: 'bg-orange-100 text-orange-700 border-orange-200',
    label: 'Approval Requested',
  },
  approval_decision: {
    icon: <CheckCircle2 className="h-4 w-4" />,
    color: 'bg-emerald-100 text-emerald-700 border-emerald-200',
    label: 'Approval Decision',
  },
  wide_research_started: {
    icon: <Loader2 className="h-4 w-4" />,
    color: 'bg-indigo-100 text-indigo-700 border-indigo-200',
    label: 'Research Started',
  },
  subagent_progress: {
    icon: <Bot className="h-4 w-4" />,
    color: 'bg-sky-100 text-sky-700 border-sky-200',
    label: 'Subagent Progress',
  },
  wide_research_completed: {
    icon: <CheckCircle2 className="h-4 w-4" />,
    color: 'bg-indigo-100 text-indigo-700 border-indigo-200',
    label: 'Research Completed',
  },
  agent_handoff: {
    icon: <ArrowRight className="h-4 w-4" />,
    color: 'bg-cyan-100 text-cyan-700 border-cyan-200',
    label: 'Agent Handoff',
  },
  flow_paused: {
    icon: <AlertTriangle className="h-4 w-4" />,
    color: 'bg-yellow-100 text-yellow-700 border-yellow-200',
    label: 'Flow Paused',
  },
  flow_resumed: {
    icon: <Loader2 className="h-4 w-4" />,
    color: 'bg-blue-100 text-blue-700 border-blue-200',
    label: 'Flow Resumed',
  },
  flow_failed: {
    icon: <XCircle className="h-4 w-4" />,
    color: 'bg-red-100 text-red-700 border-red-200',
    label: 'Flow Failed',
  },
  flow_completed: {
    icon: <CheckCircle2 className="h-4 w-4" />,
    color: 'bg-green-100 text-green-700 border-green-200',
    label: 'Flow Completed',
  },
};

interface EventData {
  agent_id?: string;
  phase?: string;
  artifact_type?: string;
  artifact_id?: string;
  from_agent?: string;
  to_agent?: string;
  from_phase?: string;
  to_phase?: string;
  instructions?: string;
  reason?: string;
  score?: number;
  message?: string;
}

function WorkflowEvent({ event }: { event: AgentFlowEvent }) {
  const config = eventTypeConfig[event.event_type] || {
    icon: <Clock className="h-4 w-4" />,
    color: 'bg-gray-100 text-gray-700 border-gray-200',
    label: event.event_type,
  };

  const eventData = parseEventData(event);

  const formatTime = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleTimeString(undefined, {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  return (
    <div className="flex gap-3 py-3 px-4 hover:bg-muted/50 transition-colors">
      {/* Timeline indicator */}
      <div className="flex flex-col items-center">
        <div className={cn('p-1.5 rounded-full border', config.color)}>
          {config.icon}
        </div>
        <div className="flex-1 w-px bg-border mt-2" />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          <Badge variant="outline" className={cn('text-xs', config.color)}>
            {config.label}
          </Badge>
          <span className="text-xs text-muted-foreground">
            {formatTime(event.created_at)}
          </span>
        </div>

        {/* Event-specific details */}
        {eventData.agent_id && (
          <div className="flex items-center gap-1 text-sm text-muted-foreground">
            <Bot className="h-3 w-3" />
            <span>{eventData.agent_id}</span>
          </div>
        )}

        {eventData.from_agent && eventData.to_agent && (
          <div className="flex items-center gap-1 text-sm text-muted-foreground">
            <span>{eventData.from_agent}</span>
            <ArrowRight className="h-3 w-3" />
            <span>{eventData.to_agent}</span>
          </div>
        )}

        {eventData.artifact_type && (
          <div className="flex items-center gap-1 text-sm text-muted-foreground">
            <FileText className="h-3 w-3" />
            <span>{eventData.artifact_type}</span>
          </div>
        )}

        {eventData.instructions && (
          <p className="text-sm text-muted-foreground mt-1 line-clamp-2">
            {eventData.instructions}
          </p>
        )}

        {eventData.message && (
          <p className="text-sm text-muted-foreground mt-1">
            {eventData.message}
          </p>
        )}

        {eventData.score !== undefined && (
          <div className="flex items-center gap-1 text-sm mt-1">
            <span className="text-muted-foreground">Score:</span>
            <Badge variant={eventData.score >= 80 ? 'default' : 'secondary'}>
              {eventData.score}%
            </Badge>
          </div>
        )}

        {eventData.reason && (
          <p className="text-sm text-destructive mt-1">
            {eventData.reason}
          </p>
        )}
      </div>
    </div>
  );
}

function parseEventData(event: AgentFlowEvent): EventData {
  try {
    return JSON.parse(event.event_data || '{}');
  } catch {
    return {};
  }
}

export function WorkflowLogViewer({ events, className }: WorkflowLogViewerProps) {
  const sortedEvents = useMemo(
    () => [...events].sort((a, b) =>
      new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
    ),
    [events]
  );

  const phaseStats = useMemo(() => {
    const phases = { planning: 0, execution: 0, verification: 0 };
    events.forEach((e) => {
      const data = parseEventData(e);
      const phase = (data.phase || data.from_phase || '').toLowerCase();
      if (!phase) return;
      if (phase.includes('plan')) phases.planning += 1;
      else if (phase.includes('verify')) phases.verification += 1;
      else if (phase.includes('execute')) phases.execution += 1;
    });
    return phases;
  }, [events]);

  if (events.length === 0) {
    return (
      <Card className={className}>
        <CardContent className="p-6 text-center text-muted-foreground">
          <Clock className="h-12 w-12 mx-auto mb-2 opacity-50" />
          <p>No workflow events yet</p>
          <p className="text-sm">Events will appear here as the workflow progresses</p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-lg flex items-center gap-2">
            <GitBranch className="h-5 w-5 text-muted-foreground" />
            Workflow Log
          </CardTitle>
          <div className="flex items-center gap-2">
            <Badge variant="outline" className="text-xs">
              {events.length} events
            </Badge>
          </div>
        </div>

        {/* Phase progress */}
        <div className="flex items-center gap-3 mt-2">
          <Badge variant="outline" className="bg-purple-50 text-purple-700">
            Planning: {phaseStats.planning}
          </Badge>
          <Badge variant="outline" className="bg-yellow-50 text-yellow-700">
            Execution: {phaseStats.execution}
          </Badge>
          <Badge variant="outline" className="bg-cyan-50 text-cyan-700">
            Verification: {phaseStats.verification}
          </Badge>
        </div>
      </CardHeader>

      <ScrollArea className="h-[400px]">
        <CardContent className="p-0">
          {sortedEvents.map((event) => (
            <WorkflowEvent key={event.id} event={event} />
          ))}
        </CardContent>
      </ScrollArea>
    </Card>
  );
}
