import {
  AlertTriangle,
  Bot,
  ChevronDown,
  ExternalLink,
  Mic,
  MicOff,
  Radio,
  Send,
  Zap,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';

import { useAuth } from '@/contexts/AuthContext';
import { resolveApiUrl } from '@/lib/api/client';
import { topsiApi } from '@/lib/api/topsi';
import { cn } from '@/lib/utils';

import { type OrbState, OrbVisualizer } from './OrbVisualizer';

// ── Nora types ────────────────────────────────────────────────────────────────

type NoraResponseType =
  | 'DirectResponse'
  | 'TaskDelegation'
  | 'StrategyRecommendation'
  | 'PerformanceInsight'
  | 'DecisionSupport'
  | 'CoordinationAction'
  | 'ProactiveAlert';

interface ExecutiveAction {
  actionId: string;
  actionType: string;
  description: string;
  parameters: Record<string, unknown>;
  requiresApproval: boolean;
  estimatedDuration?: string;
  assignedTo?: string;
}

interface NoraResponse {
  content: string;
  voiceResponse?: string;
  responseType: NoraResponseType;
  planId?: string;
  actions: ExecutiveAction[];
  followUpSuggestions: string[];
  processingTimeMs: number;
}

interface Message {
  id: string;
  role: 'user' | 'assistant';
  content: string;
}

// ── Web Speech shim ───────────────────────────────────────────────────────────

interface SRAlternative {
  transcript: string;
}
interface SRResultItem {
  isFinal: boolean;
  length: number;
  [i: number]: SRAlternative;
}
interface SREvent {
  resultIndex: number;
  results: ArrayLike<SRResultItem>;
}
interface SpeechRecognition extends EventTarget {
  lang: string;
  continuous: boolean;
  interimResults: boolean;
  onresult: ((e: SREvent) => void) | null;
  onerror: ((e: Event) => void) | null;
  onend: ((e: Event) => void) | null;
  start: () => void;
  stop: () => void;
}
declare global {
  interface Window {
    SpeechRecognition?: new () => SpeechRecognition;
    webkitSpeechRecognition?: new () => SpeechRecognition;
  }
}

// ── Constants ─────────────────────────────────────────────────────────────────

function getGreeting(name: string, isAdmin: boolean) {
  const h = new Date().getHours();
  const t =
    h < 12 ? 'Good morning' : h < 17 ? 'Good afternoon' : 'Good evening';
  return isAdmin
    ? `${t}. I'm Nora, your executive orchestrator. How may I assist you today?`
    : `${t}, ${name}. I'm Topsi, your platform intelligence agent. What can I help you with?`;
}

const RESPONSE_TYPE_LABEL: Partial<Record<NoraResponseType, string>> = {
  TaskDelegation: 'Task delegation',
  StrategyRecommendation: 'Strategy',
  PerformanceInsight: 'Performance insight',
  DecisionSupport: 'Decision support',
  CoordinationAction: 'Coordinating agents',
  ProactiveAlert: 'Alert',
  DirectResponse: 'Direct response',
};

const THINKING_PHRASES = [
  'Accessing organisational context…',
  'Reviewing active priorities…',
  'Consulting knowledge graph…',
  'Checking agent availability…',
  'Formulating executive response…',
  'Analysing request parameters…',
  'Cross-referencing project data…',
];

const SUB_AGENTS = [
  {
    key: 'astra',
    label: 'Astra',
    role: 'Strategy',
    color: 'from-purple-500/50 to-indigo-500/30',
  },
  {
    key: 'auri',
    label: 'Auri',
    role: 'Dev',
    color: 'from-cyan-500/50 to-blue-500/30',
  },
  {
    key: 'cash',
    label: 'Cash',
    role: 'Sales',
    color: 'from-emerald-500/50 to-green-500/30',
  },
  {
    key: 'scout',
    label: 'Scout',
    role: 'Intel',
    color: 'from-amber-500/50 to-yellow-500/30',
  },
  {
    key: 'genesis',
    label: 'Genesis',
    role: 'Brand',
    color: 'from-pink-500/50 to-rose-500/30',
  },
  {
    key: 'lux',
    label: 'Lux',
    role: 'Design',
    color: 'from-violet-500/50 to-purple-500/30',
  },
  {
    key: 'editron',
    label: 'Editron',
    role: 'Edit',
    color: 'from-orange-500/50 to-red-500/30',
  },
  {
    key: 'maci',
    label: 'Maci',
    role: 'Cinema',
    color: 'from-teal-500/50 to-emerald-500/30',
  },
];

// ── Sub-components ────────────────────────────────────────────────────────────

function ThinkingHUD({ idx }: { idx: number }) {
  return (
    <div className="flex items-center gap-2.5 px-4 py-1.5 rounded-full border border-amber-500/20 bg-amber-500/5">
      <span className="flex gap-1">
        {[0, 1, 2].map((i) => (
          <span
            key={i}
            className="w-1 h-1 rounded-full bg-amber-400/60"
            style={{
              animation: `dot-bounce 1.2s ease-in-out ${i * 0.2}s infinite`,
            }}
          />
        ))}
      </span>
      <span className="text-[11px] text-amber-300/70 tracking-wide">
        {THINKING_PHRASES[idx % THINKING_PHRASES.length]}
      </span>
    </div>
  );
}

// Agent pill used in both sidebar and mobile grid
function AgentPill({
  agent,
  isActive,
}: {
  agent: (typeof SUB_AGENTS)[0];
  isActive: boolean;
}) {
  return (
    <div
      className={cn(
        'flex items-center gap-2 rounded-lg px-2.5 py-2 border transition-all duration-500',
        isActive
          ? 'border-violet-500/40 bg-violet-500/10 opacity-100'
          : 'border-white/[0.06] bg-white/[0.02] opacity-40'
      )}
    >
      <div
        className={cn(
          'w-6 h-6 rounded-full shrink-0 flex items-center justify-center bg-gradient-to-br text-[9px] font-bold relative',
          agent.color,
          isActive ? 'opacity-100' : 'opacity-60'
        )}
      >
        {agent.label[0]}
        {isActive && (
          <span className="absolute inset-0 rounded-full animate-ping bg-violet-400/20" />
        )}
      </div>
      <div className="min-w-0">
        <div className="text-[10px] font-medium text-white/70 leading-none">
          {agent.label}
        </div>
        <div
          className={cn(
            'text-[9px] leading-none mt-0.5',
            isActive ? 'text-violet-400/70' : 'text-white/25'
          )}
        >
          {isActive ? 'Active' : agent.role}
        </div>
      </div>
      {isActive && (
        <span className="ml-auto w-1.5 h-1.5 rounded-full bg-violet-400/80 animate-pulse shrink-0" />
      )}
    </div>
  );
}

function OrchOverlay({
  responseType,
  planId,
  actions,
}: {
  responseType: NoraResponseType;
  planId?: string;
  actions: ExecutiveAction[];
}) {
  const label = RESPONSE_TYPE_LABEL[responseType] ?? 'Orchestration';
  const isAlert = responseType === 'ProactiveAlert';
  const isDirect = responseType === 'DirectResponse';

  return (
    <div
      className={cn(
        'rounded-xl border px-4 py-3',
        isAlert
          ? 'border-amber-500/30 bg-amber-500/5'
          : isDirect
            ? 'border-white/[0.07] bg-white/[0.02]'
            : 'border-violet-500/20 bg-violet-500/5'
      )}
    >
      <div className="flex items-center gap-2 mb-2">
        {isAlert ? (
          <AlertTriangle className="w-3.5 h-3.5 text-amber-400/80 shrink-0" />
        ) : (
          <Zap
            className={cn(
              'w-3.5 h-3.5 shrink-0',
              isDirect ? 'text-white/30' : 'text-violet-400/80'
            )}
          />
        )}
        <span
          className={cn(
            'text-[10px] font-semibold tracking-widest uppercase',
            isAlert
              ? 'text-amber-400/80'
              : isDirect
                ? 'text-white/30'
                : 'text-violet-400/80'
          )}
        >
          {label}
        </span>
        {planId && (
          <Link
            to="/nora"
            className="ml-auto flex items-center gap-1 text-[10px] text-violet-400/60 hover:text-violet-300 transition-colors shrink-0"
          >
            Plan <ExternalLink className="w-2.5 h-2.5" />
          </Link>
        )}
      </div>
      {actions.length > 0 && (
        <div className="space-y-1.5">
          {actions.map((a) => (
            <div key={a.actionId} className="flex items-start gap-2">
              <Bot className="w-3 h-3 text-white/25 mt-0.5 shrink-0" />
              <div className="min-w-0 text-[11px] text-white/55 leading-snug">
                {a.description}
                {a.assignedTo && (
                  <span className="ml-1 text-violet-400/60">
                    → {a.assignedTo}
                  </span>
                )}
                {a.requiresApproval && (
                  <span className="ml-1 text-amber-400/60 uppercase text-[9px]">
                    {' '}
                    approval needed
                  </span>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
      {planId && (
        <div className="mt-2 pt-2 border-t border-white/[0.06] flex items-center gap-1.5">
          <span className="text-[9px] text-white/20">Plan</span>
          <span className="text-[9px] font-mono text-violet-400/50 bg-violet-500/10 px-1.5 py-0.5 rounded">
            {planId.slice(0, 8)}…
          </span>
        </div>
      )}
    </div>
  );
}

// ── Audio helpers ─────────────────────────────────────────────────────────────

function getRms(analyser: AnalyserNode): number {
  const buf = new Uint8Array(analyser.frequencyBinCount);
  analyser.getByteTimeDomainData(buf);
  let sum = 0;
  for (const v of buf) {
    const n = v / 128 - 1;
    sum += n * n;
  }
  return Math.sqrt(sum / buf.length);
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function HomePage() {
  const { user } = useAuth();
  const isAdmin = user?.is_admin ?? false;
  const firstName = user?.full_name?.split(' ')[0] ?? 'there';

  const [messages, setMessages] = useState<Message[]>([]);
  const [lastResponse, setLast] = useState(() =>
    getGreeting(firstName, isAdmin)
  );
  const [input, setInput] = useState('');
  const [orbState, setOrbState] = useState<OrbState>(null);
  const [isListening, setListening] = useState(false);
  const [isSending, setSending] = useState(false);
  const [showHistory, setHistory] = useState(false);
  const [orchOverlay, setOrch] = useState<{
    responseType: NoraResponseType;
    planId?: string;
    actions: ExecutiveAction[];
  } | null>(null);
  const [suggestions, setSugg] = useState<string[]>([]);
  const [thinkingIdx, setThinkingIdx] = useState(0);
  const [delegations, setDelegations] = useState<Record<string, boolean>>({});

  const inputVolRef = useRef<number>(0);
  const outputVolRef = useRef<number>(0);
  const sessionId = useRef(`interface-${Date.now()}`);
  const speechRef = useRef<SpeechRecognition | null>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const historyRef = useRef<HTMLDivElement>(null);
  const audioCtxRef = useRef<AudioContext | null>(null);
  const analyserRef = useRef<AnalyserNode | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const rafMicRef = useRef<number>(0);
  const outAnalyserRef = useRef<AnalyserNode | null>(null);
  const outSourceRef = useRef<MediaElementAudioSourceNode | null>(null);
  const rafOutRef = useRef<number>(0);
  const agentTimersRef = useRef<Record<string, ReturnType<typeof setTimeout>>>(
    {}
  );
  const finalTranscriptRef = useRef('');
  const startListeningRef = useRef<() => void>();

  useEffect(() => {
    if (showHistory && historyRef.current)
      historyRef.current.scrollTop = historyRef.current.scrollHeight;
  }, [messages, showHistory]);

  // Thinking phrase cycling
  useEffect(() => {
    if (orbState !== 'thinking') {
      setThinkingIdx(0);
      return;
    }
    const id = setInterval(() => setThinkingIdx((i) => i + 1), 1800);
    return () => clearInterval(id);
  }, [orbState]);

  // Agent delegation from actions
  useEffect(() => {
    if (!orchOverlay || orchOverlay.actions.length === 0) return;
    const newDels: Record<string, boolean> = {};
    for (const action of orchOverlay.actions) {
      if (action.assignedTo) {
        const matched = SUB_AGENTS.find((a) =>
          action.assignedTo!.toLowerCase().includes(a.key)
        );
        if (matched) {
          newDels[matched.key] = true;
          clearTimeout(agentTimersRef.current[matched.key]);
          agentTimersRef.current[matched.key] = setTimeout(() => {
            setDelegations((prev) => {
              const n = { ...prev };
              delete n[matched.key];
              return n;
            });
          }, 10_000);
        }
      }
    }
    if (Object.keys(newDels).length > 0)
      setDelegations((prev) => ({ ...prev, ...newDels }));
  }, [orchOverlay]);

  // Mic tracking
  const startMicTracking = useCallback(async () => {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;
      const ctx = new AudioContext();
      audioCtxRef.current = ctx;
      const src = ctx.createMediaStreamSource(stream);
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 256;
      src.connect(analyser);
      analyserRef.current = analyser;
      const tick = () => {
        inputVolRef.current = getRms(analyser) * 4;
        rafMicRef.current = requestAnimationFrame(tick);
      };
      tick();
    } catch {
      /* mic denied */
    }
  }, []);

  const stopMicTracking = useCallback(() => {
    cancelAnimationFrame(rafMicRef.current);
    streamRef.current?.getTracks().forEach((t) => t.stop());
    audioCtxRef.current?.close();
    audioCtxRef.current = null;
    analyserRef.current = null;
    inputVolRef.current = 0;
  }, []);

  const startOutputTracking = useCallback((el: HTMLAudioElement) => {
    cancelAnimationFrame(rafOutRef.current);
    try {
      const ctx = audioCtxRef.current ?? new AudioContext();
      if (!audioCtxRef.current) audioCtxRef.current = ctx;
      if (!outSourceRef.current || outSourceRef.current.mediaElement !== el) {
        outSourceRef.current?.disconnect();
        outSourceRef.current = ctx.createMediaElementSource(el);
      }
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 256;
      outSourceRef.current.connect(analyser);
      analyser.connect(ctx.destination);
      outAnalyserRef.current = analyser;
      const tick = () => {
        outputVolRef.current = getRms(analyser) * 4;
        rafOutRef.current = requestAnimationFrame(tick);
      };
      tick();
    } catch {
      /* no output tracking */
    }
  }, []);

  const stopOutputTracking = useCallback(() => {
    cancelAnimationFrame(rafOutRef.current);
    outputVolRef.current = 0;
  }, []);

  useEffect(
    () => () => {
      stopMicTracking();
      stopOutputTracking();
      Object.values(agentTimersRef.current).forEach(clearTimeout);
    },
    [stopMicTracking, stopOutputTracking]
  );

  // Send message
  const sendMessage = useCallback(
    async (content: string) => {
      if (!content.trim() || isSending) return;
      setMessages((prev) => [
        ...prev,
        { id: `u-${Date.now()}`, role: 'user', content },
      ]);
      setInput('');
      setSending(true);
      setOrbState('thinking');
      setOrch(null);
      setSugg([]);

      try {
        if (isAdmin) {
          const res = await fetch(resolveApiUrl('/api/nora/chat'), {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              message: content,
              sessionId: sessionId.current,
              requestType: 'textInteraction',
              voiceEnabled: true,
              priority: 'normal',
              context: null,
            }),
          });
          if (!res.ok) throw new Error('Nora error');
          const data = (await res.json()) as NoraResponse;

          setMessages((prev) => [
            ...prev,
            { id: `a-${Date.now()}`, role: 'assistant', content: data.content },
          ]);
          setLast(data.content);
          setOrch({
            responseType: data.responseType,
            planId: data.planId,
            actions: data.actions ?? [],
          });
          if (data.followUpSuggestions?.length > 0)
            setSugg(data.followUpSuggestions.slice(0, 3));

          if (data.voiceResponse && audioRef.current) {
            setOrbState('talking');
            audioRef.current.src = `data:audio/mp3;base64,${data.voiceResponse}`;
            startOutputTracking(audioRef.current);
            audioRef.current.onended = () => {
              stopOutputTracking();
              setOrbState(null);
              // Auto-restart listening for conversational back-and-forth
              void startListeningRef.current?.();
            };
            audioRef.current.play().catch(() => {
              // Autoplay blocked — skip TTS, just show text response
              stopOutputTracking();
              setOrbState(null);
            });
          } else {
            setOrbState(null);
          }
        } else {
          const data = await topsiApi.chat({
            message: content,
            sessionId: sessionId.current,
            projectId: null,
          });
          const reply = data.message ?? 'I received your message.';
          setMessages((prev) => [
            ...prev,
            { id: `a-${Date.now()}`, role: 'assistant', content: reply },
          ]);
          setLast(reply);
          setOrbState(null);
        }
      } catch {
        setLast('I encountered an issue. Please try again.');
        setOrbState(null);
      } finally {
        setSending(false);
      }
    },
    [isAdmin, isSending, startOutputTracking, stopOutputTracking]
  );

  const handleSubmit = useCallback(() => {
    if (input.trim()) void sendMessage(input);
  }, [input, sendMessage]);
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
      }
    },
    [handleSubmit]
  );

  const startListening = useCallback(async () => {
    if (isListening) return;
    const Ctor = window.SpeechRecognition ?? window.webkitSpeechRecognition;
    if (!Ctor) return;
    await startMicTracking();
    finalTranscriptRef.current = '';
    const r = new Ctor();
    r.lang = 'en-GB';
    r.continuous = false;
    r.interimResults = true;
    r.onresult = (e: SREvent) => {
      // Accumulate all results (final + interim) — mobile browsers often never
      // set isFinal=true before onend fires, so we track the best available text
      let full = '';
      for (let i = 0; i < e.results.length; i++) {
        full += e.results[i][0].transcript;
      }
      finalTranscriptRef.current = full;
      setInput(full);
    };
    r.onend = () => {
      setListening(false);
      stopMicTracking();
      const transcript = finalTranscriptRef.current.trim();
      finalTranscriptRef.current = '';
      if (transcript) {
        setInput('');
        void sendMessage(transcript);
      } else {
        setOrbState(null);
      }
    };
    r.onerror = () => {
      setListening(false);
      setOrbState(null);
      stopMicTracking();
      finalTranscriptRef.current = '';
    };
    speechRef.current = r;
    r.start();
    setListening(true);
    setOrbState('listening');
  }, [isListening, sendMessage, startMicTracking, stopMicTracking]);

  // Keep a ref so TTS onended can restart listening without stale closure
  useEffect(() => {
    startListeningRef.current = startListening;
  }, [startListening]);

  const toggleVoice = useCallback(() => {
    if (isListening) {
      speechRef.current?.stop();
      setListening(false);
      setOrbState(null);
      stopMicTracking();
      finalTranscriptRef.current = '';
      return;
    }
    // Unlock audio on iOS — must happen inside a user gesture handler
    if (audioRef.current) {
      audioRef.current
        .play()
        .then(() => audioRef.current?.pause())
        .catch(() => {});
    }
    void startListening();
  }, [isListening, startListening, stopMicTracking]);

  const agentName = isAdmin ? 'NORA' : 'TOPSI';
  const agentSub = isAdmin ? 'Executive Orchestrator' : 'Platform Intelligence';

  const stateLabel =
    orbState === null
      ? 'Ready'
      : orbState === 'listening'
        ? '● Listening'
        : orbState === 'thinking'
          ? '⟳ Thinking'
          : '▶ Speaking';
  const stateLabelColor =
    orbState === null
      ? 'text-white/20'
      : orbState === 'listening'
        ? 'text-blue-400/80'
        : orbState === 'thinking'
          ? 'text-amber-400/80'
          : 'text-emerald-400/80';

  return (
    <div className="fixed inset-0 z-10 flex flex-col bg-[#06060a] text-white overflow-hidden">
      <audio ref={audioRef} className="hidden" />
      <style>{`
        @keyframes dot-bounce {
          0%,80%,100% { transform:translateY(0); opacity:.4; }
          40% { transform:translateY(-3px); opacity:1; }
        }
      `}</style>

      {/* ── Top identity bar ─────────────────────────────────────── */}
      <div className="shrink-0 flex items-center justify-between px-5 py-3 border-b border-white/[0.06]">
        <div className="flex items-center gap-3">
          <span
            className={cn(
              'text-[11px] font-bold tracking-[0.2em] uppercase px-2.5 py-1 rounded-full border',
              isAdmin
                ? 'border-violet-500/40 text-violet-300 bg-violet-500/10'
                : 'border-blue-500/40 text-blue-300 bg-blue-500/10'
            )}
          >
            {agentName}
          </span>
          <span className="text-[10px] text-white/25 tracking-widest hidden sm:block">
            {agentSub}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <Radio className="w-3 h-3 text-emerald-400/50" />
          <span
            className={cn(
              'text-[10px] tracking-widest uppercase transition-colors',
              stateLabelColor
            )}
          >
            {stateLabel}
          </span>
        </div>
      </div>

      {/* ── Main 2-panel layout ──────────────────────────────────── */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left panel — Agent Network (admin, lg+ screens) */}
        {isAdmin && (
          <div className="hidden lg:flex w-56 shrink-0 flex-col border-r border-white/[0.05] overflow-y-auto">
            <div className="p-4 border-b border-white/[0.05]">
              <p className="text-[9px] tracking-[0.2em] uppercase text-white/25 mb-3">
                Agent Network
              </p>
              <div className="grid grid-cols-1 gap-1.5">
                {SUB_AGENTS.map((agent) => (
                  <AgentPill
                    key={agent.key}
                    agent={agent}
                    isActive={!!delegations[agent.key]}
                  />
                ))}
              </div>
            </div>
            {orchOverlay && (
              <div className="p-4">
                <p className="text-[9px] tracking-[0.2em] uppercase text-white/25 mb-2">
                  Last Response
                </p>
                <OrchOverlay {...orchOverlay} />
              </div>
            )}
          </div>
        )}

        {/* Center — Orb + chat */}
        <div className="flex-1 flex flex-col overflow-y-auto">
          <div className="flex-1 flex flex-col items-center px-4 py-4 gap-4">
            {/* Orb — fills the responsive clamp container */}
            <div
              className="relative shrink-0"
              style={{
                width: 'clamp(180px, 35vh, 280px)',
                height: 'clamp(180px, 35vh, 280px)',
              }}
            >
              <OrbVisualizer
                state={orbState}
                isAdmin={isAdmin}
                inputVolumeRef={inputVolRef}
                outputVolumeRef={outputVolRef}
              />
            </div>

            {/* Thinking HUD */}
            {orbState === 'thinking' && <ThinkingHUD idx={thinkingIdx} />}

            {/* Response text */}
            <div className="w-full max-w-lg text-center px-2">
              <p
                className={cn(
                  'text-sm sm:text-base leading-relaxed transition-all duration-500',
                  isSending ? 'text-white/20' : 'text-white/80'
                )}
              >
                {lastResponse}
              </p>
            </div>

            {/* Mobile: Orch overlay + agent pills (hidden on lg where sidebar shows them) */}
            {isAdmin && orchOverlay && (
              <div className="lg:hidden w-full max-w-lg">
                <OrchOverlay {...orchOverlay} />
              </div>
            )}

            {/* Mobile: agent grid 2×4 (hidden on lg) */}
            {isAdmin && (
              <div className="lg:hidden w-full max-w-sm">
                <p className="text-[9px] tracking-[0.2em] uppercase text-white/20 text-center mb-2">
                  Agent Network
                </p>
                <div className="grid grid-cols-2 gap-1.5">
                  {SUB_AGENTS.map((agent) => (
                    <AgentPill
                      key={agent.key}
                      agent={agent}
                      isActive={!!delegations[agent.key]}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* Suggestion chips */}
            {suggestions.length > 0 && (
              <div className="flex flex-wrap gap-2 justify-center max-w-lg">
                {suggestions.map((s, i) => (
                  <button
                    key={i}
                    onClick={() => void sendMessage(s)}
                    className={cn(
                      'text-[11px] px-3 py-1.5 rounded-full border transition-all',
                      isAdmin
                        ? 'border-violet-500/25 text-violet-300/70 hover:border-violet-400/50 hover:bg-violet-500/10'
                        : 'border-blue-500/25 text-blue-300/70 hover:border-blue-400/50 hover:bg-blue-500/10'
                    )}
                  >
                    {s}
                  </button>
                ))}
              </div>
            )}

            {/* History */}
            {messages.length > 0 && (
              <button
                onClick={() => setHistory((v) => !v)}
                className="flex items-center gap-1.5 text-[11px] text-white/25 hover:text-white/50 transition-colors"
              >
                <ChevronDown
                  className={cn(
                    'w-3 h-3 transition-transform',
                    showHistory && 'rotate-180'
                  )}
                />
                {showHistory
                  ? 'Hide history'
                  : `${messages.length} message${messages.length !== 1 ? 's' : ''}`}
              </button>
            )}
            {showHistory && (
              <div ref={historyRef} className="w-full max-w-lg space-y-2 pb-2">
                {messages.map((m) => (
                  <div
                    key={m.id}
                    className={cn(
                      'text-xs rounded-lg px-3 py-2 leading-relaxed',
                      m.role === 'user'
                        ? 'bg-white/5 text-white/60 ml-8'
                        : 'bg-white/[0.03] text-white/50 mr-8'
                    )}
                  >
                    <span className="font-medium text-white/30 mr-1.5">
                      {m.role === 'user' ? 'You' : agentName}
                    </span>
                    {m.content}
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* ── Input bar ───────────────────────────────────────────── */}
          <div className="shrink-0 px-4 pb-4 pt-2 border-t border-white/[0.06]">
            <div
              className={cn(
                'flex items-center gap-3 max-w-xl mx-auto rounded-2xl px-4 py-3 border transition-all',
                isListening
                  ? 'border-blue-500/50 bg-blue-500/5'
                  : 'border-white/10 bg-white/[0.04] hover:border-white/20'
              )}
            >
              <button
                onClick={() => void toggleVoice()}
                disabled={isSending}
                className={cn(
                  'shrink-0 w-8 h-8 rounded-full flex items-center justify-center transition-all',
                  isListening
                    ? 'bg-blue-500/20 text-blue-400 animate-pulse'
                    : 'text-white/30 hover:text-white/60 hover:bg-white/5'
                )}
              >
                {isListening ? (
                  <Mic className="w-4 h-4" />
                ) : (
                  <MicOff className="w-4 h-4" />
                )}
              </button>
              <input
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder={
                  isListening
                    ? 'Listening…'
                    : isSending
                      ? 'Processing…'
                      : `Message ${isAdmin ? 'Nora' : 'Topsi'}…`
                }
                disabled={isSending}
                className="flex-1 bg-transparent text-sm text-white placeholder:text-white/25 outline-none min-w-0"
              />
              <button
                onClick={handleSubmit}
                disabled={!input.trim() || isSending}
                className={cn(
                  'shrink-0 w-8 h-8 rounded-full flex items-center justify-center transition-all',
                  input.trim() && !isSending
                    ? isAdmin
                      ? 'bg-violet-600/40 text-violet-300 hover:bg-violet-600/60'
                      : 'bg-blue-600/40 text-blue-300 hover:bg-blue-600/60'
                    : 'text-white/20 cursor-not-allowed'
                )}
              >
                <Send className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
