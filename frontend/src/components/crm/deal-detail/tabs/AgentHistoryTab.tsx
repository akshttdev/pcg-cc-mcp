import { useQuery } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
  Bot,
  CheckCircle2,
  Clock,
  AlertTriangle,
  Loader2,
  FileText,
  ChevronDown,
  ChevronRight,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { crmKeys } from '@/lib/query-keys';
import { makeRequest, handleApiResponse } from '@/lib/api/client';
import { cn } from '@/lib/utils';

// ── Types ────────────────────────────────────────────────────────────────────

interface AgentFlowSummary {
  id: string;
  status: string;
  flow_type: string;
  flow_config?: string;
  current_phase: string;
  retry_count: number;
  last_error?: string;
  cancel_deadline?: string;
  execution_started_at?: string;
  execution_completed_at?: string;
  created_at: string;
  updated_at: string;
  events: AgentFlowEventSummary[];
}

interface AgentFlowEventSummary {
  id: string;
  event_type: string;
  event_data: string;
  created_at: string;
}

// ── API ──────────────────────────────────────────────────────────────────────

async function fetchDealAgentFlows(dealId: string): Promise<AgentFlowSummary[]> {
  const response = await makeRequest(`/api/crm/deals/${dealId}/agent-flows`);
  return handleApiResponse<AgentFlowSummary[]>(response);
}

// ── Component ────────────────────────────────────────────────────────────────

interface AgentHistoryTabProps {
  dealId: string;
}

export function AgentHistoryTab({ dealId }: AgentHistoryTabProps) {
  const { data: flows, isLoading } = useQuery({
    queryKey: [...crmKeys.deal(dealId), 'agent-flows'],
    queryFn: () => fetchDealAgentFlows(dealId),
    staleTime: 30 * 1000,
  });

  if (isLoading) {
    return (
      <div className="p-5 flex items-center justify-center text-muted-foreground">
        <Loader2 className="h-4 w-4 animate-spin mr-2" />
        Loading agent history...
      </div>
    );
  }

  if (!flows || flows.length === 0) {
    return (
      <div className="p-5 text-center">
        <Bot className="h-8 w-8 text-muted-foreground/30 mx-auto mb-2" />
        <p className="text-sm text-muted-foreground">No agent activity yet</p>
        <p className="text-xs text-muted-foreground/60 mt-1">
          Agent flows will appear here when triggered by stage transitions
        </p>
      </div>
    );
  }

  return (
    <div className="p-5 space-y-3">
      <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide flex items-center gap-1.5">
        <Bot className="h-3 w-3" /> Agent Execution History
      </h4>
      {flows.map((flow) => (
        <FlowCard key={flow.id} flow={flow} />
      ))}
    </div>
  );
}

// ── FlowCard ─────────────────────────────────────────────────────────────────

function FlowCard({ flow }: { flow: AgentFlowSummary }) {
  const [expanded, setExpanded] = useState(false);

  const config = (() => {
    try {
      return flow.flow_config ? JSON.parse(flow.flow_config) : {};
    } catch {
      return {};
    }
  })();

  const agentName = config.agent_name || flow.flow_type || 'Agent';
  const statusIcon = getStatusIcon(flow.status);
  const statusColor = getStatusColor(flow.status);
  const duration = flow.execution_started_at && flow.execution_completed_at
    ? formatDuration(flow.execution_started_at, flow.execution_completed_at)
    : flow.execution_started_at
      ? 'Running...'
      : null;

  return (
    <Card className="bg-muted/30 border-border/60">
      <CardContent className="p-3">
        <div
          className="flex items-center gap-2 cursor-pointer"
          onClick={() => setExpanded(!expanded)}
        >
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )}
          <Bot className="h-4 w-4 shrink-0" style={{ color: statusColor }} />
          <span className="text-sm font-medium capitalize">{agentName}</span>
          <Badge
            variant="outline"
            className={cn('text-[10px] ml-auto', `text-[${statusColor}] border-[${statusColor}]/30`)}
          >
            {statusIcon}
            <span className="ml-1">{flow.status}</span>
          </Badge>
          {duration && (
            <span className="text-[10px] text-muted-foreground flex items-center gap-0.5">
              <Clock className="h-2.5 w-2.5" />
              {duration}
            </span>
          )}
        </div>

        {flow.retry_count > 0 && (
          <p className="text-[10px] text-amber-500 mt-1 ml-7">
            {flow.retry_count} retry attempt(s)
          </p>
        )}

        {flow.last_error && flow.status === 'failed' && (
          <p className="text-[10px] text-red-500 mt-1 ml-7 line-clamp-2">
            {flow.last_error}
          </p>
        )}

        <p className="text-[10px] text-muted-foreground mt-1 ml-7">
          {formatDistanceToNow(new Date(flow.created_at), { addSuffix: true })}
        </p>

        {expanded && flow.events.length > 0 && (
          <div className="mt-3 ml-7 space-y-1.5 border-l-2 border-border pl-3">
            {flow.events.map((event) => (
              <EventRow key={event.id} event={event} />
            ))}
          </div>
        )}

        {expanded && flow.events.length === 0 && (
          <p className="text-[10px] text-muted-foreground mt-2 ml-7">No events recorded</p>
        )}
      </CardContent>
    </Card>
  );
}

// ── EventRow ─────────────────────────────────────────────────────────────────

function EventRow({ event }: { event: AgentFlowEventSummary }) {
  const icon = getEventIcon(event.event_type);
  const label = event.event_type.replace(/_/g, ' ');

  return (
    <div className="flex items-center gap-2 text-[11px]">
      {icon}
      <span className="capitalize text-muted-foreground">{label}</span>
      <span className="text-muted-foreground/50 ml-auto text-[10px]">
        {formatDistanceToNow(new Date(event.created_at), { addSuffix: true })}
      </span>
    </div>
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function getStatusIcon(status: string) {
  switch (status) {
    case 'completed':
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case 'failed':
      return <AlertTriangle className="h-3 w-3 text-red-500" />;
    case 'executing':
    case 'planning':
      return <Loader2 className="h-3 w-3 text-blue-500 animate-spin" />;
    default:
      return <Clock className="h-3 w-3 text-muted-foreground" />;
  }
}

function getStatusColor(status: string): string {
  switch (status) {
    case 'completed':
      return '#22c55e';
    case 'failed':
      return '#ef4444';
    case 'executing':
    case 'planning':
      return '#3b82f6';
    default:
      return '#6b7280';
  }
}

function getEventIcon(eventType: string) {
  switch (eventType) {
    case 'phase_started':
      return <Loader2 className="h-3 w-3 text-blue-500" />;
    case 'phase_completed':
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case 'artifact_created':
    case 'artifact_updated':
      return <FileText className="h-3 w-3 text-amber-500" />;
    case 'flow_completed':
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case 'flow_failed':
      return <AlertTriangle className="h-3 w-3 text-red-500" />;
    default:
      return <Clock className="h-3 w-3 text-muted-foreground" />;
  }
}

function formatDuration(start: string, end: string): string {
  const ms = new Date(end).getTime() - new Date(start).getTime();
  const secs = Math.floor(ms / 1000);
  if (secs < 60) return `${secs}s`;
  const mins = Math.floor(secs / 60);
  const remSecs = secs % 60;
  return `${mins}m ${remSecs}s`;
}
