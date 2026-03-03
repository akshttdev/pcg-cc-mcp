import { useState, useEffect, useRef, useCallback } from 'react';
import { useQuery } from '@tanstack/react-query';
import { resolveApiUrl, projectsApi } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Mic,
  Square,
  Pause,
  Play,
  Users,
  Clock,
  Loader2,
  Network,
  FileText,
  Monitor,
  MonitorOff,
  UserPlus,
  X,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

// ── Types ─────────────────────────────────────────────────────────────────────

interface TranscriptEntry {
  text: string;
  speakerLabel?: string;
  isTopsiAddressed: boolean;
  topsiResponse?: string;
  timestamp: number;
  segmentIndex: number;
}

interface MeetingNotes {
  summary: string;
  topics: string[];
  decisions: string[];
  actionItems: { description: string; assignee?: string; deadline?: string; priority?: string }[];
  openQuestions: string[];
  participants: string[];
}

interface MeetingModeProps {
  projectId?: string;      // pre-select a project (e.g. from project detail page)
  onClose?: () => void;
  className?: string;
}

type MeetingState = 'idle' | 'active' | 'paused' | 'ended';

// ── Helpers ───────────────────────────────────────────────────────────────────

function formatDuration(seconds: number) {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return `${m}:${String(s).padStart(2, '0')}`;
}

function getAuthHeaders(): Record<string, string> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const token = localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
  if (token) headers['Authorization'] = `Bearer ${token}`;
  return headers;
}

// ── Component ─────────────────────────────────────────────────────────────────

