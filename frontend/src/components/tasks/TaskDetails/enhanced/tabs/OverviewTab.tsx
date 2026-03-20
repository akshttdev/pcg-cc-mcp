import type {
  AgentFlowEvent,
  ExecutionArtifact,
  TaskWithAttemptStatus,
} from 'shared/types';

import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';

import { AgentWatcherPanel } from '../../../AgentWatcherPanel';
import { ArtifactGallery } from '../../ArtifactGallery';
import { CollaborationTimeline } from '../../CollaborationTimeline';
import { EnhancedWorkflowView } from '../../EnhancedWorkflowView';

interface OverviewTabProps {
  task: TaskWithAttemptStatus;
  artifacts: ExecutionArtifact[];
  artifactsLoading: boolean;
  artifactsError: string | null;
  workflowEvents: AgentFlowEvent[];
  workflowLoading: boolean;
  workflowError: string | null;
  chatMessages: {
    id: string;
    role: string;
    content: string;
    createdAt: string;
  }[];
  executingAgentId: string | null;
  initialPrompt: string | undefined;
  onArtifactDownload: (artifact: ExecutionArtifact) => void;
  onSendMessage: (message: string, agentName?: string) => Promise<string>;
  onSwitchTab: (
    tab: 'overview' | 'artifacts' | 'workflow' | 'activity' | 'vibe'
  ) => void;
}

export function OverviewTab({
  task,
  artifacts,
  artifactsLoading,
  artifactsError,
  workflowEvents,
  workflowLoading,
  workflowError,
  chatMessages,
  executingAgentId,
  initialPrompt,
  onArtifactDownload,
  onSendMessage,
  onSwitchTab,
}: OverviewTabProps) {
  const artifactCount = artifacts.length;

  return (
    <ScrollArea className="h-full">
      <div className="p-4 space-y-6">
        {/* Screenshot - for bug reports */}
        {task.screenshot && (
          <div>
            <h3 className="text-sm font-medium mb-2">Screenshot</h3>
            <img
              src={task.screenshot}
              alt="Task screenshot"
              className="max-h-64 rounded-md border object-contain w-full bg-muted cursor-pointer hover:opacity-90 transition-opacity"
              onClick={() => window.open(task.screenshot!, '_blank')}
            />
          </div>
        )}

        {/* Description */}
        {task.description && (
          <div>
            <h3 className="text-sm font-medium mb-2">Description</h3>
            <p className="text-sm text-muted-foreground whitespace-pre-wrap">
              {task.description}
            </p>
          </div>
        )}

        {/* Completion Criteria */}
        {task.completion_criteria && (
          <div>
            <h3 className="text-sm font-medium mb-2">Completion Criteria</h3>
            <p className="text-sm text-muted-foreground whitespace-pre-wrap bg-muted/50 rounded-md p-3 border">
              {task.completion_criteria}
            </p>
          </div>
        )}

        {/* Output Format */}
        {task.output_format && (
          <div>
            <h3 className="text-sm font-medium mb-2">Output Format</h3>
            <p className="text-sm text-muted-foreground whitespace-pre-wrap bg-muted/50 rounded-md p-3 border">
              {task.output_format}
            </p>
          </div>
        )}

        {/* Recent Artifacts Preview */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium">Recent Artifacts</h3>
            {artifactCount > 3 && (
              <Button
                variant="link"
                size="sm"
                className="h-auto p-0"
                onClick={() => onSwitchTab('artifacts')}
              >
                View all ({artifactCount})
              </Button>
            )}
          </div>
          {artifactsLoading ? (
            <div className="grid grid-cols-3 gap-3">
              <Skeleton className="h-24" />
              <Skeleton className="h-24" />
              <Skeleton className="h-24" />
            </div>
          ) : artifactsError ? (
            <Alert variant="destructive">
              <AlertDescription>{artifactsError}</AlertDescription>
            </Alert>
          ) : artifacts.length === 0 ? (
            <p className="text-sm text-muted-foreground">No artifacts yet</p>
          ) : (
            <ArtifactGallery
              artifacts={artifacts.slice(0, 6)}
              defaultView="grid"
              showHeader={false}
              onDownload={onArtifactDownload}
              className="border-0 shadow-none"
            />
          )}
        </div>

        {/* Agent Terminal - Always show for messaging */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium">Agent Terminal</h3>
            <Button
              variant="link"
              size="sm"
              className="h-auto p-0"
              onClick={() => onSwitchTab('workflow')}
            >
              Expand
            </Button>
          </div>
          {workflowError ? (
            <Alert variant="destructive">
              <AlertDescription>{workflowError}</AlertDescription>
            </Alert>
          ) : (
            <EnhancedWorkflowView
              events={workflowEvents}
              taskId={task.id}
              taskTitle={task.title}
              onSendMessage={onSendMessage}
              initialPrompt={initialPrompt}
              executingAgentId={executingAgentId || undefined}
              className="h-64 border rounded-lg overflow-hidden"
            />
          )}
        </div>

        {/* Agent Watchers */}
        <div>
          <AgentWatcherPanel taskId={task.id} />
        </div>

        {/* Recent Activity */}
        <div>
          <div className="flex items-center justify-between mb-3">
            <h3 className="text-sm font-medium">Recent Activity</h3>
            <Button
              variant="link"
              size="sm"
              className="h-auto p-0"
              onClick={() => onSwitchTab('activity')}
            >
              View all
            </Button>
          </div>
          {workflowLoading ? (
            <Skeleton className="h-24" />
          ) : workflowEvents.length === 0 && chatMessages.length === 0 ? (
            <p className="text-sm text-muted-foreground">No activity yet</p>
          ) : (
            <CollaborationTimeline
              events={workflowEvents.slice(0, 5)}
              collaborators={task.parsed_collaborators}
              chatMessages={chatMessages.slice(0, 10)}
              className="h-48 border rounded-lg overflow-hidden"
            />
          )}
        </div>
      </div>
    </ScrollArea>
  );
}
