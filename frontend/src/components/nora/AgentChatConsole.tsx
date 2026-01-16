import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { ScrollArea } from '@/components/ui/scroll-area';
import { SlashCommandMenu, SlashCommand, useSlashCommands } from '@/components/slash-commands/SlashCommandMenu';
import { cn } from '@/lib/utils';
import { useAgentDirectory } from '@/hooks/useAgentDirectory';
import type { AgentCoordinationState, CoordinationEvent } from '@/types/nora';
import { Activity, Bot, Crown, MapPin, MessageSquare, Navigation, Radio, Terminal } from 'lucide-react';
import { useMultiplayerStore } from '@/stores/useMultiplayerStore';

interface AgentChatConsoleProps {
  className?: string;
  statusLine?: string;
  statusVersion?: number;
  selectedProject?: { name: string; energy: number } | null;
  isInputActive: boolean;
  onRequestCloseInput: () => void;
  focusToken: number;
  showHeader?: boolean;
  extraSystemMessage?: string | null;
  extraSystemMessageVersion?: number;
}

interface ConsoleMessage {
  id: string;
  channel: 'global' | 'system' | 'direct';
  author: 'user' | 'agent' | 'system';
  label: string;
  content: string;
  timestamp: Date;
  agentId?: string;
}

interface NoraApiResponse {
  content: string;
  responseId: string;
  actions: { actionId: string; actionType: string; description: string }[];
  followUpSuggestions: string[];
  timestamp: string;
}

interface AgentDirectiveResponse {
  agentId: string;
  agentLabel: string;
  acknowledgement: string;
  echoedCommand: string;
  priority?: string | null;
  timestamp: string;
}

const channelColorMap: Record<ConsoleMessage['channel'], string> = {
  global: 'text-emerald-200',
  system: 'text-cyan-200',
  direct: 'text-purple-200',
};

const authorColorMap: Record<ConsoleMessage['author'], string> = {
  user: 'text-white',
  agent: 'text-blue-100',
  system: 'text-cyan-100',
};

