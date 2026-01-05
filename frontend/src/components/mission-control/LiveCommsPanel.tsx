import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Radio, Bot, User, CheckCircle, XCircle, MessageSquare, Zap } from 'lucide-react';
import { format } from 'date-fns';

export interface CommEvent {
  id: string;
  timestamp: Date;
  type: 'agent_message' | 'human_action' | 'system_event' | 'plan_update' | 'checkpoint';
  actor: string;
  message: string;
  metadata?: Record<string, unknown>;
}

interface LiveCommsPanelProps {
  events: CommEvent[];
  className?: string;
}

function getEventIcon(type: CommEvent['type']) {
  switch (type) {
    case 'agent_message':
      return <Bot className="h-3 w-3 text-blue-500" />;
    case 'human_action':
      return <User className="h-3 w-3 text-green-500" />;
    case 'system_event':
      return <Zap className="h-3 w-3 text-yellow-500" />;
    case 'plan_update':
      return <CheckCircle className="h-3 w-3 text-purple-500" />;
    case 'checkpoint':
      return <XCircle className="h-3 w-3 text-orange-500" />;
    default:
      return <MessageSquare className="h-3 w-3" />;
  }
}

function CommEventItem({ event }: { event: CommEvent }) {
  return (
    <div className="flex items-start gap-2 py-1.5 px-2 hover:bg-muted/50 rounded transition-colors">
      <div className="shrink-0 mt-0.5">{getEventIcon(event.type)}</div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium">{event.actor}</span>
          <span className="text-[10px] text-muted-foreground">
            {format(event.timestamp, 'HH:mm:ss')}
          </span>
        </div>
        <p className="text-xs text-muted-foreground break-words">{event.message}</p>
      </div>
    </div>
  );
}

export function LiveCommsPanel({ events, className }: LiveCommsPanelProps) {
  // Sort events by timestamp, most recent first
  const sortedEvents = [...events].sort(
    (a, b) => b.timestamp.getTime() - a.timestamp.getTime()
  );

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Radio className="h-4 w-4" />
          Live Coordination
          {events.length > 0 && (
            <Badge variant="outline" className="ml-auto animate-pulse">
              Live
            </Badge>
          )}
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        {events.length === 0 ? (
          <div className="text-center text-muted-foreground py-8 text-sm">
            Waiting for events...
          </div>
        ) : (
          <ScrollArea className="h-full px-2">
            <div className="space-y-0.5 py-2">
              {sortedEvents.map((event) => (
                <CommEventItem key={event.id} event={event} />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}

// Mock function to generate sample events for development
export function generateMockEvents(): CommEvent[] {
  const now = new Date();
  return [
    {
      id: '1',
      timestamp: new Date(now.getTime() - 1000 * 60 * 2),
      type: 'agent_message',
      actor: 'Claude',
      message: 'Starting implementation of authentication module...',
    },
    {
      id: '2',
      timestamp: new Date(now.getTime() - 1000 * 60 * 1.5),
      type: 'plan_update',
      actor: 'System',
      message: 'Plan approved by human, proceeding to execution',
    },
    {
      id: '3',
      timestamp: new Date(now.getTime() - 1000 * 60 * 1),
      type: 'agent_message',
      actor: 'Claude',
      message: 'Created auth service with JWT support',
    },
    {
      id: '4',
      timestamp: new Date(now.getTime() - 1000 * 30),
      type: 'human_action',
      actor: 'Human',
      message: 'Added context: Use bcrypt for password hashing',
    },
    {
      id: '5',
      timestamp: new Date(now.getTime() - 1000 * 10),
      type: 'agent_message',
      actor: 'Claude',
      message: 'Acknowledged. Updating implementation to use bcrypt...',
    },
  ];
}
