import { useEffect, useRef, useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Mic,
  MicOff,
  Clock,
  Users,
  Radio,
  MessageSquare,
  ChevronRight,
  RefreshCw,
  Bot,
  Headphones,
  Calendar,
  Hash,
} from 'lucide-react';
import { discordApi, type DiscordSessionSummary, type DiscordSegment } from '@/lib/api';
import { discordKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';
import { formatDateTime } from '@/lib/formatters';
import { EmptyState } from '@/components/ui/empty-state';

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatElapsed(secs: number) {
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

function agentColor(agent: string) {
  if (agent === 'topsi') return 'bg-purple-500/10 text-purple-400 border-purple-500/20';
  return 'bg-blue-500/10 text-blue-400 border-blue-500/20';
}

// ── Live transcript SSE hook ──────────────────────────────────────────────────

function useLiveTranscript(sessionId: string | null) {
  const [segments, setSegments] = useState<DiscordSegment[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    if (!sessionId) {
      setSegments([]);
      setConnected(false);
      return;
    }

    const url = `/api/discord/sessions/${sessionId}/stream`;
    const es = new EventSource(url);

    es.addEventListener('transcript', (e) => {
      try {
        const seg: DiscordSegment = JSON.parse(e.data);
        setSegments((prev) => [...prev, seg]);
      } catch { /* ignore */ }
    });

    es.addEventListener('done', () => {
      setConnected(false);
      es.close();
    });

    es.onopen = () => setConnected(true);
    es.onerror = () => setConnected(false);

    return () => {
      es.close();
      setConnected(false);
    };
  }, [sessionId]);

  return { segments, connected };
}

// ── Active session card ───────────────────────────────────────────────────────

function ActiveSessionCard({
  session,
  isSelected,
  onClick,
}: {
  session: DiscordSessionSummary;
  isSelected: boolean;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full text-left rounded-xl border p-4 transition-colors',
        isSelected
          ? 'border-primary/50 bg-primary/5'
          : 'border-border hover:border-primary/30 hover:bg-muted/40'
      )}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <div className="relative flex-shrink-0">
            <Mic className="h-4 w-4 text-green-400" />
            <span className="absolute -top-0.5 -right-0.5 h-2 w-2 rounded-full bg-green-400 animate-pulse" />
          </div>
          <div className="min-w-0">
            <p className="font-medium text-sm truncate">
              #{session.channel_name || session.channel_id}
            </p>
            <p className="text-xs text-muted-foreground truncate">
              Guild {session.guild_id.slice(-6)}
            </p>
          </div>
        </div>
        <Badge
          variant="outline"
          className={cn('text-xs flex-shrink-0', agentColor(session.agent))}
        >
          {session.agent}
        </Badge>
      </div>
      <div className="mt-3 flex items-center gap-4 text-xs text-muted-foreground">
        <span className="flex items-center gap-1">
          <Clock className="h-3 w-3" />
          {formatElapsed(session.elapsed_seconds)}
        </span>
        <span className="flex items-center gap-1">
          <Users className="h-3 w-3" />
          {session.participant_count}
        </span>
        <span className="flex items-center gap-1">
          <MessageSquare className="h-3 w-3" />
          {session.segment_count}
        </span>
      </div>
    </button>
  );
}

// ── Transcript viewer (live SSE) ──────────────────────────────────────────────

function LiveTranscriptPane({ sessionId }: { sessionId: string }) {
  const { segments, connected } = useLiveTranscript(sessionId);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [segments]);

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/30">
        <div
          className={cn(
            'h-2 w-2 rounded-full',
            connected ? 'bg-green-400 animate-pulse' : 'bg-muted-foreground'
          )}
        />
        <span className="text-xs text-muted-foreground">
          {connected ? 'Live' : 'Connecting...'}
        </span>
        <span className="ml-auto text-xs text-muted-foreground">
          {segments.length} utterances
        </span>
      </div>

      <ScrollArea className="flex-1 px-4 py-3">
        {segments.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
            <Headphones className="h-8 w-8 mb-3 opacity-40" />
            <p className="text-sm">Listening... speak to see the transcript</p>
          </div>
        ) : (
          <div className="space-y-3">
            {segments.map((seg, i) => (
              <div
                key={seg.id || i}
                className={cn(
                  'rounded-lg px-3 py-2 text-sm',
                  seg.is_topsi_addressed
                    ? 'bg-purple-500/10 border border-purple-500/20'
                    : 'bg-muted/40'
                )}
              >
                {seg.speaker_label && (
                  <p className="text-xs font-medium text-muted-foreground mb-1">
                    {seg.speaker_label}
                  </p>
                )}
                <p className="leading-relaxed">{seg.text}</p>
                {seg.is_topsi_addressed && (
                  <Badge
                    variant="outline"
                    className="mt-1.5 text-xs bg-purple-500/10 text-purple-400 border-purple-500/20"
                  >
                    agent addressed
                  </Badge>
                )}
              </div>
            ))}
          </div>
        )}
        <div ref={bottomRef} />
      </ScrollArea>
    </div>
  );
}

