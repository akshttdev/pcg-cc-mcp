import { useMemo } from 'react';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import {
  Bot,
  User,
  MessageSquare,
  FileText,
  CheckCircle,
  XCircle,
  PlayCircle,
  PauseCircle,
  ArrowRightLeft,
  AlertTriangle,
  Sparkles,
  Clock,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AgentFlowEvent, TaskCollaborator } from 'shared/types';

interface ChatMessage {
  id: string;
  role: string;
  content: string;
  createdAt: string;
  agentName?: string;
}

interface CollaborationTimelineProps {
  events: AgentFlowEvent[];
  collaborators?: TaskCollaborator[] | null;
  chatMessages?: ChatMessage[];
  className?: string;
}

interface TimelineEvent {
  id: string;
  timestamp: Date;
  type: 'workflow' | 'handoff' | 'message' | 'artifact' | 'status';
  actor: {
    type: 'agent' | 'human' | 'system';
    name: string;
  };
  title: string;
  description?: string;
  status?: 'success' | 'error' | 'pending' | 'info';
  metadata?: Record<string, unknown>;
}

// Event type configuration
const eventTypeConfig: Record<string, { icon: React.ReactNode; color: string; label: string }> = {
  phase_started: {
    icon: <PlayCircle className="h-4 w-4" />,
    color: 'text-blue-500 bg-blue-100 dark:bg-blue-900',
    label: 'Phase Started',
  },
  phase_completed: {
    icon: <CheckCircle className="h-4 w-4" />,
    color: 'text-green-500 bg-green-100 dark:bg-green-900',
    label: 'Phase Completed',
  },
  artifact_created: {
    icon: <FileText className="h-4 w-4" />,
    color: 'text-purple-500 bg-purple-100 dark:bg-purple-900',
    label: 'Artifact Created',
  },
  artifact_updated: {
    icon: <FileText className="h-4 w-4" />,
    color: 'text-purple-500 bg-purple-100 dark:bg-purple-900',
    label: 'Artifact Updated',
  },
  agent_handoff: {
    icon: <ArrowRightLeft className="h-4 w-4" />,
    color: 'text-amber-500 bg-amber-100 dark:bg-amber-900',
    label: 'Handoff',
  },
  approval_requested: {
    icon: <AlertTriangle className="h-4 w-4" />,
    color: 'text-orange-500 bg-orange-100 dark:bg-orange-900',
    label: 'Approval Requested',
  },
  approval_decision: {
    icon: <CheckCircle className="h-4 w-4" />,
    color: 'text-green-500 bg-green-100 dark:bg-green-900',
    label: 'Approval Decision',
  },
  flow_paused: {
    icon: <PauseCircle className="h-4 w-4" />,
    color: 'text-yellow-500 bg-yellow-100 dark:bg-yellow-900',
    label: 'Paused',
  },
  flow_resumed: {
    icon: <PlayCircle className="h-4 w-4" />,
    color: 'text-blue-500 bg-blue-100 dark:bg-blue-900',
    label: 'Resumed',
  },
  flow_completed: {
    icon: <CheckCircle className="h-4 w-4" />,
    color: 'text-green-500 bg-green-100 dark:bg-green-900',
    label: 'Completed',
  },
  flow_failed: {
    icon: <XCircle className="h-4 w-4" />,
    color: 'text-red-500 bg-red-100 dark:bg-red-900',
    label: 'Failed',
  },
  subagent_progress: {
    icon: <Sparkles className="h-4 w-4" />,
    color: 'text-cyan-500 bg-cyan-100 dark:bg-cyan-900',
    label: 'Sub-agent Progress',
  },
  wide_research_started: {
    icon: <PlayCircle className="h-4 w-4" />,
    color: 'text-indigo-500 bg-indigo-100 dark:bg-indigo-900',
    label: 'Research Started',
  },
  wide_research_completed: {
    icon: <CheckCircle className="h-4 w-4" />,
    color: 'text-indigo-500 bg-indigo-100 dark:bg-indigo-900',
    label: 'Research Completed',
  },
  chat_user: {
    icon: <MessageSquare className="h-4 w-4" />,
    color: 'text-green-500 bg-green-100 dark:bg-green-900',
    label: 'Message',
  },
  chat_assistant: {
    icon: <MessageSquare className="h-4 w-4" />,
    color: 'text-blue-500 bg-blue-100 dark:bg-blue-900',
    label: 'Response',
  },
};