export function MeetingMode({ projectId: propProjectId, onClose, className }: MeetingModeProps) {
  // Meeting state
  const [meetingState, setMeetingState] = useState<MeetingState>('idle');
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  const [duration, setDuration] = useState(0);
  const [participantCount, setParticipantCount] = useState(0);
  const [_chunkIndex, setChunkIndex] = useState(0);
  const [isProcessing, setIsProcessing] = useState(false);
  const [meetingTitle, setMeetingTitle] = useState('');
  const [notes, setNotes] = useState<MeetingNotes | null>(null);
  const [topsiOverlay, setTopsiOverlay] = useState<string | null>(null);

  // Project selection (used when no propProjectId is provided)
  const [selectedProjectId, setSelectedProjectId] = useState<string>(propProjectId || '');

  // Participants
  const [participants, setParticipants] = useState<string[]>([]);
  const [participantInput, setParticipantInput] = useState('');

  // Screen share
  const [isScreenSharing, setIsScreenSharing] = useState(false);
  const screenStreamRef = useRef<MediaStream | null>(null);
  const screenPreviewRef = useRef<HTMLVideoElement | null>(null);

  // Audio
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const micStreamRef = useRef<MediaStream | null>(null);
  const mixedStreamRef = useRef<MediaStream | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const transcriptEndRef = useRef<HTMLDivElement>(null);
  const currentAudioRef = useRef<HTMLAudioElement | null>(null);
  // Always holds the latest processAudioChunk to avoid stale closure in MediaRecorder callbacks
  const processAudioChunkRef = useRef<(blob: Blob, idx: number) => void>(() => {});

  // Fetch all projects for the selector
  const { data: projects = [] } = useQuery({
    queryKey: ['projects'],
    queryFn: () => projectsApi.getAll(),
    staleTime: 60_000,
  });

  // Use prop project ID if provided
  const activeProjectId = propProjectId || selectedProjectId;

  // Auto-scroll transcript
  useEffect(() => {
    transcriptEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [transcript]);

  // Duration timer
  useEffect(() => {
    if (meetingState === 'active') {
      timerRef.current = setInterval(() => setDuration((d) => d + 1), 1000);
    } else {
      if (timerRef.current) { clearInterval(timerRef.current); timerRef.current = null; }
    }
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [meetingState]);

  // ── Screen share ────────────────────────────────────────────────────────────

  const startScreenShare = async () => {
    try {
      const screenStream = await navigator.mediaDevices.getDisplayMedia({
        video: { frameRate: 5, width: { ideal: 1280 } },
        audio: true, // capture system/tab audio if available
      });
      screenStreamRef.current = screenStream;

      // Show preview thumbnail
      if (screenPreviewRef.current) {
        screenPreviewRef.current.srcObject = screenStream;
        screenPreviewRef.current.play().catch(() => {});
      }

      // Rebuild the mixed audio stream with screen audio added
      if (meetingState === 'active' && micStreamRef.current) {
        rebuildMixedStream(micStreamRef.current, screenStream);
      }

      // Stop screen share automatically when the user presses "Stop Sharing" in browser UI
      screenStream.getVideoTracks()[0]?.addEventListener('ended', () => {
        stopScreenShare();
      });

      setIsScreenSharing(true);
      toast.success('Screen sharing started');
    } catch (err) {
      if ((err as Error).name !== 'NotAllowedError') {
        toast.error('Failed to start screen share');
      }
    }
  };

  const stopScreenShare = () => {
    if (screenStreamRef.current) {
      screenStreamRef.current.getTracks().forEach((t) => t.stop());
      screenStreamRef.current = null;
    }
    if (screenPreviewRef.current) {
      screenPreviewRef.current.srcObject = null;
    }
    // Rebuild mixed stream with mic only
    if (meetingState === 'active' && micStreamRef.current) {
      rebuildMixedStream(micStreamRef.current, null);
    }
    setIsScreenSharing(false);
  };

  // Mix mic + optional screen audio into a single stream for the MediaRecorder
  const rebuildMixedStream = (micStream: MediaStream, screenStream: MediaStream | null) => {
    // Close previous AudioContext
    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }

    const ctx = new AudioContext();
    audioContextRef.current = ctx;
    const dest = ctx.createMediaStreamDestination();

    // Mic
    ctx.createMediaStreamSource(micStream).connect(dest);

    // Screen audio (if any audio tracks)
    if (screenStream) {
      const audioTracks = screenStream.getAudioTracks();
      if (audioTracks.length > 0) {
        const screenAudio = new MediaStream(audioTracks);
        ctx.createMediaStreamSource(screenAudio).connect(dest);
      }
    }

    mixedStreamRef.current = dest.stream;

    // Restart MediaRecorder on the new mixed stream (if meeting is active)
    if (mediaRecorderRef.current && meetingState === 'active') {
      mediaRecorderRef.current.stop();
      startMediaRecorder(dest.stream);
    }
  };

  const startMediaRecorder = (stream: MediaStream) => {
    const recorder = new MediaRecorder(stream, {
      mimeType: MediaRecorder.isTypeSupported('audio/webm;codecs=opus')
        ? 'audio/webm;codecs=opus'
        : 'audio/webm',
    });
    mediaRecorderRef.current = recorder;
    let localChunkIndex = 0;
    recorder.ondataavailable = (e) => {
      if (e.data.size > 0) {
        const idx = localChunkIndex++;
        setChunkIndex(idx);
        processAudioChunkRef.current(e.data, idx);
      }
    };
    recorder.start(5000); // 5-second chunks
  };

  // ── Participant management ──────────────────────────────────────────────────

  const addParticipant = () => {
    const name = participantInput.trim();
    if (name && !participants.includes(name)) {
      setParticipants((prev) => [...prev, name]);
    }
    setParticipantInput('');
  };

  const removeParticipant = (name: string) => {
    setParticipants((prev) => prev.filter((p) => p !== name));
  };

  // ── Meeting lifecycle ───────────────────────────────────────────────────────

  const processAudioChunk = useCallback(
    async (blob: Blob, idx: number) => {
      if (!sessionId || meetingState !== 'active') return;

      try {
        const reader = new FileReader();
        const base64Promise = new Promise<string>((resolve) => {
          reader.onloadend = () => resolve(((reader.result as string).split(',')[1]) || '');
          reader.readAsDataURL(blob);
        });
        const audioData = await base64Promise;

        const res = await fetch(resolveApiUrl('/api/topsi/meeting/audio'), {
          method: 'POST',
          headers: getAuthHeaders(),
          credentials: 'include',
          body: JSON.stringify({ sessionId, audioData, chunkIndex: idx, durationMs: 5000 }),
        });

        if (!res.ok) return;
        const data = await res.json();

        if (data.text && data.text.trim()) {
          setTranscript((prev) => [...prev, {
            text: data.text,
            speakerLabel: data.speakerLabel,
            isTopsiAddressed: data.isTopsiAddressed,
            topsiResponse: data.topsiResponse,
            timestamp: Date.now(),
            segmentIndex: data.segmentIndex,
          }]);

          if (data.topsiResponse) {
            setTopsiOverlay(data.topsiResponse);
            setTimeout(() => setTopsiOverlay(null), 8000);
            if (data.topsiAudioResponse) {
              try {
                const audio = new Audio(`data:audio/wav;base64,${data.topsiAudioResponse}`);
                currentAudioRef.current = audio;
                await audio.play();
              } catch { /* silent */ }
            }
          }

          if (data.speakerLabel) setParticipantCount((p) => Math.max(p, 1));
        }
      } catch (err) {
        console.error('Failed to process audio chunk:', err);
      }
    },
    [sessionId, meetingState]
  );
  // Keep the ref pointing at the latest version so MediaRecorder callbacks never go stale
  processAudioChunkRef.current = processAudioChunk;

  const startMeeting = async () => {
    if (!activeProjectId) {
      toast.error('Please select a project before starting the meeting');
      return;
    }

    setIsProcessing(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/meeting/start'), {
        method: 'POST',
        headers: getAuthHeaders(),
        credentials: 'include',
        body: JSON.stringify({
          projectId: activeProjectId,
          title: meetingTitle || undefined,
          participants: participants.length > 0 ? participants : undefined,
        }),
      });

      if (!res.ok) {
        const errBody = await res.json().catch(() => ({}));
        throw new Error(errBody?.message || `Server error ${res.status}`);
      }

      const data = await res.json();
      setSessionId(data.sessionId);
      setMeetingTitle(data.title);

      // Start mic — request permission explicitly
      let micStream: MediaStream;
      try {
        micStream = await navigator.mediaDevices.getUserMedia({ audio: true });
      } catch (micErr) {
        const name = (micErr as DOMException)?.name;
        if (name === 'NotAllowedError' || name === 'PermissionDeniedError') {
          toast.error('Microphone permission denied — please allow mic access in your browser and try again');
        } else if (name === 'NotFoundError') {
          toast.error('No microphone found — please connect a mic and try again');
        } else {
          toast.error(`Microphone error: ${(micErr as Error).message}`);
        }
        setIsProcessing(false);
        return;
      }
      micStreamRef.current = micStream;

      // Build initial mixed stream (mic only at first)
      const ctx = new AudioContext();
      audioContextRef.current = ctx;
      const dest = ctx.createMediaStreamDestination();
      ctx.createMediaStreamSource(micStream).connect(dest);
      mixedStreamRef.current = dest.stream;

      startMediaRecorder(dest.stream);
      setMeetingState('active');
      setDuration(0);
      setTranscript([]);
      setNotes(null);
      toast.success('Meeting started — Topsi is listening');
    } catch (err) {
      console.error('Failed to start meeting:', err);
      toast.error(`Failed to start meeting: ${(err as Error).message}`);
    } finally {
      setIsProcessing(false);
    }
  };

  const pauseMeeting = () => {
    mediaRecorderRef.current?.pause();
    setMeetingState('paused');
  };

  const resumeMeeting = () => {
    mediaRecorderRef.current?.resume();
    setMeetingState('active');
  };

  const endMeeting = async () => {
    mediaRecorderRef.current?.stop();
    mediaRecorderRef.current = null;
    micStreamRef.current?.getTracks().forEach((t) => t.stop());
    micStreamRef.current = null;
    stopScreenShare();
    if (audioContextRef.current) { audioContextRef.current.close(); audioContextRef.current = null; }

    setMeetingState('ended');
    setIsProcessing(true);

    try {
      if (sessionId) {
        const res = await fetch(resolveApiUrl('/api/topsi/meeting/end'), {
          method: 'POST',
          headers: getAuthHeaders(),
          credentials: 'include',
          body: JSON.stringify({ sessionId, generateNotes: true }),
        });

        if (res.ok) {
          const data = await res.json();
          if (data.notes) setNotes(data.notes);
          setParticipantCount(data.participantCount || participants.length);
          toast.success('Meeting ended — notes generated');
        }
      }
    } catch (err) {
      console.error('Failed to end meeting:', err);
      toast.error('Failed to generate meeting notes');
    } finally {
      setIsProcessing(false);
    }
  };

  const resetMeeting = () => {
    setMeetingState('idle');
    setSessionId(null);
    setTranscript([]);
    setDuration(0);
    setParticipantCount(0);
    setChunkIndex(0);
    setNotes(null);
    setMeetingTitle('');
    setParticipants([]);
    setIsScreenSharing(false);
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      mediaRecorderRef.current?.stop();
      micStreamRef.current?.getTracks().forEach((t) => t.stop());
      screenStreamRef.current?.getTracks().forEach((t) => t.stop());
      if (timerRef.current) clearInterval(timerRef.current);
      audioContextRef.current?.close();
    };
  }, []);

  // ── Render ──────────────────────────────────────────────────────────────────

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Control Bar */}
      <div className="flex items-center justify-between p-3 border-b bg-cyan-50 dark:bg-cyan-950">
        <div className="flex items-center gap-2">
          <Network className="h-4 w-4 text-cyan-600" />
          <span className="text-sm font-medium">
            {meetingState === 'idle' ? 'New Meeting' : meetingTitle || 'Meeting'}
          </span>
          {meetingState === 'active' && (
            <Badge className="bg-red-500 text-white text-xs animate-pulse">REC</Badge>
          )}
          {meetingState === 'paused' && (
            <Badge variant="secondary" className="text-xs">Paused</Badge>
          )}
          {isScreenSharing && (
            <Badge className="bg-blue-500 text-white text-xs">
              <Monitor className="h-3 w-3 mr-1" />
              Screen
            </Badge>
          )}
        </div>
        <div className="flex items-center gap-2">
          {(meetingState === 'active' || meetingState === 'paused') && (
            <>
              <div className="flex items-center gap-1 text-xs text-muted-foreground">
                <Clock className="h-3 w-3" />
                {formatDuration(duration)}
              </div>
              <div className="flex items-center gap-1 text-xs text-muted-foreground">
                <Users className="h-3 w-3" />
                {Math.max(participantCount, participants.length)}
              </div>
            </>
          )}
        </div>
      </div>

      {/* ── Idle: Setup ────────────────────────────────────────────────────── */}
      {meetingState === 'idle' && (
        <ScrollArea className="flex-1">
          <div className="flex flex-col items-center p-6 gap-4 max-w-sm mx-auto">
            <div className="w-16 h-16 rounded-full bg-cyan-100 dark:bg-cyan-900 flex items-center justify-center">
              <Mic className="h-8 w-8 text-cyan-600" />
            </div>
            <div className="text-center">
              <h3 className="font-semibold">Meeting Mode</h3>
              <p className="text-sm text-muted-foreground mt-1">
                Topsi listens silently and takes structured notes.
                <br />Say <strong>"Topsi"</strong> to get its attention.
              </p>
            </div>

            {/* Project selector */}
            <div className="w-full space-y-1.5">
              <Label className="text-xs font-medium">Project</Label>
              {propProjectId ? (
                <p className="text-sm text-muted-foreground px-1">
                  {projects.find((p) => p.id === propProjectId)?.name || propProjectId}
                </p>
              ) : (
                <Select value={selectedProjectId} onValueChange={setSelectedProjectId}>
                  <SelectTrigger className="h-9 text-sm">
                    <SelectValue placeholder="Select a project…" />
                  </SelectTrigger>
                  <SelectContent>
                    {projects.map((p) => (
                      <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>

            {/* Title */}
            <div className="w-full space-y-1.5">
              <Label className="text-xs font-medium">Meeting title <span className="text-muted-foreground">(optional)</span></Label>
              <Input
                placeholder="e.g. Bulgari Perfume kick-off"
                value={meetingTitle}
                onChange={(e) => setMeetingTitle(e.target.value)}
                className="h-9 text-sm"
              />
            </div>

            {/* Participants */}
            <div className="w-full space-y-1.5">
              <Label className="text-xs font-medium">Participants <span className="text-muted-foreground">(optional, for speaker attribution)</span></Label>
              <div className="flex gap-1.5">
                <Input
                  placeholder="Name or email"
                  value={participantInput}
                  onChange={(e) => setParticipantInput(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && addParticipant()}
                  className="h-9 text-sm"
                />
                <Button variant="outline" size="sm" onClick={addParticipant} className="h-9 px-2">
                  <UserPlus className="h-4 w-4" />
                </Button>
              </div>
              {participants.length > 0 && (
                <div className="flex flex-wrap gap-1 mt-1">
                  {participants.map((p) => (
                    <Badge key={p} variant="secondary" className="text-xs gap-1">
                      {p}
                      <button onClick={() => removeParticipant(p)} className="hover:text-destructive">
                        <X className="h-2.5 w-2.5" />
                      </button>
                    </Badge>
                  ))}
                </div>
              )}
            </div>

            <Button
              onClick={startMeeting}
              disabled={isProcessing || !activeProjectId}
              className="bg-cyan-600 hover:bg-cyan-700 w-full"
            >
              {isProcessing ? (
                <Loader2 className="h-4 w-4 animate-spin mr-2" />
              ) : (
                <Mic className="h-4 w-4 mr-2" />
              )}
              Start Meeting
            </Button>
          </div>
        </ScrollArea>
      )}

      {/* ── Active / Paused ────────────────────────────────────────────────── */}
      {(meetingState === 'active' || meetingState === 'paused') && (
        <>
          {/* Screen share preview */}
          {isScreenSharing && (
            <div className="mx-3 mt-2 relative">
              <video
                ref={screenPreviewRef}
                autoPlay
                muted
                playsInline
                className="w-full max-h-32 rounded-lg border bg-black object-contain"
              />
              <button
                onClick={stopScreenShare}
                className="absolute top-1 right-1 bg-black/60 text-white rounded-full p-1 hover:bg-black/80"
                title="Stop screen share"
              >
                <MonitorOff className="h-3.5 w-3.5" />
              </button>
            </div>
          )}

          {/* Topsi Response Overlay */}
          {topsiOverlay && (
            <div className="mx-3 mt-2 p-3 bg-cyan-50 dark:bg-cyan-950 border border-cyan-200 dark:border-cyan-800 rounded-lg">
              <div className="flex items-center gap-2 mb-1">
                <Network className="h-4 w-4 text-cyan-600" />
                <span className="text-xs font-semibold text-cyan-600">Topsi</span>
              </div>
              <p className="text-sm">{topsiOverlay}</p>
            </div>
          )}

          {/* Transcript */}
          <ScrollArea className="flex-1 p-3">
            <div className="space-y-2">
              {transcript.length === 0 && (
                <div className="text-center text-sm text-muted-foreground py-8">
                  {meetingState === 'active' ? 'Listening… speak to see the transcript.' : 'Recording paused.'}
                </div>
              )}
              {transcript.map((entry, i) => (
                <div
                  key={i}
                  className={cn(
                    'text-sm rounded-lg p-2',
                    entry.isTopsiAddressed
                      ? 'bg-cyan-50 dark:bg-cyan-950 border border-cyan-200 dark:border-cyan-800'
                      : 'bg-muted'
                  )}
                >
                  {entry.speakerLabel && (
                    <span className="text-xs font-semibold text-muted-foreground">{entry.speakerLabel}: </span>
                  )}
                  <span>{entry.text}</span>
                  {entry.topsiResponse && (
                    <div className="mt-1 pt-1 border-t border-cyan-200 dark:border-cyan-800">
                      <span className="text-xs font-semibold text-cyan-600">Topsi: </span>
                      <span className="text-xs">{entry.topsiResponse}</span>
                    </div>
                  )}
                </div>
              ))}
              <div ref={transcriptEndRef} />
            </div>
          </ScrollArea>

          {/* Controls */}
          <div className="p-3 border-t flex items-center justify-between">
            {/* Screen share toggle */}
            <Button
              variant={isScreenSharing ? 'default' : 'outline'}
              size="sm"
              onClick={isScreenSharing ? stopScreenShare : startScreenShare}
              className={cn(isScreenSharing && 'bg-blue-600 hover:bg-blue-700')}
              title={isScreenSharing ? 'Stop screen share' : 'Share screen'}
            >
              {isScreenSharing ? (
                <MonitorOff className="h-4 w-4 mr-1" />
              ) : (
                <Monitor className="h-4 w-4 mr-1" />
              )}
              {isScreenSharing ? 'Stop sharing' : 'Share screen'}
            </Button>

            <div className="flex items-center gap-2">
              {meetingState === 'active' ? (
                <Button variant="outline" size="icon" onClick={pauseMeeting} title="Pause">
                  <Pause className="h-4 w-4" />
                </Button>
              ) : (
                <Button variant="outline" size="icon" onClick={resumeMeeting} title="Resume">
                  <Play className="h-4 w-4" />
                </Button>
              )}
              <Button
                variant="destructive"
                size="icon"
                onClick={endMeeting}
                title="End meeting"
                className="h-10 w-10"
              >
                <Square className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </>
      )}

      {/* ── Ended: Notes ───────────────────────────────────────────────────── */}
      {meetingState === 'ended' && (
        <ScrollArea className="flex-1 p-3">
          <div className="space-y-4">
            {isProcessing ? (
              <div className="flex flex-col items-center justify-center py-8 gap-3">
                <Loader2 className="h-8 w-8 animate-spin text-cyan-600" />
                <p className="text-sm text-muted-foreground">Generating meeting notes…</p>
              </div>
            ) : notes ? (
              <>
                <div className="flex items-center gap-2 mb-2">
                  <FileText className="h-4 w-4 text-cyan-600" />
                  <span className="font-semibold">Meeting Notes</span>
                  <Badge variant="secondary" className="text-xs">{formatDuration(duration)}</Badge>
                </div>

                {notes.summary && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Summary</h4>
                    <p className="text-sm">{notes.summary}</p>
                  </div>
                )}

                {notes.topics?.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Topics</h4>
                    <ul className="text-sm list-disc list-inside space-y-0.5">
                      {notes.topics.map((t, i) => <li key={i}>{t}</li>)}
                    </ul>
                  </div>
                )}

                {notes.decisions?.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Decisions</h4>
                    <ul className="text-sm list-disc list-inside space-y-0.5">
                      {notes.decisions.map((d, i) => <li key={i}>{d}</li>)}
                    </ul>
                  </div>
                )}

                {notes.actionItems?.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Action Items</h4>
                    <ul className="text-sm space-y-1">
                      {notes.actionItems.map((item, i) => (
                        <li key={i} className="flex items-start gap-2">
                          <span className="text-cyan-600 mt-0.5">–</span>
                          <div>
                            <span>{item.description}</span>
                            {item.assignee && (
                              <Badge variant="outline" className="ml-1 text-xs">{item.assignee}</Badge>
                            )}
                            {item.deadline && (
                              <span className="text-xs text-muted-foreground ml-1">(by {item.deadline})</span>
                            )}
                            {item.priority && (
                              <Badge
                                className={cn('ml-1 text-xs', item.priority === 'high' && 'bg-red-100 text-red-700')}
                                variant="secondary"
                              >
                                {item.priority}
                              </Badge>
                            )}
                          </div>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {notes.openQuestions?.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Open Questions</h4>
                    <ul className="text-sm list-disc list-inside space-y-0.5">
                      {notes.openQuestions.map((q, i) => <li key={i}>{q}</li>)}
                    </ul>
                  </div>
                )}

                {notes.participants?.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Participants</h4>
                    <div className="flex flex-wrap gap-1">
                      {notes.participants.map((p, i) => (
                        <Badge key={i} variant="secondary" className="text-xs">{p}</Badge>
                      ))}
                    </div>
                  </div>
                )}
              </>
            ) : (
              <div className="text-center py-8">
                <p className="text-sm text-muted-foreground">
                  Meeting ended. {transcript.length} segments recorded.
                </p>
              </div>
            )}

            <div className="flex items-center justify-center gap-2 pt-2">
              <Button variant="outline" size="sm" onClick={resetMeeting}>New Meeting</Button>
              {onClose && <Button variant="ghost" size="sm" onClick={onClose}>Close</Button>}
            </div>
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

export default MeetingMode;
