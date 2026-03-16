import { ScrollArea } from '@/components/ui/scroll-area';
import type { AgentFlowEvent, TaskCollaborator } from 'shared/types';
import { ActivityTimeline } from '../../../ActivityTimeline';
import { CollaborationTimeline } from '../../CollaborationTimeline';

interface ActivityTabProps {
  taskId: string;
  workflowEvents: AgentFlowEvent[];
  collaborators?: TaskCollaborator[] | null;
  chatMessages: { id: string; role: string; content: string; createdAt: string }[];
}

export function ActivityTab({
  taskId,
  workflowEvents,
  collaborators,
  chatMessages,
}: ActivityTabProps) {
  return (
    <ScrollArea className="h-full">
      <div className="p-4 space-y-6">
        {/* Task Activity Log - shows task changes (status, assignments, etc.) */}
        <ActivityTimeline taskId={taskId} />

        {/* Workflow Collaboration - shows agent interactions and chat */}
        <div className="pt-4 border-t">
          <h3 className="text-lg font-semibold mb-4">
            Workflow & Collaboration
          </h3>
          <CollaborationTimeline
            events={workflowEvents}
            collaborators={collaborators}
            chatMessages={chatMessages}
          />
        </div>
      </div>
    </ScrollArea>
  );
}
