import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { AgentFlowEvent } from 'shared/types';
import { EnhancedWorkflowView } from '../../EnhancedWorkflowView';

interface WorkflowTabProps {
  workflowEvents: AgentFlowEvent[];
  workflowLoading: boolean;
  workflowError: string | null;
  taskId: string;
  taskTitle: string;
  executingAgentId: string | null;
  initialPrompt: string | undefined;
  onSendMessage: (message: string, agentName?: string) => Promise<string>;
}

export function WorkflowTab({
  workflowEvents,
  workflowLoading,
  workflowError,
  taskId,
  taskTitle,
  executingAgentId,
  initialPrompt,
  onSendMessage,
}: WorkflowTabProps) {
  if (workflowLoading) {
    return (
      <div className="p-4 space-y-4">
        <Skeleton className="h-16 w-full" />
        <Skeleton className="h-64 w-full" />
      </div>
    );
  }

  if (workflowError) {
    return (
      <div className="p-4">
        <Alert variant="destructive">
          <AlertDescription>{workflowError}</AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <EnhancedWorkflowView
      events={workflowEvents}
      taskId={taskId}
      taskTitle={taskTitle}
      onSendMessage={onSendMessage}
      initialPrompt={initialPrompt}
      executingAgentId={executingAgentId || undefined}
      className="h-full"
    />
  );
}