export function AgentChatConsole({
  className,
  statusLine,
  statusVersion,
  selectedProject,
  isInputActive,
  onRequestCloseInput,
  focusToken,
  showHeader = true,
  extraSystemMessage,
  extraSystemMessageVersion,
}: AgentChatConsoleProps) {
  const { agents, socketConnected, lastEvent } = useAgentDirectory();
  const { spawnPreference, setSpawnPreference, teleport } = useMultiplayerStore();
  const [messages, setMessages] = useState<ConsoleMessage[]>(() => [
    {
      id: 'boot',
      channel: 'system',
      author: 'system',
      label: 'System',
      content: 'Command center chat console online. Type /help for commands.',
      timestamp: new Date(),
    },
  ]);
  const [draft, setDraft] = useState('');
  const [isSending, setIsSending] = useState(false);
  const sessionIdRef = useRef(`console-${Date.now()}`);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const noraReadyRef = useRef(false);

  const slashState = useSlashCommands();

  const agentCommandMap = useMemo(() => {
    const map = new Map<string, AgentCoordinationState>();
    agents.forEach((agent) => {
      map.set(agent.agentId.toLowerCase(), agent);
      map.set(agent.agentType.toLowerCase(), agent);
    });
    return map;
  }, [agents]);

  const slashCommands: SlashCommand[] = useMemo(() => {
    const base: SlashCommand[] = [
      {
        id: 'nora',
        label: '/nora',
        description: 'Direct message Nora',
        icon: Crown,
        keywords: ['executive', 'assistant'],
        action: () => setDraft('/nora '),
      },
      {
        id: 'global',
        label: '/global',
        description: 'Broadcast to everyone nearby',
        icon: Terminal,
        keywords: ['broadcast', 'say'],
        action: () => setDraft('/global '),
      },
      {
        id: 'spawnpoint',
        label: '/spawnpoint',
        description: 'Set your spawn location',
        icon: MapPin,
        keywords: ['spawn', 'home', 'default'],
        action: () => setDraft('/spawnpoint '),
      },
      {
        id: 'teleport',
        label: '/teleport',
        description: 'Teleport to a location',
        icon: Navigation,
        keywords: ['tp', 'warp', 'goto'],
        action: () => setDraft('/teleport '),
      },
      {
        id: 'help',
        label: '/help',
        description: 'Show command help',
        icon: MessageSquare,
        keywords: ['commands', 'guide'],
        action: () => setDraft('/help'),
      },
    ];

    const dynamic = agents.map((agent) => ({
      id: agent.agentId.toLowerCase(),
      label: `/${agent.agentId.toLowerCase()}`,
      description: agent.agentType,
      icon: Bot,
      keywords: agent.capabilities,
      action: () => setDraft(`/${agent.agentId.toLowerCase()} `),
    }));

    return [...base, ...dynamic];
  }, [agents, setDraft]);

  const scrollToBottom = useCallback(() => {
    if (listRef.current) {
      listRef.current.scrollTo({
        top: listRef.current.scrollHeight,
      });
    }
  }, []);

  useEffect(() => {
    scrollToBottom();
  }, [messages, scrollToBottom]);

  useEffect(() => {
    if (!isInputActive) {
      slashState.closeMenu();
      inputRef.current?.blur();
      return;
    }
    const timeout = window.setTimeout(() => {
      inputRef.current?.focus();
    }, 16);
    return () => window.clearTimeout(timeout);
  }, [isInputActive, focusToken, slashState]);

  const pushMessage = useCallback((message: Omit<ConsoleMessage, 'id' | 'timestamp'> & { timestamp?: Date }) => {
    setMessages((prev) => {
      const randomId =
        typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
          ? crypto.randomUUID()
          : Math.random().toString(36);

      const next: ConsoleMessage[] = [
        ...prev,
        {
          id: randomId,
          timestamp: message.timestamp ?? new Date(),
          ...message,
        },
      ];
      return next.slice(-150);
    });
  }, []);

  const pushSystemMessage = useCallback(
    (content: string) => {
      if (!content) return;
      pushMessage({ channel: 'system', author: 'system', label: 'System', content });
    },
    [pushMessage]
  );

  const formatEvent = useCallback((event: CoordinationEvent) => {
    switch (event.type) {
      case 'AgentStatusUpdate':
        return `${event.agentId} is now ${event.status}.`;
      case 'TaskHandoff':
        return `Task ${event.taskId} handed from ${event.fromAgent} to ${event.toAgent}.`;
      case 'ConflictResolution':
        return `Conflict ${event.conflictId} escalated (${event.priority}).`;
      case 'ExecutiveAlert':
        return `Executive alert from ${event.source}: ${event.message}`;
      case 'ApprovalRequest':
        return `Approval requested by ${event.requestingAgent}: ${event.actionDescription}`;
      case 'HumanAvailabilityUpdate':
        return `${event.userId} is now ${event.availability}.`;
      case 'AgentDirective':
        return `${event.issuedBy} issued directive to ${event.agentId}: ${event.content}`;
      default:
        return null;
    }
  }, []);

  useEffect(() => {
    if (!lastEvent) return;
    const description = formatEvent(lastEvent);
    if (description) {
      pushSystemMessage(description);
    }
  }, [lastEvent, formatEvent, pushSystemMessage]);

  useEffect(() => {
    if (statusLine && statusVersion) {
      pushSystemMessage(statusLine);
    }
  }, [statusLine, statusVersion, pushSystemMessage]);

  useEffect(() => {
    if (extraSystemMessage && extraSystemMessageVersion) {
      pushSystemMessage(extraSystemMessage);
    }
  }, [extraSystemMessage, extraSystemMessageVersion, pushSystemMessage]);

  const ensureNoraReady = useCallback(async () => {
    if (noraReadyRef.current) return true;
    try {
      const statusResp = await fetch('/api/nora/status');
      if (statusResp.ok) {
        const status = (await statusResp.json()) as { isActive?: boolean };
        if (status?.isActive) {
          noraReadyRef.current = true;
          return true;
        }
      }
    } catch (error) {
      console.warn('Nora status check failed', error);
    }

    try {
      const initResp = await fetch('/api/nora/initialize', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ activateImmediately: true }),
      });
      if (initResp.ok) {
        noraReadyRef.current = true;
        return true;
      }
    } catch (error) {
      console.warn('Nora initialization failed', error);
    }
    return false;
  }, []);

  const formatNoraContent = (response: NoraApiResponse) => {
    let text = response.content;
    if (response.actions && response.actions.length > 0) {
      const details = response.actions
        .map((action) => `• ${action.actionType}: ${action.description}`)
        .join('\n');
      text += `\n\n${details}`;
    }
    if (response.followUpSuggestions && response.followUpSuggestions.length > 0) {
      text += `\n\nSuggestions: ${response.followUpSuggestions.join(', ')}`;
    }
    return text;
  };

  const submitNoraCommand = useCallback(
    async (payload: string) => {
      pushMessage({
        channel: 'direct',
        author: 'user',
        label: 'You → Nora',
        content: payload,
      });

      setIsSending(true);
      try {
        const ready = await ensureNoraReady();
        if (!ready) {
          throw new Error('Nora link offline. Initialization failed.');
        }

        const response = await fetch('/api/nora/chat', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            message: payload,
            sessionId: sessionIdRef.current,
            requestType: 'textInteraction',
            voiceEnabled: false,
            priority: 'normal',
            context: null,
          }),
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Nora responded with ${response.status}`);
        }

        const data = (await response.json()) as NoraApiResponse;
        pushMessage({
          channel: 'direct',
          author: 'agent',
          label: 'Nora',
          content: formatNoraContent(data),
        });
      } catch (error) {
        console.error(error);
        pushSystemMessage(
          `Nora couldn’t process that directive${
            error instanceof Error ? ` (${error.message})` : ''
          }.`
        );
      } finally {
        setIsSending(false);
      }
    },
    [ensureNoraReady, pushMessage, pushSystemMessage]
  );

  const handleAgentDirective = useCallback(
    async (agent: AgentCoordinationState, payload: string, commandKey: string) => {
      pushMessage({
        channel: 'direct',
        author: 'user',
        label: `You → ${agent.agentId}`,
        content: payload,
        agentId: agent.agentId,
      });

      setIsSending(true);
      try {
        const response = await fetch(`/api/nora/coordination/agents/${agent.agentId}/directives`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            sessionId: sessionIdRef.current,
            content: payload,
            command: commandKey,
            priority: null,
            context: null,
          }),
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(errorText || `Directive failed with ${response.status}`);
        }

        const data = (await response.json()) as AgentDirectiveResponse;
        pushMessage({
          channel: 'direct',
          author: 'agent',
          label: data.agentLabel,
          content: data.acknowledgement,
          agentId: data.agentId,
        });
      } catch (error) {
        console.error(error);
        pushSystemMessage(`Unable to reach ${agent.agentId}. ${
          error instanceof Error ? error.message : ''
        }`);
      } finally {
        setIsSending(false);
      }
    },
    [pushMessage, pushSystemMessage]
  );

  const handleHelp = useCallback(() => {
    pushSystemMessage('Slash commands: /nora, /global, /spawnpoint, /teleport, /help, and /<agent>. Example: /nora execute sprint retro.');
  }, [pushSystemMessage]);

  const handleSpawnpoint = useCallback(
    (payload: string) => {
      if (!payload) {
        // Show current spawn preference
        if (spawnPreference) {
          pushSystemMessage(`Current spawn point: ${spawnPreference}. Use /spawnpoint <project-slug> to change.`);
        } else {
          pushSystemMessage('No spawn point set. Use /spawnpoint <project-slug> to set one (e.g., /spawnpoint omega-wireless).');
        }
        return;
      }
      // Set new spawn preference
      const slug = payload.toLowerCase().replace(/\s+/g, '-');
      setSpawnPreference(slug);
      pushSystemMessage(`Spawn point set to: ${slug}. You will spawn here on next login.`);
    },
    [spawnPreference, setSpawnPreference, pushSystemMessage]
  );

  const handleTeleport = useCallback(
    (payload: string) => {
      if (!payload) {
        pushSystemMessage('Usage: /teleport <location>. Examples: /teleport command-center, /teleport omega-wireless');
        return;
      }
      const destination = payload.toLowerCase().replace(/\s+/g, '-');
      teleport(destination);
      pushSystemMessage(`Teleporting to ${destination}...`);
    },
    [teleport, pushSystemMessage]
  );

  const handleGlobalBroadcast = useCallback(
    (payload: string) => {
      pushMessage({
        channel: 'global',
        author: 'user',
        label: 'You',
        content: payload,
      });
    },
    [pushMessage]
  );

  const routeSlashCommand = useCallback(
    async (input: string) => {
      const withoutSlash = input.slice(1);
      const [rawCommand, ...rest] = withoutSlash.split(/\s+/);
      const commandKey = rawCommand?.toLowerCase() ?? '';
      const payload = rest.join(' ').trim();

      switch (commandKey) {
        case 'nora':
          if (!payload) {
            pushSystemMessage('Give Nora something to do after /nora.');
            return;
          }
          await submitNoraCommand(payload);
          return;
        case 'global':
          if (!payload) {
            pushSystemMessage('Nothing to broadcast.');
            return;
          }
          handleGlobalBroadcast(payload);
          return;
        case 'spawnpoint':
        case 'spawn':
          handleSpawnpoint(payload);
          return;
        case 'teleport':
        case 'tp':
          handleTeleport(payload);
          return;
        case 'help':
          handleHelp();
          return;
        default: {
          const agent = agentCommandMap.get(commandKey);
          if (agent) {
            if (!payload) {
              pushSystemMessage(`Describe the task for /${commandKey}.`);
              return;
            }
            await handleAgentDirective(agent, payload, commandKey);
          } else {
            pushSystemMessage(`Unknown agent or command: /${commandKey}`);
          }
        }
      }
    },
    [agentCommandMap, handleAgentDirective, handleGlobalBroadcast, handleHelp, handleSpawnpoint, handleTeleport, submitNoraCommand, pushSystemMessage]
  );

  const handleSubmit = useCallback(async (): Promise<boolean> => {
    const value = draft.trim();
    if (!value) return false;

    if (value.startsWith('/')) {
      await routeSlashCommand(value);
    } else {
      handleGlobalBroadcast(value);
    }
    setDraft('');
    return true;
  }, [draft, handleGlobalBroadcast, routeSlashCommand]);

  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    if (!isInputActive) {
      event.preventDefault();
      return;
    }
    const value = slashState.handleInputChange(event);
    setDraft(value);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>) => {
    if (!isInputActive) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      onRequestCloseInput();
      return;
    }
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      if (!draft.trim()) {
        onRequestCloseInput();
        return;
      }
      // Release input immediately so avatar can move while Nora processes
      onRequestCloseInput();
      void handleSubmit();
    }
  };

  const handleButtonSend = () => {
    if (!isInputActive || !draft.trim()) {
      onRequestCloseInput();
      return;
    }
    // Release input immediately so avatar can move while Nora processes
    onRequestCloseInput();
    void handleSubmit();
  };

  const activeAgents = useMemo(
    () => agents.filter((agent) => agent.status !== 'offline'),
    [agents]
  );

  return (
    <Card className={cn('bg-slate-900/80 border-cyan-500/30 text-sm text-slate-100', className)}>
      {showHeader && (
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="flex items-center gap-2 text-base">
              <Crown className="h-4 w-4 text-cyan-300" />
              Command Net
            </CardTitle>
            <Badge variant={socketConnected ? 'default' : 'secondary'} className="flex items-center gap-1 text-[10px]">
              <Radio className="h-3 w-3" />
              {socketConnected ? 'LIVE LINK' : 'RECONNECTING'}
            </Badge>
          </div>
          {statusLine && (
            <p className="text-xs text-cyan-200/80 mt-1 flex items-center gap-2">
              <Activity className="h-3 w-3" />
              {statusLine}
            </p>
          )}
        </CardHeader>
      )}
      <CardContent className={cn('space-y-3', !showHeader && 'pt-4')}>
        <ScrollArea className="h-56 rounded border border-cyan-500/10 bg-black/30">
          <div className="p-3 space-y-2" ref={listRef}>
            {messages.map((message) => (
              <div key={message.id} className="flex gap-2 text-[13px] leading-relaxed">
                <span className={cn('font-mono text-xs', channelColorMap[message.channel])}>
                  [{message.channel.toUpperCase()}]
                </span>
                <span className={cn('font-semibold', authorColorMap[message.author])}>{message.label}</span>
                <span className="text-slate-400">
                  {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                </span>
                <span className="text-slate-100">{message.content}</span>
              </div>
            ))}
          </div>
        </ScrollArea>

        <div>
          <div className="flex items-center gap-2">
            <Input
              ref={inputRef}
              value={draft}
              onChange={handleInputChange}
              onKeyDown={handleKeyDown}
              placeholder={isInputActive ? 'Type /nora or talk to the grid...' : 'Press Enter to engage the command net'}
              disabled={isSending || !isInputActive}
              id="command-net-input"
              name="command-net-input"
              className="bg-black/60 border-cyan-500/30 text-sm"
            />
            <Button size="icon" onClick={handleButtonSend} disabled={isSending || !isInputActive}>
              <SendIcon className="h-4 w-4" />
            </Button>
          </div>
          <div className="mt-2 flex flex-wrap gap-2 text-[11px] text-cyan-200/70">
            <span>{isInputActive ? 'Enter sends · Esc exits typing' : 'Enter to open · Esc to cancel'}</span>
            <span>Slash tips: /nora, /global, /help, /&lt;agent&gt;</span>
          </div>
        </div>

        {activeAgents.length > 0 && (
          <div className="space-y-1">
            <p className="text-[11px] uppercase tracking-[0.3em] text-cyan-300">Nearby Agents</p>
            <div className="flex flex-wrap gap-2">
              {activeAgents.map((agent) => (
                <button
                  key={agent.agentId}
                  type="button"
                  className="text-xs px-2 py-1 rounded-full border border-cyan-500/40 bg-black/30 hover:bg-cyan-500/10 transition"
                  onClick={() => setDraft(`/${agent.agentId.toLowerCase()} `)}
                >
                  {agent.agentId}
                </button>
              ))}
            </div>
          </div>
        )}

        {selectedProject && (
          <div className="border-t border-cyan-500/10 pt-3">
            <p className="text-[11px] uppercase tracking-[0.25em] text-cyan-200/80">Linked Project</p>
            <div className="flex items-center justify-between text-xs text-cyan-100">
              <span className="font-semibold">{selectedProject.name}</span>
              <span className="font-mono text-cyan-300">{(selectedProject.energy * 100).toFixed(1)}%</span>
            </div>
            <p className="text-[11px] text-cyan-200/70 mt-1">Synchronized with MCP timeline feed.</p>
          </div>
        )}
      </CardContent>

      <SlashCommandMenu
        open={isInputActive && slashState.isOpen}
        search={slashState.search}
        onSelect={(command) => slashState.handleCommandSelect(command, draft, setDraft)}
        onClose={slashState.closeMenu}
        position={slashState.position}
        commands={slashCommands}
      />
    </Card>
  );
}

function SendIcon({ className, ...props }: React.ComponentProps<'svg'>) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={cn('h-4 w-4', className)}
      {...props}
    >
      <path
        d="M4 4l16 8-16 8 4-8-4-8z"
        stroke="currentColor"
        strokeWidth={2}
        strokeLinejoin="round"
        strokeLinecap="round"
      />
    </svg>
  );
}
