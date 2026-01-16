import { useEffect, useState, useMemo, useCallback } from 'react';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  FileText,
  Clock,
  Zap,
  LayoutGrid,
  RefreshCw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  TaskWithAttemptStatus,
  ExecutionArtifact,
  AgentFlowEvent,
  ArtifactType,
  FlowEventType,
} from 'shared/types';
import type { TaskCardMode } from '../EnhancedTaskCard';
import { EnhancedTaskHeader } from './EnhancedTaskHeader';
import { ArtifactGallery } from './ArtifactGallery';
import { CollaborationTimeline } from './CollaborationTimeline';
import { EnhancedWorkflowView } from './EnhancedWorkflowView';
import { ActivityTimeline } from '../ActivityTimeline';
import { agentFlowsApi, taskArtifactsApi, agentsApi } from '@/lib/api';
import type { ExecutionArtifact as ApiExecutionArtifact } from '@/lib/api';
import type { AgentChatRequest } from 'shared/types';

interface EnhancedTaskDetailsPanelProps {
  task: TaskWithAttemptStatus;
  projectId: string;
  onClose: () => void;
  onEdit?: () => void;
  onDelete?: () => void;
  onDuplicate?: () => void;
  onToggleFullscreen?: () => void;
  isFullscreen?: boolean;
  hideClose?: boolean;
  className?: string;
}

// Valid flow event types for normalization
const FLOW_EVENT_TYPES: FlowEventType[] = [
  'phase_started',
  'phase_completed',
  'artifact_created',
  'artifact_updated',
  'approval_requested',
  'approval_decision',
  'wide_research_started',
  'subagent_progress',
  'wide_research_completed',
  'agent_handoff',
  'flow_paused',
  'flow_resumed',
  'flow_failed',
  'flow_completed',
];

function normalizeFlowEventType(value: string): FlowEventType {
  return FLOW_EVENT_TYPES.includes(value as FlowEventType)
    ? (value as FlowEventType)
    : 'flow_completed';
}

// Detect card mode from artifacts/tags
function detectCardMode(task: TaskWithAttemptStatus, artifacts: ExecutionArtifact[]): TaskCardMode {
  const tags = task.tags?.toLowerCase() || '';

  // Check tags first
  if (tags.includes('visual') || tags.includes('design') || tags.includes('image')) {
    return 'visual';
  }
  if (tags.includes('document') || tags.includes('writing') || tags.includes('content')) {
    return 'document';
  }
  if (tags.includes('media') || tags.includes('video') || tags.includes('recording')) {
    return 'media';
  }
  if (tags.includes('code') || tags.includes('development') || tags.includes('programming')) {
    return 'terminal';
  }

  // Check artifacts - use actual ArtifactType values
  const artifactTypes = artifacts.map((a) => a.artifact_type);
  if (artifactTypes.some((t) => t === 'visual_brief' || t === 'screenshot')) {
    return 'visual';
  }
  if (
    artifactTypes.some(
      (t) =>
        t === 'content_draft' ||
        t === 'research_report' ||
        t === 'strategy_document' ||
        t === 'content_calendar' ||
        t === 'competitor_analysis'
    )
  ) {
    return 'document';
  }
  if (artifactTypes.some((t) => t === 'browser_recording' || t === 'walkthrough')) {
    return 'media';
  }
  if (artifactTypes.some((t) => t === 'diff_summary' || t === 'test_result' || t === 'checkpoint')) {
    return 'terminal';
  }

  // Default based on assigned agent
  const agentName = task.assigned_agent?.toLowerCase() || '';
  if (agentName.includes('design') || agentName.includes('visual')) return 'visual';
  if (agentName.includes('writer') || agentName.includes('content')) return 'document';

  return 'terminal';
}

type DetailTab = 'overview' | 'artifacts' | 'workflow' | 'activity';