// ── Archived transcript panel ─────────────────────────────────────────────────

function ArchivedTranscriptPane({ sessionId }: { sessionId: string }) {
  const { data, isLoading } = useQuery({
    queryKey: ['discord-transcript', sessionId],
    queryFn: () => discordApi.getTranscript(sessionId),
    enabled: !!sessionId,
  });

  if (isLoading) {
    return (
      <div className="space-y-2 p-4">
        {[...Array(5)].map((_, i) => (
          <Skeleton key={i} className="h-10 w-full rounded-lg" />
        ))}
      </div>
    );
  }

  const segments = data?.segments ?? [];

  return (
    <ScrollArea className="h-full px-4 py-3">
      {segments.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
          <MessageSquare className="h-8 w-8 mb-3 opacity-40" />
          <p className="text-sm">No transcript segments recorded</p>
        </div>
      ) : (
        <div className="space-y-3">
          {segments.map((seg, i) => (
            <div
              key={seg.id || i}
              className={cn(
                'rounded-lg px-3 py-2 text-sm',
                seg.is_topsi_addressed
                  ? 'bg-purple-500/10 border border-purple-500/20'
                  : 'bg-muted/40'
              )}
            >
              {seg.speaker_label && (
                <p className="text-xs font-medium text-muted-foreground mb-1">
                  {seg.speaker_label}
                </p>
              )}
              <p className="leading-relaxed">{seg.text}</p>
              <p className="text-xs text-muted-foreground mt-1">
                {(seg.start_time_ms / 1000).toFixed(1)}s
              </p>
            </div>
          ))}
        </div>
      )}
    </ScrollArea>
  );
}

// ── Archived session row ──────────────────────────────────────────────────────

