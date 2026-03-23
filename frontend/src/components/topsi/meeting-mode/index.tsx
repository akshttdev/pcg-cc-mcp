// ── Meeting Mode ────────────────────────────────────────────────────────────
//
// Real-time meeting recording with Topsi AI note-taking.
// Split from the original monolithic MeetingMode.tsx:
//   - types.ts        — shared types & helpers
//   - MeetingSetup    — idle state (new meeting form + join active)
//   - MeetingNotesView — ended state (notes display + publish)
//   - ControlBar      — top status bar
//   - ActiveSession   — transcript, chat, screen share, controls
//   - index.tsx        — orchestrator with all state and hooks

import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';

import { useProjectList } from '@/hooks/queries';
import { meetingsApi, resolveApiUrl } from '@/lib/api';
import { cn } from '@/lib/utils';

import { ActiveSession } from './ActiveSession';
import { ControlBar } from './ControlBar';
import { MeetingNotesView } from './MeetingNotesView';
import { MeetingSetup } from './MeetingSetup';
import type {
  ActiveMeeting,
  MeetingModeProps,
  MeetingNotes,
  MeetingRole,
  MeetingState,
  TranscriptEntry,
} from './types';
import { getAuthHeaders, isUrl } from './types';

export function MeetingMode({ projectId: propProjectId, onClose, className }: MeetingModeProps) {
  // Meeting state
  const [meetingState, setMeetingState] = useState<MeetingState>('idle');
  const [meetingRole, setMeetingRole] = useState<MeetingRole>('host');
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [transcript, setTranscript] = useState<TranscriptEntry[]>([]);
  const [duration, setDuration] = useState(0);
  const [participantCount, setParticipantCount] = useState(0);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars, unused-imports/no-unused-vars
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

  // Active meetings for join tab
  const [activeMeetings, setActiveMeetings] = useState<ActiveMeeting[]>([]);
  const [isLoadingActive, setIsLoadingActive] = useState(false);

  // Collaborative chat / link sharing
  const [chatInput, setChatInput] = useState('');
  const [isSendingChat, setIsSendingChat] = useState(false);

  // Knowledge graph publish
  const [isPublishing, setIsPublishing] = useState(false);
  const [publishedKgId, setPublishedKgId] = useState<string | null>(null);

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
  // First WebM chunk contains the EBML container header — must be prepended to all later chunks
  const firstChunkRef = useRef<Blob | null>(null);
  // Track which server segment indices we've already shown (to avoid duplicates when polling)
  const seenSegmentIndicesRef = useRef<Set<number>>(new Set());

  // Fetch all projects for the selector
  const { data: projects = [] } = useProjectList();

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

  // ── Transcript polling from server ──────────────────────────────────────────

  useEffect(() => {
    if (meetingState !== 'active' || !sessionId) return;

    const pollTranscript = async () => {
      try {
        const res = await fetch(resolveApiUrl(`/api/topsi/meeting/transcript/${sessionId}`), {
          headers: getAuthHeaders(),
          credentials: 'include',
        });
        if (!res.ok) return;
        const data = await res.json();
        const segments: Array<{
          segmentIndex: number;
          segment_index?: number;
          text: string;
          speakerLabel?: string;
          speaker_label?: string;
          isTopsiAddressed?: boolean;
          is_topsi_addressed?: boolean;
          metadata?: string;
        }> = data.segments || [];

        const newEntries: TranscriptEntry[] = [];
        for (const seg of segments) {
          const idx = seg.segmentIndex ?? seg.segment_index ?? 0;
          if (seenSegmentIndicesRef.current.has(idx)) continue;
          seenSegmentIndicesRef.current.add(idx);

          // Parse Topsi response from metadata if present
          let topsiResponse: string | undefined;
          if (seg.metadata) {
            try {
              const meta = JSON.parse(seg.metadata);
              if (meta.topsi_response) topsiResponse = meta.topsi_response;
            } catch { /* ignore */ }
          }

          newEntries.push({
            text: seg.text,
            speakerLabel: seg.speakerLabel ?? seg.speaker_label,
            isTopsiAddressed: seg.isTopsiAddressed ?? seg.is_topsi_addressed ?? false,
            topsiResponse,
            timestamp: Date.now(),
            segmentIndex: idx,
          });
        }

        if (newEntries.length > 0) {
          setTranscript((prev) => [...prev, ...newEntries]);
        }
      } catch {
        // silent — polling errors are expected when offline
      }
    };

    // Poll every 3 seconds
    const pollInterval = setInterval(pollTranscript, 3000);
    return () => clearInterval(pollInterval);
  }, [meetingState, sessionId]);

  // ── Active meetings fetch ────────────────────────────────────────────────────

  const fetchActiveMeetings = useCallback(async () => {
    setIsLoadingActive(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/meeting/list?status=active&limit=20'), {
        headers: getAuthHeaders(),
        credentials: 'include',
      });
      if (!res.ok) return;
      const data = await res.json();
      const meetings: ActiveMeeting[] = (data.meetings || []).map((m: {
        id: string;
        title: string;
        project_id?: string;
        projectId?: string;
        status: string;
        started_by?: string;
        startedBy?: string;
        started_at?: string;
        startedAt?: string;
        participant_count?: number;
        participantCount?: number;
      }) => ({
        id: m.id,
        title: m.title,
        projectId: m.project_id ?? m.projectId ?? '',
        status: m.status,
        startedBy: m.started_by ?? m.startedBy ?? '',
        startedAt: m.started_at ?? m.startedAt ?? '',
        participantCount: m.participant_count ?? m.participantCount,
      }));
      setActiveMeetings(meetings);
    } catch {
      toast.error('Failed to load active meetings');
    } finally {
      setIsLoadingActive(false);
    }
  }, []);

  // ── Screen share ────────────────────────────────────────────────────────────

  const startScreenShare = async () => {
    try {
      const screenStream = await navigator.mediaDevices.getDisplayMedia({
        video: { frameRate: 5, width: { ideal: 1280 } },
        audio: true,
      });
      screenStreamRef.current = screenStream;

      if (screenPreviewRef.current) {
        screenPreviewRef.current.srcObject = screenStream;
        screenPreviewRef.current.play().catch(() => {});
      }

      if (meetingState === 'active' && micStreamRef.current) {
        rebuildMixedStream(micStreamRef.current, screenStream);
      }

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
    if (meetingState === 'active' && micStreamRef.current) {
      rebuildMixedStream(micStreamRef.current, null);
    }
    setIsScreenSharing(false);
  };

  const rebuildMixedStream = (micStream: MediaStream, screenStream: MediaStream | null) => {
    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }

    const ctx = new AudioContext();
    audioContextRef.current = ctx;
    const dest = ctx.createMediaStreamDestination();

    ctx.createMediaStreamSource(micStream).connect(dest);

    if (screenStream) {
      const audioTracks = screenStream.getAudioTracks();
      if (audioTracks.length > 0) {
        const screenAudio = new MediaStream(audioTracks);
        ctx.createMediaStreamSource(screenAudio).connect(dest);
      }
    }

    mixedStreamRef.current = dest.stream;

    if (mediaRecorderRef.current && meetingState === 'active') {
      mediaRecorderRef.current.stop();
      startMediaRecorder(dest.stream);
    }
  };

  const startMediaRecorder = (stream: MediaStream) => {
    firstChunkRef.current = null;
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
        if (idx === 0) {
          firstChunkRef.current = e.data;
          processAudioChunkRef.current(e.data, idx);
        } else {
          const blob = firstChunkRef.current
            ? new Blob([firstChunkRef.current, e.data], { type: e.data.type })
            : e.data;
          processAudioChunkRef.current(blob, idx);
        }
      }
    };
    recorder.start(5000);
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
          const segIdx = data.segmentIndex as number;
          if (!seenSegmentIndicesRef.current.has(segIdx)) {
            seenSegmentIndicesRef.current.add(segIdx);
            setTranscript((prev) => [...prev, {
              text: data.text,
              speakerLabel: data.speakerLabel,
              isTopsiAddressed: data.isTopsiAddressed,
              topsiResponse: data.topsiResponse,
              timestamp: Date.now(),
              segmentIndex: segIdx,
            }]);
          }

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
  processAudioChunkRef.current = processAudioChunk;

  // ── Start mic and begin sending audio (shared by host start + observer join) ──

  const beginAudioCapture = async (sid: string) => {
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
      return false;
    }
    micStreamRef.current = micStream;

    const ctx = new AudioContext();
    audioContextRef.current = ctx;
    const dest = ctx.createMediaStreamDestination();
    ctx.createMediaStreamSource(micStream).connect(dest);
    mixedStreamRef.current = dest.stream;

    seenSegmentIndicesRef.current = new Set();
    setSessionId(sid);
    startMediaRecorder(dest.stream);
    setMeetingState('active');
    setDuration(0);
    setTranscript([]);
    setNotes(null);
    return true;
  };

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
      setMeetingTitle(data.title);
      setMeetingRole('host');

      const ok = await beginAudioCapture(data.sessionId);
      if (ok) {
        toast.success('Meeting started — Topsi is listening');
      }
    } catch (err) {
      console.error('Failed to start meeting:', err);
      toast.error(`Failed to start meeting: ${(err as Error).message}`);
    } finally {
      setIsProcessing(false);
    }
  };

  const joinMeeting = async (meeting: ActiveMeeting) => {
    setIsProcessing(true);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/meeting/join'), {
        method: 'POST',
        headers: getAuthHeaders(),
        credentials: 'include',
        body: JSON.stringify({ sessionId: meeting.id }),
      });

      if (!res.ok) {
        const errBody = await res.json().catch(() => ({}));
        throw new Error(errBody?.message || `Server error ${res.status}`);
      }

      const data = await res.json();
      setMeetingTitle(data.title);
      setMeetingRole('observer');
      setParticipantCount(data.participantCount || 1);

      const ok = await beginAudioCapture(data.sessionId);
      if (ok) {
        toast.success(`Joined "${data.title}" — Topsi is listening`);
      }
    } catch (err) {
      console.error('Failed to join meeting:', err);
      toast.error(`Failed to join: ${(err as Error).message}`);
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

    if (meetingRole === 'observer') {
      setIsProcessing(false);
      toast.success('You have left the meeting');
      return;
    }

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
    setPublishedKgId(null);
    seenSegmentIndicesRef.current = new Set();
  };

  const publishToKnowledgeGraph = async () => {
    if (!sessionId || !activeProjectId) {
      toast.error('No session or project to publish');
      return;
    }
    setIsPublishing(true);
    try {
      const result = await meetingsApi.publish(sessionId, {
        project_id: activeProjectId,
        source_title: meetingTitle || `Meeting ${new Date().toLocaleDateString()}`,
      });
      setPublishedKgId(result.knowledge_source_id);
      toast.success('Meeting notes published to knowledge graph');
    } catch {
      toast.error('Failed to publish meeting notes');
    } finally {
      setIsPublishing(false);
    }
  };

  // ── Collaborative chat / link sharing ───────────────────────────────────────

  const sendChatMessage = async () => {
    const text = chatInput.trim();
    if (!text || !sessionId || isSendingChat) return;

    setIsSendingChat(true);
    const link = isUrl(text);
    try {
      const res = await fetch(resolveApiUrl('/api/topsi/meeting/message'), {
        method: 'POST',
        headers: getAuthHeaders(),
        credentials: 'include',
        body: JSON.stringify({
          sessionId,
          text,
          speakerLabel: 'You',
          isLink: link,
        }),
      });
      if (!res.ok) throw new Error('Failed to send');
      setChatInput('');
    } catch {
      toast.error('Failed to send message');
    } finally {
      setIsSendingChat(false);
    }
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
      <ControlBar
        meetingState={meetingState}
        meetingRole={meetingRole}
        meetingTitle={meetingTitle}
        isScreenSharing={isScreenSharing}
        duration={duration}
        participantCount={participantCount}
        participants={participants}
      />

      {/* ── Idle: Setup + Join ────────────────────────────────────────────── */}
      {meetingState === 'idle' && (
        <MeetingSetup
          propProjectId={propProjectId}
          selectedProjectId={selectedProjectId}
          onSelectedProjectIdChange={setSelectedProjectId}
          projects={projects}
          activeProjectId={activeProjectId}
          meetingTitle={meetingTitle}
          onMeetingTitleChange={setMeetingTitle}
          participants={participants}
          participantInput={participantInput}
          onParticipantInputChange={setParticipantInput}
          onAddParticipant={addParticipant}
          onRemoveParticipant={removeParticipant}
          isProcessing={isProcessing}
          onStartMeeting={startMeeting}
          activeMeetings={activeMeetings}
          isLoadingActive={isLoadingActive}
          onFetchActiveMeetings={fetchActiveMeetings}
          onJoinMeeting={joinMeeting}
        />
      )}

      {/* ── Active / Paused ────────────────────────────────────────────────── */}
      {(meetingState === 'active' || meetingState === 'paused') && (
        <ActiveSession
          meetingState={meetingState}
          meetingRole={meetingRole}
          transcript={transcript}
          isScreenSharing={isScreenSharing}
          topsiOverlay={topsiOverlay}
          chatInput={chatInput}
          isSendingChat={isSendingChat}
          screenPreviewRef={screenPreviewRef}
          transcriptEndRef={transcriptEndRef}
          onChatInputChange={setChatInput}
          onSendChatMessage={sendChatMessage}
          onStartScreenShare={startScreenShare}
          onStopScreenShare={stopScreenShare}
          onPauseMeeting={pauseMeeting}
          onResumeMeeting={resumeMeeting}
          onEndMeeting={endMeeting}
        />
      )}

      {/* ── Ended: Notes ───────────────────────────────────────────────────── */}
      {meetingState === 'ended' && (
        <MeetingNotesView
          notes={notes}
          duration={duration}
          transcriptLength={transcript.length}
          meetingRole={meetingRole}
          isProcessing={isProcessing}
          isPublishing={isPublishing}
          publishedKgId={publishedKgId}
          onPublishToKnowledgeGraph={publishToKnowledgeGraph}
          onResetMeeting={resetMeeting}
          onClose={onClose}
        />
      )}
    </div>
  );
}
