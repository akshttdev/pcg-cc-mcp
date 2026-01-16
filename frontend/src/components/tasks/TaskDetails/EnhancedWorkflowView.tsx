import { useState, useRef, useEffect, useMemo } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Send,
  Bot,
  User,
  CheckCircle2,
  Loader2,
  XCircle,
  Clock,
  Zap,
  Terminal,
  ArrowRight,
  FileText,
  AlertTriangle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { agentsApi } from '@/lib/api';
import type { AgentFlowEvent } from 'shared/types';

interface EnhancedWorkflowViewProps {
  events: AgentFlowEvent[];
  taskId: string;
  taskTitle: string;
  onSendMessage?: (message: string, agentName?: string) => Promise<string>;
  initialPrompt?: string;
  className?: string;
  /** Agent ID for loading persisted conversation */
  executingAgentId?: string;
}

// Unified stream entry - can be a workflow event or a chat message
interface StreamEntry {
  id: string;
  timestamp: Date;
  type: 'workflow' | 'user' | 'agent';
  // For workflow events
  eventType?: string;
  phase?: string;
  description?: string;
  output?: string;
  status?: 'started' | 'completed' | 'failed' | 'info';
  agentName?: string;
  durationMs?: number;
  // For chat messages
  content?: string;
}

function parseEventData(eventData: string): Record<string, unknown> {
  try {
    return JSON.parse(eventData || '{}');
  } catch {
    return {};
  }
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.floor((ms % 60000) / 1000)}s`;
}

function formatTime(date: Date): string {
  return date.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

// Single stream entry renderer
function StreamEntryItem({ entry }: { entry: StreamEntry }) {
  if (entry.type === 'user') {
    return (
      <div className="flex gap-3 py-2">
        <div className="w-6 h-6 rounded-full bg-green-600 flex items-center justify-center shrink-0">
          <User className="h-3.5 w-3.5 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 text-xs text-gray-400 mb-1">
            <span className="font-medium text-green-400">You</span>
            <span>{formatTime(entry.timestamp)}</span>
          </div>
          <div className="text-sm text-gray-200 whitespace-pre-wrap">{entry.content}</div>
        </div>
      </div>
    );
  }

  if (entry.type === 'agent') {
    return (
      <div className="flex gap-3 py-2">
        <div className="w-6 h-6 rounded-full bg-blue-600 flex items-center justify-center shrink-0">
          <Bot className="h-3.5 w-3.5 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 text-xs text-gray-400 mb-1">
            <span className="font-medium text-blue-400">{entry.agentName || 'Agent'}</span>
            <span>{formatTime(entry.timestamp)}</span>
          </div>
          <div className="text-sm text-gray-200 whitespace-pre-wrap">{entry.content}</div>
        </div>
      </div>
    );
  }

  // Workflow event
  const getEventIcon = () => {
    switch (entry.eventType) {
      case 'phase_started':
        return <Zap className="h-3.5 w-3.5 text-blue-400" />;
      case 'phase_completed':
        return <CheckCircle2 className="h-3.5 w-3.5 text-green-400" />;
      case 'flow_completed':
        return <CheckCircle2 className="h-3.5 w-3.5 text-green-400" />;
      case 'flow_failed':
        return <XCircle className="h-3.5 w-3.5 text-red-400" />;
      case 'artifact_created':
      case 'artifact_updated':
        return <FileText className="h-3.5 w-3.5 text-purple-400" />;
      case 'agent_handoff':
        return <ArrowRight className="h-3.5 w-3.5 text-amber-400" />;
      case 'approval_requested':
        return <AlertTriangle className="h-3.5 w-3.5 text-orange-400" />;
      default:
        return <Clock className="h-3.5 w-3.5 text-gray-400" />;
    }
  };

  const getEventColor = () => {
    switch (entry.status) {
      case 'completed':
        return 'text-green-400';
      case 'failed':
        return 'text-red-400';
      case 'started':
        return 'text-blue-400';
      default:
        return 'text-gray-400';
    }
  };

  return (
    <div className="flex gap-3 py-1.5">
      <div className="w-6 flex items-start justify-center pt-0.5 shrink-0">
        {getEventIcon()}
      </div>
      <div className="flex-1 min-w-0 font-mono text-xs">
        <div className="flex items-center gap-2">
          <span className="text-gray-500">{formatTime(entry.timestamp)}</span>
          <span className={cn('font-medium', getEventColor())}>
            [{entry.eventType}]
          </span>
          {entry.agentName && (
            <span className="text-cyan-400">{entry.agentName}</span>
          )}
        </div>
        {entry.phase && (
          <div className="text-gray-300 mt-0.5">
            {entry.phase}
            {entry.durationMs && (
              <span className="text-gray-500 ml-2">({formatDuration(entry.durationMs)})</span>
            )}
          </div>
        )}
        {entry.description && (
          <div className="text-gray-400 mt-0.5">{entry.description}</div>
        )}
        {entry.output && (
          <div className="mt-2 p-2 bg-gray-900/50 rounded border border-gray-800 text-gray-300 whitespace-pre-wrap max-h-48 overflow-auto">
            {entry.output}
          </div>
        )}
      </div>
    </div>
  );
}

export function EnhancedWorkflowView({
  events,
  taskId,
  taskTitle,
  onSendMessage,
  initialPrompt,
  className,
  executingAgentId,
}: EnhancedWorkflowViewProps) {
  const [chatMessages, setChatMessages] = useState<StreamEntry[]>([]);
  const [inputValue, setInputValue] = useState('');
  const [sending, setSending] = useState(false);
  const [hasAutoPrompted, setHasAutoPrompted] = useState(false);
  const [messagesLoaded, setMessagesLoaded] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  // Session ID for this task's conversation
  const sessionId = `task-${taskId}`;

  // Load persisted conversation messages on mount
  useEffect(() => {
    if (!executingAgentId || messagesLoaded) return;

    const loadPersistedMessages = async () => {
      try {
        console.log('[WorkflowView] Loading persisted messages for agent:', executingAgentId, 'session:', sessionId);
        const result = await agentsApi.getConversationBySession(executingAgentId, sessionId);

        if (result && result.messages && result.messages.length > 0) {
          console.log('[WorkflowView] Found', result.messages.length, 'persisted messages');

          // Convert to StreamEntry format
          const loadedMessages: StreamEntry[] = result.messages.map((msg) => ({
            id: msg.id,
            timestamp: new Date(msg.createdAt),
            type: msg.role === 'user' ? 'user' : 'agent',
            content: msg.content,
            agentName: msg.role === 'assistant' ? 'Agent' : undefined,
          }));

          setChatMessages(loadedMessages);
          setHasAutoPrompted(true); // Don't auto-prompt if we have history
        }
      } catch (error) {
        console.error('[WorkflowView] Failed to load persisted messages:', error);
      } finally {
        setMessagesLoaded(true);
      }
    };

    loadPersistedMessages();
  }, [executingAgentId, sessionId, messagesLoaded]);

  // Convert workflow events to stream entries
  const workflowEntries = useMemo((): StreamEntry[] => {
    return events.map((event) => {
      const data = parseEventData(event.event_data);

      let status: StreamEntry['status'] = 'info';
      if (event.event_type === 'phase_started') status = 'started';
      if (event.event_type === 'phase_completed' || event.event_type === 'flow_completed') status = 'completed';
      if (event.event_type === 'flow_failed') status = 'failed';

      return {
        id: event.id,
        timestamp: new Date(event.created_at),
        type: 'workflow',
        eventType: event.event_type,
        phase: data.phase as string | undefined,
        description: data.description as string | undefined,
        output: data.output as string | undefined,
        status,
        agentName: data.agent_name as string | undefined,
        durationMs: data.duration_ms as number | undefined,
      };
    });
  }, [events]);

  // Merge workflow entries and chat messages, sorted by timestamp
  const allEntries = useMemo(() => {
    return [...workflowEntries, ...chatMessages].sort(
      (a, b) => a.timestamp.getTime() - b.timestamp.getTime()
    );
  }, [workflowEntries, chatMessages]);

  // Extract executing agent name
  const executingAgentName = useMemo(() => {
    for (const entry of workflowEntries) {
      if (entry.agentName) return entry.agentName;
    }
    return null;
  }, [workflowEntries]);

  // Check if workflow is active
  const isActive = useMemo(() => {
    const hasStarted = events.some(e => e.event_type === 'phase_started');
    const hasEnded = events.some(e => e.event_type === 'flow_completed' || e.event_type === 'flow_failed');
    return hasStarted && !hasEnded;
  }, [events]);

  // Auto-prompt assigned agent when there's an initial prompt and no activity
  useEffect(() => {
    if (initialPrompt && !hasAutoPrompted && onSendMessage && chatMessages.length === 0) {
      setHasAutoPrompted(true);
      console.log('[WorkflowView] Auto-sending initial prompt to agent');

      // Add user message to show what was sent
      const userMessage: StreamEntry = {
        id: `auto-${Date.now()}`,
        timestamp: new Date(),
        type: 'user',
        content: initialPrompt,
      };
      setChatMessages([userMessage]);
      setSending(true);

      // Send the prompt
      onSendMessage(initialPrompt, executingAgentName || undefined)
        .then((response) => {
          const agentMessage: StreamEntry = {
            id: `agent-${Date.now()}`,
            timestamp: new Date(),
            type: 'agent',
            content: response,
            agentName: executingAgentName || 'Agent',
          };
          setChatMessages(prev => [...prev, agentMessage]);
        })
        .catch((error) => {
          console.error('[WorkflowView] Auto-prompt failed:', error);
          const errorMsg = error instanceof Error ? error.message : 'Failed to get response';
          const lowerMsg = errorMsg.toLowerCase();
          const isVibeError = lowerMsg.includes('vibe');
          const isQuotaError = lowerMsg.includes('quota') || lowerMsg.includes('rate limit') || lowerMsg.includes('exceeded');

          let content: string;
          let agentName: string;

          if (isVibeError) {
            content = `⚠️ ${errorMsg}\n\nTo continue chatting with agents, please add VIBE to your project budget.`;
            agentName = 'System (VIBE Balance)';
          } else if (isQuotaError) {
            content = `⚠️ ${errorMsg}\n\nThe AI service quota has been reached. Please check your API billing settings.`;
            agentName = 'System (API Quota)';
          } else {
            content = `Error: ${errorMsg}`;
            agentName = 'System';
          }

          const errorMessage: StreamEntry = {
            id: `error-${Date.now()}`,
            timestamp: new Date(),
            type: 'agent',
            content,
            agentName,
          };
          setChatMessages(prev => [...prev, errorMessage]);
        })
        .finally(() => {
          setSending(false);
        });
    }
  }, [initialPrompt, hasAutoPrompted, onSendMessage, chatMessages.length, executingAgentName]);

  // Auto-scroll to bottom when new entries arrive
  useEffect(() => {
    if (scrollRef.current) {
      const scrollContainer = scrollRef.current.querySelector('[data-radix-scroll-area-viewport]');
      if (scrollContainer) {
        scrollContainer.scrollTop = scrollContainer.scrollHeight;
      }
    }
  }, [allEntries.length]);

  const handleSend = async () => {
    if (!inputValue.trim() || sending || !onSendMessage) return;

    const userMessage: StreamEntry = {
      id: `user-${Date.now()}`,
      timestamp: new Date(),
      type: 'user',
      content: inputValue.trim(),
    };

    setChatMessages(prev => [...prev, userMessage]);
    const messageToSend = inputValue.trim();
    setInputValue('');
    setSending(true);

    try {
      const response = await onSendMessage(messageToSend, executingAgentName || undefined);

      const agentMessage: StreamEntry = {
        id: `agent-${Date.now()}`,
        timestamp: new Date(),
        type: 'agent',
        content: response,
        agentName: executingAgentName || 'Agent',
      };

      setChatMessages(prev => [...prev, agentMessage]);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : 'Failed to get response';
      const lowerMsg = errorMsg.toLowerCase();
      const isVibeError = lowerMsg.includes('vibe');
      const isQuotaError = lowerMsg.includes('quota') || lowerMsg.includes('rate limit') || lowerMsg.includes('exceeded');

      let content: string;
      let agentName: string;

      if (isVibeError) {
        content = `⚠️ ${errorMsg}\n\nTo continue chatting with agents, please add VIBE to your project budget.`;
        agentName = 'System (VIBE Balance)';
      } else if (isQuotaError) {
        content = `⚠️ ${errorMsg}\n\nThe AI service quota has been reached. Please check your API billing settings.`;
        agentName = 'System (API Quota)';
      } else {
        content = `Error: ${errorMsg}`;
        agentName = 'System';
      }

      const errorMessage: StreamEntry = {
        id: `error-${Date.now()}`,
        timestamp: new Date(),
        type: 'agent',
        content,
        agentName,
      };
      setChatMessages(prev => [...prev, errorMessage]);
    } finally {
      setSending(false);
      inputRef.current?.focus();
    }
  };

  return (
    <div className={cn('flex flex-col h-full bg-gray-950 text-gray-100', className)}>
      {/* Terminal header */}
      <div className="flex items-center gap-3 px-4 py-2 bg-gray-900/80 border-b border-gray-800">
        <div className="flex gap-1.5">
          <div className={cn(
            "w-3 h-3 rounded-full",
            isActive ? "bg-green-500" : events.some(e => e.event_type === 'flow_failed') ? "bg-red-500" : "bg-gray-600"
          )} />
          <div className="w-3 h-3 rounded-full bg-gray-600" />
          <div className="w-3 h-3 rounded-full bg-gray-600" />
        </div>
        <div className="flex-1 flex items-center gap-2">
          <Terminal className="h-4 w-4 text-gray-400" />
          <span className="text-sm font-medium text-gray-300">
            {executingAgentName || 'Agent'} — {taskTitle}
          </span>
        </div>
        {isActive && (
          <div className="flex items-center gap-2 text-xs text-green-400">
            <Loader2 className="h-3 w-3 animate-spin" />
            Working...
          </div>
        )}
      </div>

      {/* Unified stream */}
      <ScrollArea className="flex-1" ref={scrollRef}>
        <div className="p-4 space-y-1">
          {allEntries.length === 0 ? (
            <div className="text-center py-12 text-gray-500">
              <Terminal className="h-12 w-12 mx-auto mb-3 opacity-50" />
              <p className="text-sm">No activity yet</p>
              <p className="text-xs mt-1">
                Agent work and your messages will appear here
              </p>
            </div>
          ) : (
            allEntries.map((entry) => (
              <StreamEntryItem key={entry.id} entry={entry} />
            ))
          )}
          {sending && (
            <div className="flex gap-3 py-2">
              <div className="w-6 h-6 rounded-full bg-blue-600 flex items-center justify-center shrink-0">
                <Loader2 className="h-3.5 w-3.5 text-white animate-spin" />
              </div>
              <div className="flex-1">
                <span className="text-sm text-gray-400">
                  {executingAgentName || 'Agent'} is thinking...
                </span>
              </div>
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Input area */}
      <div className="border-t border-gray-800 bg-gray-900/50 p-3">
        <div className="flex items-center gap-2">
          <span className="text-green-400 font-mono">❯</span>
          <Input
            ref={inputRef}
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder={`Message ${executingAgentName || 'the agent'}...`}
            className="flex-1 bg-transparent border-none text-gray-100 placeholder:text-gray-600 focus-visible:ring-0 font-mono"
            disabled={sending || !onSendMessage}
          />
          <Button
            onClick={handleSend}
            disabled={!inputValue.trim() || sending || !onSendMessage}
            size="sm"
            className="bg-blue-600 hover:bg-blue-700 text-white"
          >
            {sending ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </Button>
        </div>
        {!onSendMessage && (
          <p className="text-xs text-gray-500 mt-2 ml-5">
            Messaging is not available for this task
          </p>
        )}
      </div>
    </div>
  );
}
