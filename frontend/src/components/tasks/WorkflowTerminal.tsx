import { useState, useRef, useEffect, useMemo } from 'react';
import { Card, CardContent } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Terminal,
  Send,
  Bot,
  CheckCircle2,
  Loader2,
  XCircle,
  ChevronRight,
  ChevronDown,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { AgentFlowEvent } from 'shared/types';

interface WorkflowTerminalProps {
  events: AgentFlowEvent[];
  taskId: string;
  onSendMessage?: (message: string, agentName?: string) => Promise<string>;
  className?: string;
}

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

interface ParsedEventData {
  type?: string;
  phase?: string;
  description?: string;
  expected_output?: string;
  output?: string;
  stage_index?: number;
  total_stages?: number;
  duration_ms?: number;
  agent_id?: string;
  agent_name?: string;
  error?: string;
  message?: string;
}

function parseEventData(event: AgentFlowEvent): ParsedEventData {
  try {
    return JSON.parse(event.event_data || '{}');
  } catch {
    return {};
  }
}

function formatTime(dateString: string): string {
  const date = new Date(dateString);
  return date.toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

function TerminalLine({
  prefix,
  content,
  className,
  expandable = false,
  defaultExpanded = false,
}: {
  prefix?: string;
  content: string;
  className?: string;
  expandable?: boolean;
  defaultExpanded?: boolean;
}) {
  const [expanded, setExpanded] = useState(defaultExpanded);
  const isLong = content.length > 200;
  const shouldTruncate = expandable && isLong && !expanded;

  return (
    <div className={cn('font-mono text-sm leading-relaxed', className)}>
      {prefix && (
        <span className="text-amber-500 mr-2">{prefix}</span>
      )}
      {expandable && isLong && (
        <button
          onClick={() => setExpanded(!expanded)}
          className="inline-flex items-center text-cyan-400 hover:text-cyan-300 mr-1"
        >
          {expanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      )}
      <span className={cn(shouldTruncate && 'line-clamp-2')}>
        {content}
      </span>
    </div>
  );
}

function StageBlock({
  stageName,
  stageIndex,
  totalStages,
  description,
  output,
  status,
  timestamp,
  agentName,
  durationMs,
}: {
  stageName: string;
  stageIndex: number;
  totalStages: number;
  description?: string;
  output?: string;
  status: 'started' | 'completed' | 'failed';
  timestamp: string;
  agentName?: string;
  durationMs?: number;
}) {
  const [expanded, setExpanded] = useState(status === 'completed');

  const statusIcon = {
    started: <Loader2 className="h-4 w-4 animate-spin text-blue-400" />,
    completed: <CheckCircle2 className="h-4 w-4 text-green-400" />,
    failed: <XCircle className="h-4 w-4 text-red-400" />,
  }[status];

  const statusColor = {
    started: 'border-blue-500/50',
    completed: 'border-green-500/50',
    failed: 'border-red-500/50',
  }[status];

  return (
    <div className={cn('border-l-2 pl-4 py-3 mb-2', statusColor)}>
      {/* Stage header */}
      <button
        onClick={() => setExpanded(!expanded)}
        className="flex items-center gap-2 w-full text-left hover:opacity-80"
      >
        {statusIcon}
        <span className="font-mono font-semibold text-cyan-300">
          [{stageIndex + 1}/{totalStages}] {stageName}
        </span>
        {durationMs && (
          <span className="text-xs text-gray-500 ml-auto">
            {(durationMs / 1000).toFixed(1)}s
          </span>
        )}
        {expanded ? (
          <ChevronDown className="h-4 w-4 text-gray-500" />
        ) : (
          <ChevronRight className="h-4 w-4 text-gray-500" />
        )}
      </button>

      {/* Agent info */}
      {agentName && (
        <div className="flex items-center gap-1 mt-1 ml-6 text-xs text-gray-500">
          <Bot className="h-3 w-3" />
          <span>{agentName}</span>
          <span className="mx-1">|</span>
          <span>{formatTime(timestamp)}</span>
        </div>
      )}

      {/* Expanded content */}
      {expanded && (
        <div className="mt-3 ml-6 space-y-3">
          {/* Description */}
          {description && (
            <div>
              <span className="text-xs uppercase text-gray-500 font-semibold">Task:</span>
              <p className="text-sm text-gray-300 mt-1">{description}</p>
            </div>
          )}

          {/* Output */}
          {output && (
            <div>
              <span className="text-xs uppercase text-gray-500 font-semibold">Output:</span>
              <pre className="text-sm text-green-300 mt-1 whitespace-pre-wrap bg-black/30 p-3 rounded overflow-x-auto max-h-64 overflow-y-auto">
                {output}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export function WorkflowTerminal({
  events,
  taskId: _taskId,
  onSendMessage,
  className,
}: WorkflowTerminalProps) {
  const [inputValue, setInputValue] = useState('');
  const [sending, setSending] = useState(false);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Group events by stage
  const stages = useMemo(() => {
    const stageMap = new Map<string, {
      name: string;
      index: number;
      total: number;
      description?: string;
      output?: string;
      status: 'started' | 'completed' | 'failed';
      timestamp: string;
      agentName?: string;
      durationMs?: number;
    }>();

    const sortedEvents = [...events].sort(
      (a, b) => new Date(a.created_at).getTime() - new Date(b.created_at).getTime()
    );

    for (const event of sortedEvents) {
      const data = parseEventData(event);
      const phase = data.phase;

      if (!phase || phase === 'execution') continue;

      const key = phase;

      if (event.event_type === 'phase_started') {
        stageMap.set(key, {
          name: phase,
          index: data.stage_index ?? 0,
          total: data.total_stages ?? 1,
          description: data.description,
          status: 'started',
          timestamp: event.created_at,
          agentName: data.agent_name,
        });
      } else if (event.event_type === 'phase_completed') {
        const existing = stageMap.get(key);
        if (existing) {
          stageMap.set(key, {
            ...existing,
            output: data.output,
            status: 'completed',
            durationMs: data.duration_ms,
          });
        } else {
          // Handle case where phase_completed comes without phase_started
          stageMap.set(key, {
            name: phase,
            index: data.stage_index ?? 0,
            total: data.total_stages ?? 1,
            description: data.description,
            output: data.output,
            status: 'completed',
            timestamp: event.created_at,
            agentName: data.agent_name,
            durationMs: data.duration_ms,
          });
        }
      } else if (event.event_type === 'flow_failed') {
        const existing = stageMap.get(key);
        if (existing) {
          stageMap.set(key, {
            ...existing,
            status: 'failed',
            output: data.error || data.message,
          });
        }
      }
    }

    return Array.from(stageMap.values()).sort((a, b) => a.index - b.index);
  }, [events]);

  // Check if workflow is complete
  const isComplete = useMemo(() => {
    return events.some(e => e.event_type === 'flow_completed' || e.event_type === 'flow_failed');
  }, [events]);

  // Extract agent name from the first event that has it
  const executingAgentName = useMemo(() => {
    for (const event of events) {
      const data = parseEventData(event);
      if (data.agent_name) return data.agent_name;
    }
    return undefined;
  }, [events]);

  // Auto-scroll to bottom on new events or chat messages
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [events, chatMessages]);

  const handleSend = async () => {
    if (!inputValue.trim() || sending || !onSendMessage) return;

    const userMessage = inputValue.trim();
    setSending(true);
    setInputValue('');

    // Add user message immediately
    const userMsgId = `user-${Date.now()}`;
    setChatMessages(prev => [...prev, {
      id: userMsgId,
      role: 'user',
      content: userMessage,
      timestamp: new Date(),
    }]);

    try {
      const response = await onSendMessage(userMessage, executingAgentName);
      // Add assistant response
      setChatMessages(prev => [...prev, {
        id: `assistant-${Date.now()}`,
        role: 'assistant',
        content: response,
        timestamp: new Date(),
      }]);
    } catch (error) {
      // Add error message
      setChatMessages(prev => [...prev, {
        id: `error-${Date.now()}`,
        role: 'assistant',
        content: `Error: ${error instanceof Error ? error.message : 'Failed to get response'}`,
        timestamp: new Date(),
      }]);
    } finally {
      setSending(false);
    }
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <Card className={cn('bg-gray-950 border-gray-800', className)}>
      {/* Terminal header */}
      <div className="flex items-center gap-2 px-4 py-2 border-b border-gray-800 bg-gray-900/50">
        <Terminal className="h-4 w-4 text-green-400" />
        <span className="text-sm font-semibold text-gray-300">
          {executingAgentName ? `${executingAgentName} Workflow` : 'Workflow Terminal'}
        </span>
        <div className="flex-1" />
        <span className="text-xs text-gray-500">
          {stages.length} stages | {events.length} events
        </span>
      </div>

      {/* Terminal content */}
      <ScrollArea className="h-[400px]" ref={scrollRef}>
        <CardContent className="p-4 space-y-1">
          {/* Workflow start message */}
          {stages.length > 0 && (
            <TerminalLine
              prefix="$"
              content={`Starting workflow execution...`}
              className="text-green-400 mb-4"
            />
          )}

          {/* Stage blocks */}
          {stages.map((stage, idx) => (
            <StageBlock
              key={`${stage.name}-${idx}`}
              stageName={stage.name}
              stageIndex={stage.index}
              totalStages={stage.total}
              description={stage.description}
              output={stage.output}
              status={stage.status}
              timestamp={stage.timestamp}
              agentName={stage.agentName}
              durationMs={stage.durationMs}
            />
          ))}

          {/* Completion message */}
          {isComplete && (
            <TerminalLine
              prefix="$"
              content={events.some(e => e.event_type === 'flow_completed')
                ? 'Workflow completed successfully.'
                : 'Workflow failed. Check logs above for details.'}
              className={events.some(e => e.event_type === 'flow_completed')
                ? 'text-green-400 mt-4'
                : 'text-red-400 mt-4'}
            />
          )}

          {/* Chat messages */}
          {chatMessages.length > 0 && (
            <div className="mt-4 pt-4 border-t border-gray-800 space-y-3">
              {chatMessages.map((msg) => (
                <div key={msg.id} className="font-mono text-sm">
                  {msg.role === 'user' ? (
                    <div className="flex items-start gap-2">
                      <span className="text-cyan-400 shrink-0">{'>'}</span>
                      <span className="text-gray-200">{msg.content}</span>
                    </div>
                  ) : (
                    <div className="flex items-start gap-2 ml-4">
                      <Bot className="h-4 w-4 text-purple-400 shrink-0 mt-0.5" />
                      <span className="text-gray-300 whitespace-pre-wrap">{msg.content}</span>
                    </div>
                  )}
                </div>
              ))}
              {sending && (
                <div className="flex items-center gap-2 ml-4 text-gray-500">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  <span className="text-sm">{executingAgentName || 'Nora'} is thinking...</span>
                </div>
              )}
            </div>
          )}

          {/* Empty state */}
          {events.length === 0 && chatMessages.length === 0 && (
            <div className="text-center py-8 text-gray-500">
              <Terminal className="h-12 w-12 mx-auto mb-2 opacity-50" />
              <p>No workflow events yet</p>
              <p className="text-xs mt-1">Events will appear here as the workflow runs</p>
            </div>
          )}
        </CardContent>
      </ScrollArea>

      {/* Input area */}
      <div className="border-t border-gray-800 p-3 bg-gray-900/50">
        <div className="flex items-center gap-2">
          <span className="text-green-400 font-mono text-sm">{'>'}</span>
          <Input
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder={
              isComplete
                ? `Ask ${executingAgentName || 'the agent'} to refine or expand on results...`
                : `Message ${executingAgentName || 'the agent'}...`
            }
            className="flex-1 bg-transparent border-none focus-visible:ring-0 focus-visible:ring-offset-0 text-gray-200 placeholder:text-gray-600 font-mono text-sm"
            disabled={sending}
          />
          <Button
            size="sm"
            variant="ghost"
            onClick={handleSend}
            disabled={!inputValue.trim() || sending || !onSendMessage}
            className="text-cyan-400 hover:text-cyan-300 hover:bg-gray-800"
          >
            {sending ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
          </Button>
        </div>
        <p className="text-xs text-gray-600 mt-2 ml-5">
          Press Enter to send{executingAgentName ? ` to ${executingAgentName}` : ''}. Ask follow-up questions or request refinements.
        </p>
      </div>
    </Card>
  );
}
