import { useState, useEffect, useRef } from 'react';
import { makeRequest } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import {
  Network,
  X,
  Send,
  Mic,
  MicOff,
  Phone,
  PhoneOff,
  Volume2,
  VolumeX,
  Loader2,
  Users,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import { MeetingMode } from './meeting-mode';
import { TopsiConnectionStatus } from './TopsiConnectionStatus';
import { useTopsiVoice } from './hooks/useTopsiVoice';
import { useAgentChatStore } from '@/stores/useAgentChatStore';
import { useActivityStore } from '@/stores/useActivityStore';

interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  hasAudio?: boolean;
}

interface TopsiWidgetProps {
  className?: string;
}

const SUGGESTED_PROMPTS = [
  { label: 'Summarize my tasks', text: 'Summarize my current tasks and priorities' },
  { label: 'Deals needing attention', text: 'What deals in my pipeline need attention today?' },
  { label: 'Draft a status update', text: 'Draft a status update for my active projects' },
  { label: 'Recent activity', text: 'What happened across my projects this week?' },
] as const;

export function TopsiWidget({ className }: TopsiWidgetProps) {
  // Widget state — driven by global store
  const widgetState = useAgentChatStore((s) => s.widgetState);
  const setWidgetState = useAgentChatStore((s) => s.setWidgetState);
  const pendingMessage = useAgentChatStore((s) => s.pendingMessage);
  const pendingContext = useAgentChatStore((s) => s.pendingContext);
  const clearPending = useAgentChatStore((s) => s.clearPending);
  const collapseChat = useAgentChatStore((s) => s.collapse);
  const logActivity = useActivityStore((s) => s.logActivity);

  const [isInitialized, setIsInitialized] = useState(false);
  const isInitializedRef = useRef(false);
  const [isInitializing, setIsInitializing] = useState(false);

  // Chat state
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputMessage, setInputMessage] = useState('');
  const [isSending, setIsSending] = useState(false);
  const [sessionId] = useState(() => `topsi-widget-${Date.now()}`);

  // Meeting state (set when voice-activated)
  const [meetingProjectId, setMeetingProjectId] = useState<string | undefined>();

  // Refs
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const workflowPollsRef = useRef<Map<string, ReturnType<typeof setInterval>>>(new Map());
  const sendMessageDirectRef = useRef<(message: string, context?: typeof pendingContext) => Promise<void>>();

  // Helper to add messages
  const addMessage = (role: 'user' | 'assistant', content: string, hasAudio?: boolean) => {
    const message: ChatMessage = {
      id: crypto.randomUUID(),
      role,
      content,
      timestamp: new Date(),
      hasAudio,
    };
    setMessages(prev => [...prev, message]);
  };

  // Voice hook
  const [voiceState, voiceActions] = useTopsiVoice({
    sessionId,
    onUserMessage: (content) => addMessage('user', content),
    onAssistantMessage: (content, hasAudio) => addMessage('assistant', content, hasAudio),
    onActionComplete: (responseText) => {
      window.dispatchEvent(new CustomEvent('topsi-action-complete', {
        detail: { responseText, timestamp: new Date() }
      }));
    },
    onMeetingAction: (projectId) => {
      if (projectId) setMeetingProjectId(projectId);
      setWidgetState('meeting');
    },
  });

  // Connection status
  const connectionState = isInitializing ? 'connecting' : isInitialized ? 'connected' : 'disconnected';

  // Auto-scroll to bottom
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Check Topsi status on mount
  useEffect(() => {
    checkTopsiStatus();
  }, []);

  // Handle pending message from store (e.g. from AskTopsiButton)
  useEffect(() => {
    if (!pendingMessage || widgetState !== 'chat') return;

    const msg = pendingMessage;
    const ctx = pendingContext;
    clearPending();

    const sendPending = async () => {
      if (!isInitializedRef.current) {
        await initializeTopsi();
      }
      setInputMessage('');
      sendMessageDirectRef.current?.(msg, ctx);
    };

    sendPending();
  }, [pendingMessage, widgetState]);

  const checkTopsiStatus = async () => {
    try {
      const res = await makeRequest('/api/topsi/status');
      if (res.ok) {
        const data = await res.json();
        setIsInitialized(data.isActive);
        isInitializedRef.current = data.isActive;
      }
    } catch (error) {
      console.error('Failed to check Topsi status:', error);
    }
  };

  const initializeTopsi = async () => {
    setIsInitializing(true);
    try {
      const res = await makeRequest('/api/topsi/initialize', {
        method: 'POST',
        body: JSON.stringify({ activateImmediately: true }),
      });
      if (res.ok) {
        setIsInitialized(true);
        isInitializedRef.current = true;
        addMessage('assistant', "Hello! I'm Topsi, your platform orchestrator. How can I help you today?");
      } else {
        toast.error('Failed to initialize Topsi');
      }
    } catch (error) {
      console.error('Failed to initialize Topsi:', error);
      toast.error('Failed to connect to Topsi');
    } finally {
      setIsInitializing(false);
    }
  };

  // Core message send logic
  const sendMessageDirect = async (message: string, context?: typeof pendingContext) => {
    if (!message.trim() || isSending) return;

    addMessage('user', message);
    setIsSending(true);

    try {
      const res = await makeRequest('/api/topsi/chat', {
        method: 'POST',
        body: JSON.stringify({
          message,
          sessionId,
          ...(context ? { context } : {}),
        }),
      });

      if (res.ok) {
        const data = await res.json();
        const responseData = data.data || data;
        const responseText = responseData.message || 'I received your message.';

        addMessage('assistant', responseText);

        const entityId = context?.entityId || 'agent-global';
        if (responseData.tool_calls && Array.isArray(responseData.tool_calls)) {
          for (const tc of responseData.tool_calls) {
            const toolName = tc.name || tc.tool || 'unknown';
            logActivity(
              entityId,
              'agent_tool_call',
              `Topsi: ${toolName}`,
              {
                agent: 'topsi',
                tool: toolName,
                success: tc.success !== false,
                args: JSON.stringify(tc.args || tc.input || {}).slice(0, 200),
              }
            );

            if ((toolName === 'trigger_workflow' || toolName === 'build_workflow') && tc.result?.workflow_run_id) {
              startWorkflowPolling(tc.result.workflow_run_id, entityId);
            }
          }
        }

        window.dispatchEvent(new CustomEvent('topsi-action-complete', {
          detail: { responseText, timestamp: new Date() }
        }));
      } else {
        const errorBody = await res.text().catch(() => '');
        console.error('Topsi message failed:', res.status, errorBody);
        addMessage('assistant', 'Sorry, I encountered an error processing your request.');
      }
    } catch (error) {
      console.error('Failed to send message:', error);
      addMessage('assistant', 'Sorry, I lost connection. Please try again.');
    } finally {
      setIsSending(false);
    }
  };

  useEffect(() => {
    sendMessageDirectRef.current = sendMessageDirect;
  }, [sendMessageDirect]);

  const sendTextMessage = async () => {
    if (!inputMessage.trim() || isSending) return;
    const msg = inputMessage.trim();
    setInputMessage('');
    await sendMessageDirect(msg);
  };

  // Workflow polling
  const startWorkflowPolling = (runId: string, entityId: string = 'agent-global') => {
    const taskId = entityId;

    logActivity(taskId, 'agent_workflow_triggered', `Workflow run ${runId} started`, {
      agent: 'topsi',
      workflow_run_id: runId,
    });

    addMessage('assistant', `Workflow running... (${runId})`);

    let pollCount = 0;
    const maxPolls = 60;

    const interval = setInterval(async () => {
      pollCount++;
      try {
        const res = await makeRequest(`/api/workflows/runs/${runId}`);
        if (!res.ok) return;
        const run = await res.json();
        const status = run.status || run.data?.status;

        if (status === 'completed' || status === 'failed' || status === 'cancelled') {
          clearInterval(interval);
          workflowPollsRef.current.delete(runId);

          const summary = status === 'completed'
            ? `Workflow completed (${run.records_staged ?? run.data?.records_staged ?? '?'} records staged)`
            : `Workflow ${status}`;
          addMessage('assistant', summary);

          logActivity(taskId, 'agent_workflow_completed', summary, {
            agent: 'topsi',
            workflow_run_id: runId,
            status,
          });
        } else if (pollCount >= maxPolls) {
          clearInterval(interval);
          workflowPollsRef.current.delete(runId);
          addMessage('assistant', `Workflow polling timed out after 5 minutes (${runId}). Check status manually.`);
        }
      } catch {
        // Silently retry on next interval
      }
    }, 5000);

    workflowPollsRef.current.set(runId, interval);
  };

  // Call mode handlers that set widget state
  const handleStartCall = async () => {
    await voiceActions.startCall();
    setWidgetState('call');
  };

  const handleEndCall = () => {
    voiceActions.endCall();
    setWidgetState('chat');
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      voiceActions.cleanup();
      workflowPollsRef.current.forEach((interval) => clearInterval(interval));
      workflowPollsRef.current.clear();
    };
  }, []);

  const openWidget = () => {
    if (!isInitialized) {
      initializeTopsi();
    }
    useAgentChatStore.getState().openChat();
  };

  // Collapsed state - just the floating button
  if (widgetState === 'collapsed') {
    return (
      <div className={cn("fixed bottom-6 right-6 z-50", className)}>
        <Button
          onClick={openWidget}
          title="Open Topsi AI Assistant"
          className="h-14 w-14 rounded-full bg-cyan-600 hover:bg-cyan-700 shadow-lg hover:shadow-xl transition-all"
          disabled={isInitializing}
        >
          {isInitializing ? (
            <Loader2 className="h-6 w-6 animate-spin" />
          ) : (
            <Network className="h-6 w-6" />
          )}
        </Button>
        <TopsiConnectionStatus state={connectionState} className="absolute -top-1 -right-1" />
      </div>
    );
  }

  // Expanded widget
  return (
    <div className={cn(
      "fixed bottom-6 right-6 z-50 flex flex-col bg-background border rounded-xl shadow-2xl transition-all",
      widgetState === 'call' ? "w-80 h-96" : "w-96 h-[32rem]",
      className
    )}>
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b bg-cyan-600 text-white rounded-t-xl">
        <div className="flex items-center gap-2">
          <Network className="h-5 w-5" />
          <span className="font-semibold">Topsi</span>
          <TopsiConnectionStatus state={connectionState} />
          {voiceState.isInCall && (
            <Badge variant="secondary" className="bg-green-500 text-white text-xs">
              On Call
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-1">
          {widgetState === 'chat' && (
            <>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-white hover:bg-cyan-700"
                onClick={() => setWidgetState('meeting')}
                title="Meeting mode"
              >
                <Users className="h-4 w-4" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8 text-white hover:bg-cyan-700"
                onClick={handleStartCall}
                title="Start voice call"
                disabled={connectionState === 'disconnected'}
              >
                <Phone className="h-4 w-4" />
              </Button>
            </>
          )}
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-white hover:bg-cyan-700"
            onClick={collapseChat}
            title="Close"
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      {/* Call Mode UI */}
      {widgetState === 'call' && (
        <div className="flex-1 flex flex-col items-center justify-center p-6 gap-6">
          {/* Audio visualization */}
          <div className="relative">
            <div className={cn(
              "w-24 h-24 rounded-full bg-cyan-100 dark:bg-cyan-900 flex items-center justify-center transition-all",
              voiceState.isRecording && "ring-4 ring-cyan-400 ring-opacity-50"
            )}
            style={{
              transform: `scale(${1 + voiceState.audioLevel * 0.3})`,
            }}>
              <Network className="h-12 w-12 text-cyan-600" />
            </div>
            {voiceState.isProcessingVoice && (
              <div className="absolute inset-0 flex items-center justify-center">
                <Loader2 className="h-8 w-8 animate-spin text-cyan-600" />
              </div>
            )}
          </div>

          <div className="text-center">
            <p className="text-sm text-muted-foreground">
              {voiceState.isProcessingVoice ? 'Processing...' : voiceState.isRecording ? 'Listening...' : voiceState.isMuted ? 'Muted' : 'Ready'}
            </p>
          </div>

          {/* Call controls */}
          <div className="flex items-center gap-4">
            <Button
              variant={voiceState.isMuted ? "destructive" : "outline"}
              size="icon"
              className="h-12 w-12 rounded-full"
              onClick={voiceActions.toggleMute}
              title={voiceState.isMuted ? 'Unmute' : 'Mute'}
            >
              {voiceState.isMuted ? <MicOff className="h-5 w-5" /> : <Mic className="h-5 w-5" />}
            </Button>

            <Button
              variant="destructive"
              size="icon"
              className="h-14 w-14 rounded-full"
              onClick={handleEndCall}
              title="End call"
            >
              <PhoneOff className="h-6 w-6" />
            </Button>

            <Button
              variant={voiceState.isSpeakerOn ? "outline" : "secondary"}
              size="icon"
              className="h-12 w-12 rounded-full"
              onClick={voiceActions.toggleSpeaker}
              title={voiceState.isSpeakerOn ? 'Mute speaker' : 'Unmute speaker'}
            >
              {voiceState.isSpeakerOn ? <Volume2 className="h-5 w-5" /> : <VolumeX className="h-5 w-5" />}
            </Button>
          </div>

          {/* Switch to chat */}
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setWidgetState('chat')}
            className="text-muted-foreground"
          >
            Switch to text chat
          </Button>
        </div>
      )}

      {/* Chat Mode UI */}
      {widgetState === 'chat' && (
        <>
          {/* Messages */}
          <ScrollArea className="flex-1 p-3">
            <div className="space-y-3">
              {messages.length === 0 && (
                <div className="text-center text-muted-foreground py-6">
                  <Network className="h-10 w-10 mx-auto mb-3 opacity-30" />
                  <p className="text-sm mb-3">Start a conversation with Topsi</p>
                  <div className="flex flex-wrap justify-center gap-1.5">
                    {SUGGESTED_PROMPTS.map((prompt) => (
                      <button
                        key={prompt.label}
                        type="button"
                        className="px-2.5 py-1 rounded-full border border-border/60 bg-muted/50 text-xs text-foreground hover:bg-muted hover:border-border transition-colors"
                        onClick={() => setInputMessage(prompt.text)}
                      >
                        {prompt.label}
                      </button>
                    ))}
                  </div>
                </div>
              )}
              {messages.map((msg) => (
                <div
                  key={msg.id}
                  className={cn(
                    "flex",
                    msg.role === 'user' ? "justify-end" : "justify-start"
                  )}
                >
                  <div
                    className={cn(
                      "max-w-[85%] rounded-lg px-3 py-2 text-sm",
                      msg.role === 'user'
                        ? "bg-cyan-600 text-white"
                        : "bg-muted"
                    )}
                  >
                    <p className="whitespace-pre-wrap">{msg.content}</p>
                    <div className={cn(
                      "flex items-center gap-2 mt-1 text-xs",
                      msg.role === 'user' ? "text-cyan-100" : "text-muted-foreground"
                    )}>
                      <span>{msg.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                      {msg.hasAudio && msg.role === 'assistant' && (
                        <Volume2 className="h-3 w-3" />
                      )}
                    </div>
                  </div>
                </div>
              ))}
              {(isSending || voiceState.isProcessingVoice) && (
                <div className="flex justify-start">
                  <div className="bg-muted rounded-lg px-3 py-2">
                    <Loader2 className="h-4 w-4 animate-spin" />
                  </div>
                </div>
              )}
              <div ref={messagesEndRef} />
            </div>
          </ScrollArea>

          {/* Input area */}
          <div className="p-3 border-t">
            {/* Audio level indicator when recording */}
            {voiceState.isRecording && (
              <div className="mb-2">
                <div className="h-1 bg-gray-200 rounded-full overflow-hidden">
                  <div
                    className="h-full bg-cyan-500 transition-all duration-100"
                    style={{ width: `${voiceState.audioLevel * 100}%` }}
                  />
                </div>
              </div>
            )}

            <div className="flex items-center gap-2">
              {/* Push-to-talk button */}
              <Button
                variant={voiceState.isRecording ? "destructive" : "outline"}
                size="icon"
                className="h-10 w-10 shrink-0"
                onMouseDown={voiceActions.handlePushToTalkStart}
                onMouseUp={voiceActions.handlePushToTalkEnd}
                onMouseLeave={voiceActions.handlePushToTalkEnd}
                onTouchStart={voiceActions.handlePushToTalkStart}
                onTouchEnd={voiceActions.handlePushToTalkEnd}
                disabled={voiceState.isProcessingVoice || connectionState === 'disconnected'}
                title={connectionState === 'disconnected' ? 'Connect to Topsi first' : 'Hold to talk'}
              >
                {voiceState.isRecording ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
              </Button>

              {/* Text input */}
              <Input
                placeholder="Type a message..."
                value={inputMessage}
                onChange={(e) => setInputMessage(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && !e.shiftKey && sendTextMessage()}
                disabled={isSending || voiceState.isRecording}
                className="flex-1"
              />

              {/* Send button */}
              <Button
                size="icon"
                className="h-10 w-10 shrink-0 bg-cyan-600 hover:bg-cyan-700"
                onClick={sendTextMessage}
                disabled={isSending || !inputMessage.trim()}
                title="Send message"
              >
                {isSending ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Send className="h-4 w-4" />
                )}
              </Button>
            </div>

            {/* Quick actions */}
            <div className="flex items-center justify-between mt-2 text-xs text-muted-foreground">
              <span>Hold mic for push-to-talk</span>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 text-xs"
                onClick={handleStartCall}
                disabled={connectionState === 'disconnected'}
              >
                <Phone className="h-3 w-3 mr-1" />
                Start call
              </Button>
            </div>
          </div>
        </>
      )}

      {/* Meeting Mode UI */}
      {widgetState === 'meeting' && (
        <MeetingMode
          projectId={meetingProjectId}
          onClose={() => {
            setMeetingProjectId(undefined);
            setWidgetState('chat');
          }}
          className="flex-1"
        />
      )}
    </div>
  );
}

export default TopsiWidget;
