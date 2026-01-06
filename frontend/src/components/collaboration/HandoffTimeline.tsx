import { cn } from '@/lib/utils';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { ArrowRight, Bot, User, Server, GitBranch } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import type { ExecutionHandoff, ActorType, HandoffType } from '@/hooks/useCollaboration';
import { useHandoffs } from '@/hooks/useCollaboration';

interface HandoffTimelineProps {
  executionId: string;
  className?: string;
}

function getActorIcon(type: ActorType) {
  switch (type) {
    case 'agent':
      return <Bot className="h-4 w-4" />;
    case 'human':
      return <User className="h-4 w-4" />;
    case 'system':
      return <Server className="h-4 w-4" />;
  }
}

function getHandoffTypeBadge(type: HandoffType) {
  switch (type) {
    case 'takeover':
      return <Badge className="bg-blue-500">Takeover</Badge>;
    case 'return':
      return <Badge className="bg-green-500">Return</Badge>;
    case 'escalation':
      return <Badge variant="destructive">Escalation</Badge>;
    case 'delegation':
      return <Badge variant="secondary">Delegation</Badge>;
    case 'assistance':
      return <Badge variant="outline">Assistance</Badge>;
    case 'review_request':
      return <Badge className="bg-yellow-500">Review</Badge>;
    default:
      return <Badge variant="outline">{type}</Badge>;
  }
}

function HandoffItem({ handoff }: { handoff: ExecutionHandoff }) {
  const fromName = handoff.from_actor_name || handoff.from_actor_id;
  const toName = handoff.to_actor_name || handoff.to_actor_id;

  return (
    <div className="flex items-start gap-3 p-3 border rounded-lg">
      <div className="flex items-center gap-2 min-w-0">
        {/* From actor */}
        <div className="flex items-center gap-1 shrink-0">
          <div
            className={cn(
              'rounded-full p-1.5',
              handoff.from_actor_type === 'human'
                ? 'bg-blue-100 text-blue-600'
                : handoff.from_actor_type === 'agent'
                ? 'bg-purple-100 text-purple-600'
                : 'bg-gray-100 text-gray-600'
            )}
          >
            {getActorIcon(handoff.from_actor_type)}
          </div>
          <span className="text-sm font-medium truncate max-w-[80px]" title={fromName}>
            {fromName}
          </span>
        </div>

        {/* Arrow */}
        <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />

        {/* To actor */}
        <div className="flex items-center gap-1 shrink-0">
          <div
            className={cn(
              'rounded-full p-1.5',
              handoff.to_actor_type === 'human'
                ? 'bg-blue-100 text-blue-600'
                : handoff.to_actor_type === 'agent'
                ? 'bg-purple-100 text-purple-600'
                : 'bg-gray-100 text-gray-600'
            )}
          >
            {getActorIcon(handoff.to_actor_type)}
          </div>
          <span className="text-sm font-medium truncate max-w-[80px]" title={toName}>
            {toName}
          </span>
        </div>
      </div>

      <div className="flex-1 min-w-0 space-y-1">
        <div className="flex items-center gap-2">
          {getHandoffTypeBadge(handoff.handoff_type)}
          <span className="text-xs text-muted-foreground">
            {formatDistanceToNow(new Date(handoff.created_at), { addSuffix: true })}
          </span>
        </div>
        {handoff.reason && (
          <p className="text-xs text-muted-foreground truncate" title={handoff.reason}>
            {handoff.reason}
          </p>
        )}
      </div>
    </div>
  );
}

export function HandoffTimeline({ executionId, className }: HandoffTimelineProps) {
  const { data: handoffs = [], isLoading } = useHandoffs(executionId);

  if (isLoading) {
    return (
      <Card className={cn('h-full', className)}>
        <CardContent className="h-full flex items-center justify-center">
          <div className="animate-pulse text-muted-foreground">Loading...</div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={cn('h-full flex flex-col', className)}>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <GitBranch className="h-4 w-4" />
          Handoff History
          <Badge variant="secondary" className="ml-auto">
            {handoffs.length}
          </Badge>
        </CardTitle>
      </CardHeader>
      <CardContent className="flex-1 overflow-hidden p-0">
        {handoffs.length === 0 ? (
          <div className="h-full flex items-center justify-center text-muted-foreground text-sm">
            No handoffs yet
          </div>
        ) : (
          <ScrollArea className="h-full px-4 py-2">
            <div className="space-y-2">
              {handoffs
                .slice()
                .reverse()
                .map((handoff) => (
                  <HandoffItem key={handoff.id} handoff={handoff} />
                ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