function parseEventData(eventData: string): Record<string, unknown> {
  try {
    return JSON.parse(eventData || '{}');
  } catch {
    return {};
  }
}

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHour = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHour / 24);

  if (diffSec < 60) return 'just now';
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHour < 24) return `${diffHour}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;

  return date.toLocaleDateString(undefined, {
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function TimelineEventItem({ event }: { event: TimelineEvent }) {
  const config = eventTypeConfig[event.type] || {
    icon: <Clock className="h-4 w-4" />,
    color: 'text-gray-500 bg-gray-100 dark:bg-gray-800',
    label: event.type,
  };

  return (
    <div className="flex gap-3 pb-4 last:pb-0">
      {/* Timeline connector */}
      <div className="flex flex-col items-center">
        <div className={cn('p-1.5 rounded-full', config.color)}>
          {config.icon}
        </div>
        <div className="flex-1 w-px bg-border mt-2" />
      </div>

      {/* Content */}
      <div className="flex-1 pb-4">
        <div className="flex items-start justify-between gap-2">
          <div className="flex items-center gap-2">
            {/* Actor avatar */}
            <Avatar className="h-6 w-6">
              <AvatarFallback
                className={cn(
                  'text-[10px]',
                  event.actor.type === 'agent'
                    ? 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300'
                    : event.actor.type === 'human'
                      ? 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300'
                      : 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300'
                )}
              >
                {event.actor.type === 'agent' ? (
                  <Bot className="h-3 w-3" />
                ) : event.actor.type === 'human' ? (
                  <User className="h-3 w-3" />
                ) : (
                  <Sparkles className="h-3 w-3" />
                )}
              </AvatarFallback>
            </Avatar>

            <div>
              <span className="text-sm font-medium">{event.actor.name}</span>
              <span className="text-xs text-muted-foreground ml-2">{event.title}</span>
            </div>
          </div>

          <span className="text-xs text-muted-foreground shrink-0">
            {formatRelativeTime(event.timestamp)}
          </span>
        </div>

        {event.description && (
          <p className="text-sm text-muted-foreground mt-1 ml-8">
            {event.description}
          </p>
        )}

        {event.status && (
          <Badge
            variant="outline"
            className={cn(
              'ml-8 mt-2 text-xs',
              event.status === 'success' && 'border-green-500 text-green-500',
              event.status === 'error' && 'border-red-500 text-red-500',
              event.status === 'pending' && 'border-yellow-500 text-yellow-500',
              event.status === 'info' && 'border-blue-500 text-blue-500'
            )}
          >
            {event.status}
          </Badge>
        )}
      </div>
    </div>
  );
}

export function CollaborationTimeline({
  events,
  collaborators,
  chatMessages = [],
  className,
}: CollaborationTimelineProps) {
  // Convert AgentFlowEvents and chat messages to TimelineEvents
  const timelineEvents = useMemo(() => {
    const items: TimelineEvent[] = [];

    // Add chat messages
    for (const msg of chatMessages) {
      const isUser = msg.role === 'user';
      const eventType = isUser ? 'chat_user' : 'chat_assistant';

      items.push({
        id: `chat-${msg.id}`,
        timestamp: new Date(msg.createdAt),
        type: 'message',
        actor: {
          type: isUser ? 'human' : 'agent',
          name: isUser ? 'You' : (msg.agentName || 'Agent'),
        },
        title: isUser ? 'Sent message' : 'Responded',
        description: msg.content.length > 100
          ? msg.content.substring(0, 100) + '...'
          : msg.content,
        status: 'info',
        metadata: { eventType },
      });
    }

    // Add workflow events
    for (const event of events) {
      const data = parseEventData(event.event_data);
      const agentName = (data.agent_name as string) || 'Agent';

      let title = eventTypeConfig[event.event_type]?.label || event.event_type;
      let description = data.description as string | undefined;
      let status: TimelineEvent['status'];

      // Customize based on event type
      switch (event.event_type) {
        case 'phase_started':
          title = `Started ${data.phase || 'phase'}`;
          break;
        case 'phase_completed':
          title = `Completed ${data.phase || 'phase'}`;
          status = 'success';
          if (data.duration_ms) {
            description = `Duration: ${((data.duration_ms as number) / 1000).toFixed(1)}s`;
          }
          break;
        case 'artifact_created':
          title = `Created artifact: ${data.title || 'Untitled'}`;
          break;
        case 'agent_handoff':
          title = `Handed off to ${data.to_agent || 'next agent'}`;
          description = data.reason as string;
          break;
        case 'approval_requested':
          title = 'Requested approval';
          status = 'pending';
          break;
        case 'approval_decision':
          title = data.approved ? 'Approved' : 'Rejected';
          status = data.approved ? 'success' : 'error';
          break;
        case 'flow_failed':
          title = 'Workflow failed';
          status = 'error';
          description = (data.error as string) || (data.message as string);
          break;
        case 'flow_completed':
          title = 'Workflow completed';
          status = 'success';
          break;
      }

      items.push({
        id: event.id,
        timestamp: new Date(event.created_at),
        type: event.event_type as TimelineEvent['type'],
        actor: {
          type: 'agent',
          name: agentName,
        },
        title,
        description,
        status,
        metadata: data,
      });
    }

    // Sort by timestamp (newest first for display)
    return items.sort((a, b) => b.timestamp.getTime() - a.timestamp.getTime());
  }, [events]);

  // Get unique collaborators for summary
  const collaboratorSummary = useMemo(() => {
    const agents = new Set<string>();
    const humans = new Set<string>();

    if (collaborators) {
      for (const c of collaborators) {
        if (c.actor_type === 'agent') {
          agents.add(c.actor_id);
        } else {
          humans.add(c.actor_id);
        }
      }
    }

    // Also extract from events
    for (const event of events) {
      const data = parseEventData(event.event_data);
      if (data.agent_name) {
        agents.add(data.agent_name as string);
      }
    }

    return { agents: Array.from(agents), humans: Array.from(humans) };
  }, [events, collaborators]);

  if (timelineEvents.length === 0) {
    return (
      <div className={cn('flex flex-col items-center justify-center py-8 text-muted-foreground', className)}>
        <Clock className="h-12 w-12 mb-2 opacity-50" />
        <p className="text-sm">No activity yet</p>
        <p className="text-xs">Events will appear here as work progresses</p>
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Header with collaborator summary */}
      <div className="flex items-center justify-between gap-3 p-3 border-b">
        <div className="flex items-center gap-2">
          <MessageSquare className="h-5 w-5 text-muted-foreground" />
          <span className="font-medium">Activity</span>
          <Badge variant="secondary">{timelineEvents.length}</Badge>
        </div>

        <div className="flex items-center gap-2">
          {collaboratorSummary.agents.length > 0 && (
            <Badge variant="outline" className="gap-1">
              <Bot className="h-3 w-3 text-blue-500" />
              {collaboratorSummary.agents.length}
            </Badge>
          )}
          {collaboratorSummary.humans.length > 0 && (
            <Badge variant="outline" className="gap-1">
              <User className="h-3 w-3 text-green-500" />
              {collaboratorSummary.humans.length}
            </Badge>
          )}
        </div>
      </div>

      {/* Timeline */}
      <ScrollArea className="flex-1">
        <div className="p-4">
          {timelineEvents.map((event) => (
            <TimelineEventItem key={event.id} event={event} />
          ))}
        </div>
      </ScrollArea>
    </div>
  );
}
