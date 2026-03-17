import { useEffect, useState, useMemo, useCallback } from 'react';
import type {
  TaskWithAttemptStatus,
  ExecutionArtifact,
  AgentFlowEvent,
  ArtifactType,
} from 'shared/types';
import type { AgentChatRequest } from 'shared/types';
import type { TaskCardMode } from '../../EnhancedTaskCard';
import { agentFlowsApi, taskArtifactsApi, agentsApi, artifactContentApi, projectsApi } from '@/lib/api';
import type { ExecutionArtifact as ApiExecutionArtifact } from '@/lib/api';
import { resolveApiUrl } from '@/lib/api';
import { detectCardMode, normalizeFlowEventType } from './utils';

type DetailTab = 'overview' | 'artifacts' | 'workflow' | 'activity' | 'vibe';

interface UseTaskPanelDataParams {
  task: TaskWithAttemptStatus;
  projectId: string;
}

// Helper to wrap fetch with timeout
function fetchWithTimeout(url: string, options: RequestInit, timeoutMs: number): Promise<Response> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);
  return fetch(resolveApiUrl(url), { ...options, signal: controller.signal })
    .then((response) => {
      clearTimeout(timeoutId);
      return response;
    })
    .catch((error) => {
      clearTimeout(timeoutId);
      if (error instanceof Error && error.name === 'AbortError') {
        throw new Error(`Request timed out after ${timeoutMs / 1000} seconds`);
      }
      throw error;
    });
}

// Helper to extract user-friendly error message from API response
function extractErrorMessage(status: number, responseText: string): string {
  try {
    const data = JSON.parse(responseText);
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

  if (status === 402) return 'Insufficient VIBE balance. Please add more VIBE to continue.';
  if (status === 404) return 'Agent not found.';
  if (status === 429) return 'API rate limit exceeded. Please try again in a moment.';
  if (status === 500 || status === 502 || status === 503) {
    return 'Server error. The AI service may be temporarily unavailable.';
  }
  return responseText || `Request failed with status ${status}`;
}

// Normalize API artifacts to shared ExecutionArtifact type
function normalizeArtifacts(data: Awaited<ReturnType<typeof taskArtifactsApi.list>>): ExecutionArtifact[] {
  return data
    .map((item) => item.artifact)
    .filter((artifact): artifact is ApiExecutionArtifact => Boolean(artifact))
    .map((artifact) => ({
      id: artifact.id,
      execution_process_id: artifact.execution_process_id ?? '',
      artifact_type: artifact.artifact_type as ArtifactType,
      title: artifact.title ?? 'Untitled',
      content: artifact.content ?? null,
      file_path: artifact.file_path ?? null,
      metadata: artifact.metadata ?? null,
      created_at: artifact.created_at,
    }));
}

export function useTaskPanelData({ task, projectId }: UseTaskPanelDataParams) {
  const [activeTab, setActiveTab] = useState<DetailTab>('overview');
  const [compactMode, setCompactMode] = useState<boolean>(() => {
    try { return localStorage.getItem('orcha:task-detail-compact') === 'true'; } catch { return false; }
  });
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
  const mode: TaskCardMode = useMemo(() => detectCardMode(task, artifacts), [task, artifacts]);

  // Look up agent ID for message persistence
  useEffect(() => {
    const agentName = task.assigned_agent;
    if (!agentName || executingAgentId) return;

    agentsApi.getByName(agentName)
      .then((agent) => setExecutingAgentId(agent.id))
      .catch((err) => console.error(`Failed to look up agent "${agentName}":`, err));
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
      .catch((err) => console.error('Failed to fetch conversation history:', err));
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
        setArtifacts(normalizeArtifacts(data));
      })
      .catch((error) => {
        if (cancelled) return;
        setArtifacts([]);
        setArtifactsError(error instanceof Error ? error.message : 'Failed to load artifacts.');
      })
      .finally(() => {
        if (!cancelled) setArtifactsLoading(false);
      });

    return () => { cancelled = true; };
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
        if (!cancelled) setWorkflowLoading(false);
      });

    return () => { cancelled = true; };
  }, [task.id]);

  // Fetch VIBE balance when vibe tab is opened
  useEffect(() => {
    if (activeTab !== 'vibe' || !projectId) return;
    setVibeLoading(true);

    Promise.all([
      projectsApi.getVibeBalance(projectId).catch(() => null),
      projectsApi.getVibeTransactions(projectId, 50).catch(() => []),
    ]).then(([balance, txns]) => {
      setVibeBalance(balance);
      setVibeTransactions(txns.filter((t) => t.task_id === task.id));
    }).finally(() => setVibeLoading(false));
  }, [activeTab, projectId, task.id]);

  // Extract agent name from workflow events
  const executingAgentName = useMemo(() => {
    for (const event of workflowEvents) {
      try {
        const data = JSON.parse(event.event_data || '{}');
        if (data.agent_name) return data.agent_name as string;
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

      // Try agent-specific chat if we have an agent name
      if (targetAgentName) {
        try {
          const agent = await agentsApi.getByName(targetAgentName);

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
            if (response.status === 402) throw new Error(userMessage);
            throw new Error(`Agent chat failed: ${userMessage}`);
          }

          const data = await response.json();
          return data.content;
        } catch (error) {
          console.warn(`[TaskPanel] Agent chat failed for ${targetAgentName}:`, error);
          if (error instanceof Error && error.message.includes('VIBE')) throw error;
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
          60000
        );

        if (!response.ok) {
          const errorText = await response.text();
          console.error('[TaskPanel] Nora chat failed:', response.status, errorText);
          throw new Error(extractErrorMessage(response.status, errorText));
        }

        const data = await response.json();
        return data.content || data.message || data.response || 'Response received';
      } catch (error) {
        console.error('[TaskPanel] Chat error:', error);
        throw error;
      }
    },
    [task, projectId, executingAgentName, executingAgentId]
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
    setArtifactsLoading(true);
    setWorkflowLoading(true);

    taskArtifactsApi
      .list(task.id)
      .then((data) => setArtifacts(normalizeArtifacts(data)))
      .finally(() => setArtifactsLoading(false));

    agentFlowsApi
      .list({ task_id: task.id })
      .then(async (flows) => {
        if (!flows.length) {
          setWorkflowEvents([]);
          return;
        }
        const events = await agentFlowsApi.getEvents(flows[0].id);
        setWorkflowEvents(events.map((event) => ({
          ...event,
          event_type: normalizeFlowEventType(event.event_type),
          event_data: event.event_data ?? '',
        })));
      })
      .finally(() => setWorkflowLoading(false));
  }, [task.id]);

  // Artifact download handler
  const handleArtifactDownload = useCallback((artifact: ExecutionArtifact) => {
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
    const a = document.createElement('a');
    a.href = artifactContentApi.getDownloadUrl(artifact.id);
    a.download = '';
    a.style.display = 'none';
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
  }, []);

  const toggleCompactMode = useCallback(() => {
    setCompactMode((prev) => {
      const next = !prev;
      localStorage.setItem('orcha:task-detail-compact', String(next));
      if (next && activeTab !== 'overview') setActiveTab('overview');
      return next;
    });
  }, [activeTab]);

  return {
    activeTab,
    setActiveTab,
    compactMode,
    toggleCompactMode,
    mode,
    artifacts,
    artifactsLoading,
    artifactsError,
    workflowEvents,
    workflowLoading,
    workflowError,
    executingAgentId,
    chatMessages,
    vibeBalance,
    vibeTransactions,
    vibeLoading,
    initialPrompt,
    handleSendMessage,
    handleRefresh,
    handleArtifactDownload,
  };
}