function ArchivedSessionRow({
  session,
  isSelected,
  onClick,
}: {
  session: any;
  isSelected: boolean;
  onClick: () => void;
}) {
  const agent = session.agent_name || session.agent || 'nora';
  return (
    <button
      onClick={onClick}
      className={cn(
        'w-full text-left flex items-center justify-between gap-3 px-4 py-3 border-b last:border-0 transition-colors hover:bg-muted/40',
        isSelected && 'bg-primary/5'
      )}
    >
      <div className="flex items-center gap-3 min-w-0">
        <MicOff className="h-4 w-4 text-muted-foreground flex-shrink-0" />
        <div className="min-w-0">
          <p className="text-sm font-medium truncate">
            {session.title || session.id?.slice(0, 8)}
          </p>
          <p className="text-xs text-muted-foreground">
            {formatDateTime(session.started_at || session.created_at)}
            {session.ended_at && (
              <> · {formatElapsed(Math.round((new Date(session.ended_at).getTime() - new Date(session.started_at).getTime()) / 1000))}</>
            )}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-2 flex-shrink-0">
        <Badge variant="outline" className={cn('text-xs', agentColor(agent))}>
          {agent}
        </Badge>
        <ChevronRight className="h-3 w-3 text-muted-foreground" />
      </div>
    </button>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function DiscordPage() {
  const qc = useQueryClient();
  const [selectedActiveId, setSelectedActiveId] = useState<string | null>(null);
  const [selectedArchivedId, setSelectedArchivedId] = useState<string | null>(null);

  const { data: activeSessions = [], isLoading: loadingActive } = useQuery({
    queryKey: discordKeys.active(),
    queryFn: discordApi.activeSessions,
    refetchInterval: 5000,
  });

  const { data: archivedSessions = [], isLoading: loadingArchived } = useQuery({
    queryKey: discordKeys.archive(),
    queryFn: () => discordApi.archivedSessions({ limit: 50 }),
    refetchInterval: 30000,
  });

  // Auto-select first active session
  useEffect(() => {
    if (activeSessions.length > 0 && !selectedActiveId) {
      setSelectedActiveId(activeSessions[0].meeting_session_id);
    }
    if (activeSessions.length === 0) {
      setSelectedActiveId(null);
    }
  }, [activeSessions, selectedActiveId]);

  const selectedActive = activeSessions.find(
    (s) => s.meeting_session_id === selectedActiveId
  );

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b">
        <div className="flex items-center gap-3">
          <Radio className="h-5 w-5 text-primary" />
          <h1 className="text-lg font-bold">Discord Voice</h1>
          {activeSessions.length > 0 && (
            <Badge className="bg-green-500/15 text-green-400 border-green-500/20 text-xs">
              {activeSessions.length} live
            </Badge>
          )}
        </div>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => {
            qc.invalidateQueries({ queryKey: discordKeys.active() });
            qc.invalidateQueries({ queryKey: discordKeys.archive() });
          }}
        >
          <RefreshCw className="h-4 w-4" />
        </Button>
      </div>

      <Tabs defaultValue="live" className="flex-1 flex flex-col overflow-hidden">
        <TabsList className="mx-6 mt-4 w-fit">
          <TabsTrigger value="live" className="flex items-center gap-1.5">
            <Mic className="h-3.5 w-3.5" />
            Live Sessions
            {activeSessions.length > 0 && (
              <span className="ml-1 h-1.5 w-1.5 rounded-full bg-green-400 animate-pulse" />
            )}
          </TabsTrigger>
          <TabsTrigger value="archive" className="flex items-center gap-1.5">
            <Calendar className="h-3.5 w-3.5" />
            Archive
          </TabsTrigger>
          <TabsTrigger value="setup" className="flex items-center gap-1.5">
            <Bot className="h-3.5 w-3.5" />
            Bot Setup
          </TabsTrigger>
        </TabsList>

        {/* ── Live tab ── */}
        <TabsContent value="live" className="flex-1 overflow-hidden mt-4 px-6 pb-6">
          {loadingActive ? (
            <div className="space-y-2">
              {[...Array(3)].map((_, i) => (
                <Skeleton key={i} className="h-24 w-full rounded-xl" />
              ))}
            </div>
          ) : activeSessions.length === 0 ? (
            <EmptyState
              icon={Headphones}
              title="No active voice sessions"
              description="Use /nora-join or /topsi-join in Discord to start a session"
              className="h-48"
            />
          ) : (
            <div className="grid grid-cols-1 lg:grid-cols-[280px_1fr] gap-4 h-full">
              {/* Session list */}
              <div className="space-y-2 overflow-y-auto">
                {activeSessions.map((s) => (
                  <ActiveSessionCard
                    key={s.meeting_session_id}
                    session={s}
                    isSelected={s.meeting_session_id === selectedActiveId}
                    onClick={() => setSelectedActiveId(s.meeting_session_id)}
                  />
                ))}
              </div>

              {/* Live transcript */}
              {selectedActive ? (
                <Card className="flex flex-col overflow-hidden">
                  <CardHeader className="pb-3 border-b flex-row items-center justify-between">
                    <div>
                      <CardTitle className="text-sm font-medium flex items-center gap-2">
                        <Hash className="h-3.5 w-3.5" />
                        {selectedActive.channel_name || selectedActive.channel_id}
                      </CardTitle>
                      <p className="text-xs text-muted-foreground mt-0.5">
                        Started {formatDateTime(selectedActive.started_at)}
                      </p>
                    </div>
                    <Badge variant="outline" className={cn('text-xs', agentColor(selectedActive.agent))}>
                      {selectedActive.agent}
                    </Badge>
                  </CardHeader>
                  <CardContent className="flex-1 p-0 overflow-hidden">
                    <LiveTranscriptPane sessionId={selectedActive.meeting_session_id} />
                  </CardContent>
                </Card>
              ) : (
                <div className="flex items-center justify-center text-muted-foreground text-sm">
                  Select a session to view live transcript
                </div>
              )}
            </div>
          )}
        </TabsContent>

        {/* ── Archive tab ── */}
        <TabsContent value="archive" className="flex-1 overflow-hidden mt-4 px-6 pb-6">
          {loadingArchived ? (
            <div className="space-y-1">
              {[...Array(8)].map((_, i) => (
                <Skeleton key={i} className="h-14 w-full rounded-lg" />
              ))}
            </div>
          ) : archivedSessions.length === 0 ? (
            <EmptyState
              icon={Calendar}
              title="No archived sessions yet"
              className="h-48"
            />
          ) : (
            <div className="grid grid-cols-1 lg:grid-cols-[320px_1fr] gap-4 h-full">
              {/* Session list */}
              <Card className="overflow-hidden flex flex-col">
                <CardHeader className="pb-2 border-b py-3">
                  <CardTitle className="text-sm font-medium text-muted-foreground">
                    {archivedSessions.length} sessions
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-0 flex-1 overflow-hidden">
                  <ScrollArea className="h-full">
                    {archivedSessions.map((s) => (
                      <ArchivedSessionRow
                        key={s.id}
                        session={s}
                        isSelected={s.id === selectedArchivedId}
                        onClick={() => setSelectedArchivedId(s.id)}
                      />
                    ))}
                  </ScrollArea>
                </CardContent>
              </Card>

              {/* Transcript */}
              {selectedArchivedId ? (
                <Card className="flex flex-col overflow-hidden">
                  <CardHeader className="pb-3 border-b">
                    <CardTitle className="text-sm font-medium">Transcript</CardTitle>
                  </CardHeader>
                  <CardContent className="flex-1 p-0 overflow-hidden">
                    <ArchivedTranscriptPane sessionId={selectedArchivedId} />
                  </CardContent>
                </Card>
              ) : (
                <div className="flex items-center justify-center text-muted-foreground text-sm rounded-xl border border-dashed">
                  Select a session to view transcript
                </div>
              )}
            </div>
          )}
        </TabsContent>

        {/* ── Setup tab ── */}
        <TabsContent value="setup" className="flex-1 overflow-auto mt-4 px-6 pb-6">
          <div className="max-w-2xl space-y-4">
            <Card>
              <CardHeader>
                <CardTitle className="text-sm flex items-center gap-2">
                  <Bot className="h-4 w-4" />
                  How to use Discord Voice
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-4 text-sm">
                <div className="space-y-2">
                  <p className="font-medium">1. Join a voice channel</p>
                  <p className="text-muted-foreground">
                    Use these slash commands in any Discord channel where the bot is present:
                  </p>
                  <div className="rounded-lg bg-muted px-4 py-3 space-y-1 font-mono text-xs">
                    <p><span className="text-blue-400">/nora-join</span> — Nora listens and responds</p>
                    <p><span className="text-purple-400">/topsi-join</span> — Topsi listens and responds</p>
                    <p><span className="text-muted-foreground">/bot-leave</span> — End the session</p>
                    <p><span className="text-muted-foreground">/meeting-summary</span> — Generate AI summary</p>
                  </div>
                </div>

                <div className="space-y-2">
                  <p className="font-medium">2. Wake word detection</p>
                  <p className="text-muted-foreground">
                    The agents listen passively and transcribe all speakers. To get a response, address the agent by name:
                  </p>
                  <div className="rounded-lg bg-muted px-4 py-3 space-y-1 text-xs">
                    <p>"Hey <span className="text-blue-400">Nora</span>, what's the status of Project Alpha?"</p>
                    <p>"<span className="text-purple-400">Topsi</span>, summarise the discussion so far"</p>
                  </div>
                </div>

                <div className="space-y-2">
                  <p className="font-medium">3. Required environment variable</p>
                  <div className="rounded-lg bg-muted px-4 py-3 font-mono text-xs">
                    <p>DISCORD_BOT_TOKEN=your_bot_token_here</p>
                  </div>
                  <p className="text-muted-foreground text-xs">
                    Create a bot at discord.com/developers → add to your server with the{' '}
                    <code className="bg-muted px-1 rounded">bot</code> scope and{' '}
                    <code className="bg-muted px-1 rounded">applications.commands</code> scope.
                    Enable the <strong>Server Members Intent</strong> and{' '}
                    <strong>Voice State</strong> intent.
                  </p>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-sm flex items-center gap-2">
                  <Headphones className="h-4 w-4" />
                  Audio pipeline
                </CardTitle>
              </CardHeader>
              <CardContent className="text-sm text-muted-foreground space-y-2">
                <p>
                  The bot receives per-user Opus audio streams via Discord's RTP protocol,
                  decodes to PCM, applies 1.5s silence gating, then pipes each speaker's audio
                  through speech-to-text (Whisper).
                </p>
                <p>
                  Wake word detection checks each utterance for the agent's name. When addressed,
                  the response is synthesized via ElevenLabs and played back into the voice channel.
                </p>
                <p>
                  All segments are archived to the database and streamed live to this dashboard via
                  SSE.
                </p>
              </CardContent>
            </Card>
          </div>
        </TabsContent>
      </Tabs>
    </div>
  );
}

export default DiscordPage;
