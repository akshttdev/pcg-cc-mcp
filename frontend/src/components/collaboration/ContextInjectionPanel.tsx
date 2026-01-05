import { useState } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  MessageSquare,
  AlertCircle,
  CheckCircle,
  XCircle,
  Compass,
  HelpCircle,
  MessageCircle,
  Check,
  CheckCheck,
  Clock,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import {
  useInjections,
  usePendingInjections,
  useAcknowledgeInjection,
  useAcknowledgeAllInjections,
  type ContextInjection,
  type InjectionType,
} from '@/hooks/useCollaboration';

interface ContextInjectionPanelProps {
  executionId: string;
  showPendingOnly?: boolean;
  className?: string;
}

function getInjectionIcon(type: InjectionType) {
  switch (type) {
    case 'note':
      return <MessageSquare className="h-4 w-4" />;
    case 'correction':
      return <AlertCircle className="h-4 w-4" />;
    case 'approval':
      return <CheckCircle className="h-4 w-4" />;
    case 'rejection':
      return <XCircle className="h-4 w-4" />;
    case 'directive':
      return <Compass className="h-4 w-4" />;
    case 'question':
      return <HelpCircle className="h-4 w-4" />;
    case 'answer':
      return <MessageCircle className="h-4 w-4" />;
    default:
      return <MessageSquare className="h-4 w-4" />;
  }
}

function getInjectionBadge(type: InjectionType) {
  const variants: Record<InjectionType, { variant: 'default' | 'secondary' | 'destructive' | 'outline'; label: string }> = {
    note: { variant: 'secondary', label: 'Note' },
    correction: { variant: 'destructive', label: 'Correction' },
    approval: { variant: 'default', label: 'Approved' },
    rejection: { variant: 'destructive', label: 'Rejected' },
    directive: { variant: 'default', label: 'Directive' },
    question: { variant: 'outline', label: 'Question' },
    answer: { variant: 'secondary', label: 'Answer' },
  };

  const config = variants[type] || { variant: 'secondary' as const, label: type };
  return <Badge variant={config.variant}>{config.label}</Badge>;
}

interface InjectionItemProps {
  injection: ContextInjection;
  onAcknowledge: (id: string) => void;
  isAcknowledging: boolean;
}

function InjectionItem({ injection, onAcknowledge, isAcknowledging }: InjectionItemProps) {
  const isBlocking = injection.injection_type === 'question' || injection.injection_type === 'directive';

  return (
    <div
      className={`p-3 rounded-lg border ${
        !injection.acknowledged
          ? isBlocking
            ? 'border-yellow-500 bg-yellow-500/10'
            : 'border-blue-500 bg-blue-500/10'
          : 'border-border bg-muted/30'
      }`}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2">
          {getInjectionIcon(injection.injection_type)}
          {getInjectionBadge(injection.injection_type)}
          <span className="text-xs text-muted-foreground">
            from {injection.injector_name || injection.injector_id}
          </span>
        </div>
        <div className="flex items-center gap-2">
          {injection.acknowledged ? (
            <Badge variant="outline" className="text-green-600">
              <Check className="h-3 w-3 mr-1" />
              Acknowledged
            </Badge>
          ) : (
            <Button
              size="sm"
              variant="outline"
              onClick={() => onAcknowledge(injection.id)}
              disabled={isAcknowledging}
            >
              <Check className="h-3 w-3 mr-1" />
              Acknowledge
            </Button>
          )}
        </div>
      </div>

      <p className="mt-2 text-sm">{injection.content}</p>

      <div className="mt-2 flex items-center gap-2 text-xs text-muted-foreground">
        <Clock className="h-3 w-3" />
        {formatDistanceToNow(new Date(injection.created_at), { addSuffix: true })}
        {injection.acknowledged_at && (
          <span className="ml-2">
            (acknowledged {formatDistanceToNow(new Date(injection.acknowledged_at), { addSuffix: true })})
          </span>
        )}
      </div>
    </div>
  );
}

export function ContextInjectionPanel({
  executionId,
  showPendingOnly = false,
  className,
}: ContextInjectionPanelProps) {
  const [filter, setFilter] = useState<'all' | 'pending'>(showPendingOnly ? 'pending' : 'all');

  const { data: allInjections = [], isLoading: isLoadingAll } = useInjections(executionId);
  const { data: pendingInjections = [], isLoading: isLoadingPending } = usePendingInjections(executionId);
  const { mutate: acknowledgeInjection, isPending: isAcknowledging } = useAcknowledgeInjection();
  const { mutate: acknowledgeAll, isPending: isAcknowledgingAll } = useAcknowledgeAllInjections(executionId);

  const injections = filter === 'pending' ? pendingInjections : allInjections;
  const isLoading = filter === 'pending' ? isLoadingPending : isLoadingAll;
  const pendingCount = pendingInjections.length;

  const handleAcknowledge = (id: string) => {
    acknowledgeInjection(id);
  };

  const handleAcknowledgeAll = () => {
    acknowledgeAll();
  };

  return (
    <Card className={className}>
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <CardTitle className="text-base flex items-center gap-2">
            <MessageSquare className="h-4 w-4" />
            Context Injections
            {pendingCount > 0 && (
              <Badge variant="destructive" className="ml-1">
                {pendingCount} pending
              </Badge>
            )}
          </CardTitle>
          <div className="flex items-center gap-2">
            <div className="flex rounded-md border">
              <Button
                size="sm"
                variant={filter === 'all' ? 'secondary' : 'ghost'}
                className="rounded-r-none h-7 px-2"
                onClick={() => setFilter('all')}
              >
                All
              </Button>
              <Button
                size="sm"
                variant={filter === 'pending' ? 'secondary' : 'ghost'}
                className="rounded-l-none h-7 px-2"
                onClick={() => setFilter('pending')}
              >
                Pending
              </Button>
            </div>
            {pendingCount > 1 && (
              <Button
                size="sm"
                variant="outline"
                onClick={handleAcknowledgeAll}
                disabled={isAcknowledgingAll}
              >
                <CheckCheck className="h-3 w-3 mr-1" />
                Ack All
              </Button>
            )}
          </div>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="text-center py-4 text-muted-foreground">Loading...</div>
        ) : injections.length === 0 ? (
          <div className="text-center py-4 text-muted-foreground">
            {filter === 'pending' ? 'No pending injections' : 'No context injections yet'}
          </div>
        ) : (
          <ScrollArea className="h-[300px]">
            <div className="space-y-3">
              {injections.map((injection) => (
                <InjectionItem
                  key={injection.id}
                  injection={injection}
                  onAcknowledge={handleAcknowledge}
                  isAcknowledging={isAcknowledging}
                />
              ))}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