export function EnhancedTaskDetailsPanel({
  task,
  projectId,
  onClose,
  onEdit,
  onDelete,
  onDuplicate,
  onToggleFullscreen,
  isFullscreen,
  hideClose,
  className,
}: EnhancedTaskDetailsPanelProps) {
  const [activeTab, setActiveTab] = useState<DetailTab>('overview');
  const [artifacts, setArtifacts] = useState<ExecutionArtifact[]>([]);
  const [artifactsLoading, setArtifactsLoading] = useState(false);
  const [artifactsError, setArtifactsError] = useState<string | null>(null);
  const [workflowEvents, setWorkflowEvents] = useState<AgentFlowEvent[]>([]);
  const [workflowLoading, setWorkflowLoading] = useState(false);
  const [workflowError, setWorkflowError] = useState<string | null>(null);
  const [executingAgentId, setExecutingAgentId] = useState<string | null>(null);
  const [chatMessages, setChatMessages] = useState<{ id: string; role: string; content: string; createdAt: string }[]>([]);

  // Detect card mode
  const mode = useMemo(() => detectCardMode(task, artifacts), [task, artifacts]);

  // Look up agent ID for message persistence
  useEffect(() => {
    const agentName = task.assigned_agent;
    if (!agentName || executingAgentId) return;

    agentsApi.getByName(agentName)
      .then((agent) => {
        console.log('[TaskPanel] Resolved agent ID for persistence:', agent.id, agent.short_name);
        setExecutingAgentId(agent.id);
      })
      .catch((err) => {
        console.log('[TaskPanel] Could not resolve agent ID:', err.message);
      });
  }, [task.assigned_agent, executingAgentId]);

  // Fetch chat messages for activity log
  useEffect(() => {
    if (!executingAgentId) return;

    const sessionId = `task-${task.id}`;
    agentsApi.getConversationBySession(executingAgentId, sessionId)
      .then((result) => {
        if (result && result.messages) {
          console.log('[TaskPanel] Loaded', result.messages.length, 'chat messages for activity');
          setChatMessages(result.messages);
        }
      })
      .catch((err) => {
        console.log('[TaskPanel] Could not load chat messages:', err.message);
      });
  }, [executingAgentId, task.id]);

  // Fetch artifacts
  useEffect(() => {
    let cancelled = false;
    setArtifactsLoading(true);
    setArtifactsError(null);

    taskArtifactsApi
      .list(task.id)
      .then((data) => {
        if (cancelled) return;
        const executionArtifacts = data
          .map((item) => item.artifact)
          .filter((artifact): artifact is ApiExecutionArtifact => Boolean(artifact));
        const normalized: ExecutionArtifact[] = executionArtifacts.map((artifact) => ({
          id: artifact.id,
          execution_process_id: artifact.execution_process_id ?? '',
          artifact_type: artifact.artifact_type as ArtifactType,
          title: artifact.title ?? 'Untitled',
          content: artifact.content ?? null,
          file_path: artifact.file_path ?? null,
          metadata: artifact.metadata ?? null,
          created_at: artifact.created_at,
        }));
        setArtifacts(normalized);
      })
      .catch((error) => {
        if (cancelled) return;
        setArtifacts([]);
        setArtifactsError(error instanceof Error ? error.message : 'Failed to load artifacts.');
      })
      .finally(() => {
        if (!cancelled) {
          setArtifactsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [task.id]);

  // Fetch workflow events
  useEffect(() => {
    let cancelled = false;
    setWorkflowLoading(true);
    setWorkflowError(null);

    agentFlowsApi
      .list({ task_id: task.id })
      .then(async (flows) => {
        if (cancelled) return [] as AgentFlowEvent[];
        if (!flows.length) {
          setWorkflowEvents([]);
          return [] as AgentFlowEvent[];
        }
        return agentFlowsApi.getEvents(flows[0].id);
      })
      .then((events) => {
        if (!events || cancelled) return;
        const normalized: AgentFlowEvent[] = events.map((event) => ({
          ...event,
          event_type: normalizeFlowEventType(event.event_type),
          event_data: event.event_data ?? '',
        }));
        setWorkflowEvents(normalized);
      })
      .catch((error) => {
        if (cancelled) return;
        setWorkflowEvents([]);
        setWorkflowError(error instanceof Error ? error.message : 'Failed to load workflow.');
      })
      .finally(() => {
        if (!cancelled) {
          setWorkflowLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [task.id]);

  // Extract agent name from workflow events
  const executingAgentName = useMemo(() => {
    for (const event of workflowEvents) {
      try {
        const data = JSON.parse(event.event_data || '{}');
        if (data.agent_name) {
          return data.agent_name as string;
        }
      } catch {
        // Skip invalid JSON
      }
    }
    return task.assigned_agent || null;
  }, [workflowEvents, task.assigned_agent]);

  // Handler to send messages to agent
  const handleSendMessage = useCallback(
    async (message: string, agentName?: string): Promise<string> => {
      const targetAgentName = executingAgentName || agentName || task.assigned_agent;

      console.log('[TaskPanel] handleSendMessage called:', {
        messagePreview: message.substring(0, 50) + '...',
        targetAgentName,
        executingAgentName,
        assignedAgent: task.assigned_agent,
        taskId: task.id,
      });

      // Helper to wrap fetch with timeout
      const fetchWithTimeout = async (url: string, options: RequestInit, timeoutMs: number): Promise<Response> => {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
        try {
          const response = await fetch(url, { ...options, signal: controller.signal });
          clearTimeout(timeoutId);
          return response;
        } catch (error) {
          clearTimeout(timeoutId);
          if (error instanceof Error && error.name === 'AbortError') {
            throw new Error(`Request timed out after ${timeoutMs / 1000} seconds`);
          }
          throw error;
        }
      };

      // Helper to extract user-friendly error message from API response
      const extractErrorMessage = (status: number, responseText: string): string => {
        try {
          const data = JSON.parse(responseText);
          // Check for nested error messages (OpenAI style)
          if (data.error?.message) {
            const msg = data.error.message;
            if (msg.includes('quota') || msg.includes('exceeded')) {
              return 'API quota exceeded. Please check your OpenAI billing or try again later.';
            }
            return msg;
          }
          if (data.message) {
            if (data.message.includes('quota') || data.message.includes('exceeded')) {
              return 'API quota exceeded. Please check your OpenAI billing or try again later.';
            }
            return data.message;
          }
          if (data.error) return data.error;
        } catch {
          // Not JSON, use raw text
        }

        if (status === 402) {
          return 'Insufficient VIBE balance. Please add more VIBE to continue.';
        }
        if (status === 404) {
          return 'Agent not found.';
        }
        if (status === 429) {
          return 'API rate limit exceeded. Please try again in a moment.';
        }
        if (status === 500 || status === 502 || status === 503) {
          return 'Server error. The AI service may be temporarily unavailable.';
        }
        return responseText || `Request failed with status ${status}`;
      };

      // Try agent-specific chat if we have an agent name
      if (targetAgentName) {
        try {
          console.log('[TaskPanel] Looking up agent:', targetAgentName);
          const agent = await agentsApi.getByName(targetAgentName);
          console.log('[TaskPanel] Found agent:', agent.id, agent.short_name);

          // Save agent ID for persistence
          if (!executingAgentId) {
            setExecutingAgentId(agent.id);
          }

          const request: AgentChatRequest = {
            message,
            sessionId: `task-${task.id}`,
            projectId: projectId || null,
            context: {
              taskId: task.id,
              taskTitle: task.title,
              isWorkflowFollowUp: true,
            },
            stream: false,
          };

          console.log('[TaskPanel] Calling agent chat API...');

          // Use fetchWithTimeout for agent chat (30 second timeout)
          const response = await fetchWithTimeout(
            `/api/agents/${agent.id}/chat`,
            {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify(request),
              credentials: 'include',
            },
            30000
          );

          if (!response.ok) {
            const errorText = await response.text();
            const userMessage = extractErrorMessage(response.status, errorText);

            // For payment errors, don't fall back - throw immediately with clear message
            if (response.status === 402) {
              throw new Error(userMessage);
            }

            throw new Error(`Agent chat failed: ${userMessage}`);
          }

          const data = await response.json();
          console.log('[TaskPanel] Got agent response:', data.content?.substring(0, 100) + '...');
          return data.content;
        } catch (error) {
          console.warn(`[TaskPanel] Agent chat failed for ${targetAgentName}:`, error);

          // If it's a payment error, don't fall back to Nora - just throw
          if (error instanceof Error && error.message.includes('VIBE')) {
            throw error;
          }

          console.log('[TaskPanel] Falling back to Nora...');
        }
      }

      // Fallback: route through Nora with timeout
      console.log('[TaskPanel] Using Nora fallback');
      const agentContext = targetAgentName ? `[To ${targetAgentName}] ` : '';
      const contextualMessage = `${agentContext}Regarding task "${task.title}": ${message}`;

      try {
        const response = await fetchWithTimeout(
          '/api/nora/chat',
          {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            credentials: 'include',
            body: JSON.stringify({
              message: contextualMessage,
              sessionId: `task-${task.id}`,
              requestType: 'textInteraction',
              voiceEnabled: false,
              priority: 'normal',
              context: {
                taskId: task.id,
                taskTitle: task.title,
                projectId,
                executingAgent: targetAgentName,
                isWorkflowFollowUp: true,
              },
            }),
          },
          60000 // 60 second timeout for Nora
        );

        if (!response.ok) {
          const errorText = await response.text();
          console.error('[TaskPanel] Nora chat failed:', response.status, errorText);
          const userMessage = extractErrorMessage(response.status, errorText);
          throw new Error(userMessage);
        }

        const data = await response.json();
        console.log('[TaskPanel] Nora response:', data);
        return data.content || data.message || data.response || 'Response received';
      } catch (error) {
        console.error('[TaskPanel] Chat error:', error);
        throw error;
      }
    },
    [task, projectId, executingAgentName]
  );

  // Generate initial prompt for assigned agent
  const initialPrompt = useMemo(() => {
    if (task.assigned_agent && workflowEvents.length === 0 && !workflowLoading) {
      return `Please review this task and let me know your approach for completing it. The task is: "${task.title}"${task.description ? `\n\nDescription: ${task.description}` : ''}`;
    }
    return undefined;
  }, [task.assigned_agent, task.title, task.description, workflowEvents.length, workflowLoading]);

  // Refresh data
  const handleRefresh = useCallback(() => {
    // Trigger refetch by updating state
    setArtifactsLoading(true);
    setWorkflowLoading(true);

    taskArtifactsApi
      .list(task.id)
      .then((data) => {
        const executionArtifacts = data
          .map((item) => item.artifact)
          .filter((artifact): artifact is ApiExecutionArtifact => Boolean(artifact));
        const normalized: ExecutionArtifact[] = executionArtifacts.map((artifact) => ({
          id: artifact.id,
          execution_process_id: artifact.execution_process_id ?? '',
          artifact_type: artifact.artifact_type as ArtifactType,
          title: artifact.title ?? 'Untitled',
          content: artifact.content ?? null,
          file_path: artifact.file_path ?? null,
          metadata: artifact.metadata ?? null,
          created_at: artifact.created_at,
        }));
        setArtifacts(normalized);
      })
      .finally(() => setArtifactsLoading(false));

    agentFlowsApi
      .list({ task_id: task.id })
      .then(async (flows) => {
        if (!flows.length) {
          setWorkflowEvents([]);
          return;
        }
        const events = await agentFlowsApi.getEvents(flows[0].id);
        const normalized: AgentFlowEvent[] = events.map((event) => ({
          ...event,
          event_type: normalizeFlowEventType(event.event_type),
          event_data: event.event_data ?? '',
        }));
        setWorkflowEvents(normalized);
      })
      .finally(() => setWorkflowLoading(false));
  }, [task.id]);

  // Tab counts
  const artifactCount = artifacts.length;
  const eventCount = workflowEvents.length;

  return (
    <div
      className={cn(
        'flex flex-col h-full bg-background border-l',
        isFullscreen && 'fixed inset-0 z-50',
        className
      )}
    >
      {/* Header */}
      <EnhancedTaskHeader
        task={task}
        mode={mode}
        onEdit={onEdit}
        onDelete={onDelete}
        onDuplicate={onDuplicate}
        onClose={onClose}
        onToggleFullscreen={onToggleFullscreen}
        isFullscreen={isFullscreen}
        hideClose={hideClose}
      />

      {/* Tabs */}
      <Tabs
        value={activeTab}
        onValueChange={(v) => setActiveTab(v as DetailTab)}
        className="flex-1 flex flex-col min-h-0"
      >
        <div className="border-b px-4 flex items-center justify-between">
          <TabsList className="h-10 bg-transparent p-0">
            <TabsTrigger
              value="overview"
              className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
            >
              <LayoutGrid className="h-4 w-4 mr-2" />
              Overview
            </TabsTrigger>
            <TabsTrigger
              value="artifacts"
              className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
            >
              <FileText className="h-4 w-4 mr-2" />
              Artifacts
              {artifactCount > 0 && (
                <Badge variant="secondary" className="ml-2 h-5">
                  {artifactCount}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger
              value="workflow"
              className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
            >
              <Zap className="h-4 w-4 mr-2" />
              Workflow
            </TabsTrigger>
            <TabsTrigger
              value="activity"
              className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
            >
              <Clock className="h-4 w-4 mr-2" />
              Activity
              {eventCount > 0 && (
                <Badge variant="secondary" className="ml-2 h-5">
                  {eventCount}
                </Badge>
              )}
            </TabsTrigger>
          </TabsList>

          <Button variant="ghost" size="icon" onClick={handleRefresh} className="h-8 w-8">
            <RefreshCw className={cn('h-4 w-4', (artifactsLoading || workflowLoading) && 'animate-spin')} />
          </Button>
        </div>

        {/* Overview Tab */}
        <TabsContent value="overview" className="flex-1 m-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-6">
              {/* Description */}
              {task.description && (
                <div>
                  <h3 className="text-sm font-medium mb-2">Description</h3>
                  <p className="text-sm text-muted-foreground whitespace-pre-wrap">
                    {task.description}
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
                      onClick={() => setActiveTab('artifacts')}
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
                    onClick={() => setActiveTab('workflow')}
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
                    onSendMessage={handleSendMessage}
                    initialPrompt={initialPrompt}
                    executingAgentId={executingAgentId || undefined}
                    className="h-64 border rounded-lg overflow-hidden"
                  />
                )}
              </div>

              {/* Recent Activity */}
              <div>
                <div className="flex items-center justify-between mb-3">
                  <h3 className="text-sm font-medium">Recent Activity</h3>
                  <Button
                    variant="link"
                    size="sm"
                    className="h-auto p-0"
                    onClick={() => setActiveTab('activity')}
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
                    collaborators={task.collaborators}
                    chatMessages={chatMessages.slice(0, 10)}
                    className="h-48 border rounded-lg overflow-hidden"
                  />
                )}
              </div>
            </div>
          </ScrollArea>
        </TabsContent>

        {/* Artifacts Tab */}
        <TabsContent value="artifacts" className="flex-1 m-0 overflow-hidden">
          {artifactsLoading ? (
            <div className="p-4 space-y-4">
              <Skeleton className="h-10 w-full" />
              <div className="grid grid-cols-3 gap-4">
                <Skeleton className="h-32" />
                <Skeleton className="h-32" />
                <Skeleton className="h-32" />
                <Skeleton className="h-32" />
                <Skeleton className="h-32" />
                <Skeleton className="h-32" />
              </div>
            </div>
          ) : artifactsError ? (
            <div className="p-4">
              <Alert variant="destructive">
                <AlertDescription>{artifactsError}</AlertDescription>
              </Alert>
            </div>
          ) : (
            <ArtifactGallery
              artifacts={artifacts}
              className="h-full"
              onUpload={async () => {
                // TODO: Implement file upload
                console.log('File upload requested');
              }}
              onLinkAdd={async () => {
                // TODO: Implement link addition
                console.log('Add link requested');
              }}
            />
          )}
        </TabsContent>

        {/* Workflow Tab */}
        <TabsContent value="workflow" className="flex-1 m-0 overflow-hidden">
          {workflowLoading ? (
            <div className="p-4 space-y-4">
              <Skeleton className="h-16 w-full" />
              <Skeleton className="h-64 w-full" />
            </div>
          ) : workflowError ? (
            <div className="p-4">
              <Alert variant="destructive">
                <AlertDescription>{workflowError}</AlertDescription>
              </Alert>
            </div>
          ) : (
            <EnhancedWorkflowView
              events={workflowEvents}
              taskId={task.id}
              taskTitle={task.title}
              onSendMessage={handleSendMessage}
              initialPrompt={initialPrompt}
              executingAgentId={executingAgentId || undefined}
              className="h-full"
            />
          )}
        </TabsContent>

        {/* Activity Tab */}
        <TabsContent value="activity" className="flex-1 m-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-6">
              {/* Task Activity Log - shows task changes (status, assignments, etc.) */}
              <ActivityTimeline taskId={task.id} />

              {/* Workflow Collaboration - shows agent interactions and chat */}
              <div className="pt-4 border-t">
                <h3 className="text-lg font-semibold mb-4">
                  Workflow & Collaboration
                </h3>
                <CollaborationTimeline
                  events={workflowEvents}
                  collaborators={task.collaborators}
                  chatMessages={chatMessages}
                />
              </div>
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </div>
  );
}
