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
  Coins,
  TrendingUp,
  Cpu,
  CircleDollarSign,
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
import { agentFlowsApi, taskArtifactsApi, agentsApi, artifactContentApi } from '@/lib/api';
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

type DetailTab = 'overview' | 'artifacts' | 'workflow' | 'activity' | 'vibe';

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

  // Vibe tracking state
  const [vibeBalance, setVibeBalance] = useState<{
    total_deposited: number;
    total_withdrawn: number;
    total_spent: number;
    available_balance: number;
  } | null>(null);
  const [vibeTransactions, setVibeTransactions] = useState<Array<{
    id: string;
    amount_vibe: number;
    model: string | null;
    description: string | null;
    task_id: string | null;
    input_tokens: number | null;
    output_tokens: number | null;
    created_at: string;
  }>>([]);
  const [vibeLoading, setVibeLoading] = useState(false);

  // Detect card mode
  const mode = useMemo(() => detectCardMode(task, artifacts), [task, artifacts]);

  // Look up agent ID for message persistence
  useEffect(() => {
    const agentName = task.assigned_agent;
    if (!agentName || executingAgentId) return;

    agentsApi.getByName(agentName)
      .then((agent) => {
        setExecutingAgentId(agent.id);
      })
      .catch(() => {
      });
  }, [task.assigned_agent, executingAgentId]);

  // Fetch chat messages for activity log
  useEffect(() => {
    if (!executingAgentId) return;

    const sessionId = `task-${task.id}`;
    agentsApi.getConversationBySession(executingAgentId, sessionId)
      .then((result) => {
        if (result && result.messages) {
          setChatMessages(result.messages);
        }
      })
      .catch(() => {
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

  // Fetch VIBE balance when vibe tab is opened
  useEffect(() => {
    if (activeTab !== 'vibe' || !projectId) return;
    setVibeLoading(true);

    Promise.all([
      fetch(`/api/projects/${projectId}/vibe/balance`, { credentials: 'include' })
        .then(r => r.ok ? r.json() : null)
        .then(d => d?.data ?? null)
        .catch(() => null),
      fetch(`/api/projects/${projectId}/vibe/transactions?limit=50`, { credentials: 'include' })
        .then(r => r.ok ? r.json() : null)
        .then(d => Array.isArray(d?.data) ? d.data : [])
        .catch(() => []),
    ]).then(([balance, txns]) => {
      setVibeBalance(balance);
      // Filter to this task's transactions
      setVibeTransactions(txns.filter((t: any) => t.task_id === task.id));
    }).finally(() => setVibeLoading(false));
  }, [activeTab, projectId, task.id]);

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
          const agent = await agentsApi.getByName(targetAgentName);

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
            model: null,
            provider: null,
          };


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
          return data.content;
        } catch (error) {
          console.warn(`[TaskPanel] Agent chat failed for ${targetAgentName}:`, error);

          // If it's a payment error, don't fall back to Nora - just throw
          if (error instanceof Error && error.message.includes('VIBE')) {
            throw error;
          }

        }
      }

      // Fallback: route through Nora with timeout
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

  // Artifact download handler
  const handleArtifactDownload = useCallback((artifact: ExecutionArtifact) => {
    // For video/render types, try to download the actual media file
    if (['render_deliverable', 'video_edit_session'].includes(artifact.artifact_type) && artifact.content) {
      try {
        const data = JSON.parse(artifact.content);
        const files = data.deliverables || data.edits || [];
        if (files.length > 0) {
          const file = files[0].file || files[0].path?.split('/').pop();
          if (file) {
            const a = document.createElement('a');
            a.href = artifactContentApi.getFileUrl(artifact.id, file);
            a.download = file;
            a.style.display = 'none';
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            return;
          }
        }
      } catch { /* fall through */ }
    }
    // Fallback: download the JSON content
    const a = document.createElement('a');
    a.href = artifactContentApi.getDownloadUrl(artifact.id);
    a.download = '';
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  }, []);

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
              Logs
              {eventCount > 0 && (
                <Badge variant="secondary" className="ml-2 h-5">
                  {eventCount}
                </Badge>
              )}
            </TabsTrigger>
            <TabsTrigger
              value="vibe"
              className="data-[state=active]:bg-transparent data-[state=active]:shadow-none data-[state=active]:border-b-2 data-[state=active]:border-primary rounded-none px-4"
            >
              <Coins className="h-4 w-4 mr-2" />
              Vibe
              {task.vibe_cost && Number(task.vibe_cost) > 0 ? (
                <Badge variant="secondary" className="ml-2 h-5">
                  {Number(task.vibe_cost).toLocaleString()}
                </Badge>
              ) : null}
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
                    onDownload={handleArtifactDownload}
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
              onDownload={handleArtifactDownload}
              onUpload={async () => {
                // TODO: Implement file upload
              }}
              onLinkAdd={async () => {
                // TODO: Implement link addition
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

        {/* Activity / Logs Tab */}
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

        {/* Vibe Tracking Tab */}
        <TabsContent value="vibe" className="flex-1 m-0 overflow-hidden">
          <ScrollArea className="h-full">
            <div className="p-4 space-y-4">
              {/* Task-level VIBE cost */}
              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-lg border bg-card p-4 space-y-1">
                  <div className="flex items-center gap-2 text-muted-foreground text-xs font-medium">
                    <Coins className="h-3.5 w-3.5" />
                    VIBE Spent (Task)
                  </div>
                  <div className="text-2xl font-bold">
                    {task.vibe_cost ? Number(task.vibe_cost).toLocaleString() : '0'}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    ≈ ${task.vibe_cost ? (Number(task.vibe_cost) / 100).toFixed(2) : '0.00'} USD
                  </div>
                </div>
                <div className="rounded-lg border bg-card p-4 space-y-1">
                  <div className="flex items-center gap-2 text-muted-foreground text-xs font-medium">
                    <Cpu className="h-3.5 w-3.5" />
                    Model
                  </div>
                  <div className="text-sm font-semibold truncate">
                    {task.vibe_model ?? '—'}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {task.assigned_agent ?? 'No agent'}
                  </div>
                </div>
              </div>

              {/* Project VIBE balance */}
              <div>
                <h3 className="text-sm font-medium mb-2 flex items-center gap-2">
                  <TrendingUp className="h-4 w-4 text-muted-foreground" />
                  Project Budget
                </h3>
                {vibeLoading ? (
                  <div className="grid grid-cols-2 gap-3">
                    <div className="h-20 rounded-lg border bg-muted animate-pulse" />
                    <div className="h-20 rounded-lg border bg-muted animate-pulse" />
                  </div>
                ) : vibeBalance ? (
                  <div className="grid grid-cols-2 gap-3">
                    <div className="rounded-lg border bg-card p-3 space-y-1">
                      <div className="text-xs text-muted-foreground">Available Balance</div>
                      <div className={cn('text-xl font-bold', vibeBalance.available_balance < 0 && 'text-destructive')}>
                        {vibeBalance.available_balance.toLocaleString()}
                      </div>
                      <div className="text-xs text-muted-foreground">VIBE</div>
                    </div>
                    <div className="rounded-lg border bg-card p-3 space-y-1">
                      <div className="text-xs text-muted-foreground">Total Spent (Project)</div>
                      <div className="text-xl font-bold">
                        {vibeBalance.total_spent.toLocaleString()}
                      </div>
                      <div className="text-xs text-muted-foreground">VIBE</div>
                    </div>
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground">No budget data available.</p>
                )}
              </div>

              {/* Task transactions */}
              {vibeTransactions.length > 0 && (
                <div>
                  <h3 className="text-sm font-medium mb-2 flex items-center gap-2">
                    <CircleDollarSign className="h-4 w-4 text-muted-foreground" />
                    Task Transactions ({vibeTransactions.length})
                  </h3>
                  <div className="space-y-2">
                    {vibeTransactions.map((tx) => (
                      <div key={tx.id} className="flex items-center justify-between rounded-md border px-3 py-2 text-sm">
                        <div className="min-w-0">
                          <div className="truncate text-xs font-medium">
                            {tx.description ?? tx.model ?? 'AI execution'}
                          </div>
                          <div className="text-xs text-muted-foreground">
                            {tx.model && <span className="mr-2">{tx.model}</span>}
                            {tx.input_tokens != null && tx.output_tokens != null && (
                              <span>{tx.input_tokens.toLocaleString()} in / {tx.output_tokens.toLocaleString()} out</span>
                            )}
                          </div>
                        </div>
                        <div className="shrink-0 ml-3 text-right">
                          <div className="font-semibold text-orange-500">{tx.amount_vibe.toLocaleString()} V</div>
                          <div className="text-xs text-muted-foreground">
                            ${(tx.amount_vibe / 100).toFixed(2)}
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {!vibeLoading && vibeTransactions.length === 0 && (
                <p className="text-sm text-muted-foreground pt-2">No VIBE transactions recorded for this task yet.</p>
              )}
            </div>
          </ScrollArea>
        </TabsContent>
      </Tabs>
    </div>
  );
}
