import { useState, useEffect, useRef, useCallback } from 'react';
import { resolveApiUrl } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
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
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

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
  projectId?: string;
  onClose?: () => void;
  className?: string;
}

type MeetingState = 'idle' | 'active' | 'paused' | 'ended';

export function MeetingMode({ projectId, onClose, className }: MeetingModeProps) {
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

  // Refs
  const mediaRecorderRef = useRef<MediaRecorder | null>(null);
  const streamRef = useRef<MediaStream | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const transcriptEndRef = useRef<HTMLDivElement>(null);
  const currentAudioRef = useRef<HTMLAudioElement | null>(null);

  // Auto-scroll transcript
  useEffect(() => {
    transcriptEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [transcript]);

  // Duration timer
  useEffect(() => {
    if (meetingState === 'active') {
      timerRef.current = setInterval(() => {
        setDuration((d) => d + 1);
      }, 1000);
    } else {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    }
    return () => {
      if (timerRef.current) clearInterval(timerRef.current);
    };
  }, [meetingState]);

  const formatDuration = (seconds: number) => {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
    return `${m}:${String(s).padStart(2, '0')}`;
  };

  const getAuthHeaders = (): Record<string, string> => {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    const token =
      localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    return headers;
  };

  const processAudioChunk = useCallback(
    async (blob: Blob, idx: number) => {
      if (!sessionId || meetingState !== 'active') return;

      try {
        // Convert blob to base64
        const reader = new FileReader();
        const base64Promise = new Promise<string>((resolve) => {
          reader.onloadend = () => {
            const base64 = (reader.result as string).split(',')[1] || '';
            resolve(base64);
          };
          reader.readAsDataURL(blob);
        });
        const audioData = await base64Promise;

        const res = await fetch(resolveApiUrl('/api/topsi/meeting/audio'), {
          method: 'POST',
          headers: getAuthHeaders(),
          credentials: 'include',
          body: JSON.stringify({
            sessionId,
            audioData,
            chunkIndex: idx,
            durationMs: 5000,
          }),
        });

        if (!res.ok) return;

        const data = await res.json();

        if (data.text && data.text.trim()) {
          const entry: TranscriptEntry = {
            text: data.text,
            speakerLabel: data.speakerLabel,
            isTopsiAddressed: data.isTopsiAddressed,
            topsiResponse: data.topsiResponse,
            timestamp: Date.now(),
            segmentIndex: data.segmentIndex,
          };

          setTranscript((prev) => [...prev, entry]);

          // If Topsi responded, show overlay and play audio
          if (data.topsiResponse) {
            setTopsiOverlay(data.topsiResponse);
            setTimeout(() => setTopsiOverlay(null), 8000);

            if (data.topsiAudioResponse) {
              try {
                const audio = new Audio(`data:audio/wav;base64,${data.topsiAudioResponse}`);
                currentAudioRef.current = audio;
                await audio.play();
              } catch {
                // Audio playback failed silently
              }
            }
          }

          // Update speaker count from unique labels
          if (data.speakerLabel) {
            setParticipantCount((prev) => {
              // This is approximate — real count comes from backend
              return Math.max(prev, 1);
            });
          }
        }
      } catch (err) {
        console.error('Failed to process audio chunk:', err);
      }
    },
    [sessionId, meetingState]
  );

  const startMeeting = async () => {
    if (!projectId) {
      toast.error('No project selected for meeting');
      return;
    }

    setIsProcessing(true);
    try {
      // Start meeting session
      const res = await fetch(resolveApiUrl('/api/topsi/meeting/start'), {
        method: 'POST',
        headers: getAuthHeaders(),
        credentials: 'include',
        body: JSON.stringify({
          projectId,
          title: meetingTitle || undefined,
        }),
      });

      if (!res.ok) throw new Error('Failed to start meeting');

      const data = await res.json();
      setSessionId(data.sessionId);
      setMeetingTitle(data.title);

      // Start mic capture
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      streamRef.current = stream;

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
          processAudioChunk(e.data, idx);
        }
      };

      // Start recording with 5-second chunks
      recorder.start(5000);

      setMeetingState('active');
      setDuration(0);
      setTranscript([]);
      setNotes(null);
      toast.success('Meeting started — Topsi is listening');
    } catch (err) {
      console.error('Failed to start meeting:', err);
      toast.error('Failed to start meeting');
    } finally {
      setIsProcessing(false);
    }
  };

  const pauseMeeting = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'recording') {
      mediaRecorderRef.current.pause();
    }
    setMeetingState('paused');
  };

  const resumeMeeting = () => {
    if (mediaRecorderRef.current && mediaRecorderRef.current.state === 'paused') {
      mediaRecorderRef.current.resume();
    }
    setMeetingState('active');
  };

  const endMeeting = async () => {
    // Stop recording
    if (mediaRecorderRef.current) {
      mediaRecorderRef.current.stop();
      mediaRecorderRef.current = null;
    }
    if (streamRef.current) {
      streamRef.current.getTracks().forEach((t) => t.stop());
      streamRef.current = null;
    }

    setMeetingState('ended');
    setIsProcessing(true);

    try {
      if (sessionId) {
        const res = await fetch(resolveApiUrl('/api/topsi/meeting/end'), {
          method: 'POST',
          headers: getAuthHeaders(),
          credentials: 'include',
          body: JSON.stringify({
            sessionId,
            generateNotes: true,
          }),
        });

        if (res.ok) {
          const data = await res.json();
          if (data.notes) {
            setNotes(data.notes);
          }
          setParticipantCount(data.participantCount || 0);
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
  };

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (mediaRecorderRef.current) {
        mediaRecorderRef.current.stop();
      }
      if (streamRef.current) {
        streamRef.current.getTracks().forEach((t) => t.stop());
      }
      if (timerRef.current) {
        clearInterval(timerRef.current);
      }
    };
  }, []);

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Meeting Control Bar */}
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
        </div>
        <div className="flex items-center gap-2">
          {meetingState === 'active' || meetingState === 'paused' ? (
            <>
              <div className="flex items-center gap-1 text-xs text-muted-foreground">
                <Clock className="h-3 w-3" />
                {formatDuration(duration)}
              </div>
              <div className="flex items-center gap-1 text-xs text-muted-foreground">
                <Users className="h-3 w-3" />
                {participantCount}
              </div>
            </>
          ) : null}
        </div>
      </div>

      {/* Idle State - Start Meeting */}
      {meetingState === 'idle' && (
        <div className="flex-1 flex flex-col items-center justify-center p-6 gap-4">
          <div className="w-16 h-16 rounded-full bg-cyan-100 dark:bg-cyan-900 flex items-center justify-center">
            <Mic className="h-8 w-8 text-cyan-600" />
          </div>
          <div className="text-center">
            <h3 className="font-semibold">Meeting Mode</h3>
            <p className="text-sm text-muted-foreground mt-1">
              Topsi will listen silently and take notes.
              <br />
              Say "Topsi" to get its attention.
            </p>
          </div>
          <input
            type="text"
            placeholder="Meeting title (optional)"
            value={meetingTitle}
            onChange={(e) => setMeetingTitle(e.target.value)}
            className="w-full max-w-xs px-3 py-2 border rounded-md text-sm"
          />
          <Button
            onClick={startMeeting}
            disabled={isProcessing}
            className="bg-cyan-600 hover:bg-cyan-700"
          >
            {isProcessing ? (
              <Loader2 className="h-4 w-4 animate-spin mr-2" />
            ) : (
              <Mic className="h-4 w-4 mr-2" />
            )}
            Start Meeting
          </Button>
        </div>
      )}

      {/* Active/Paused State - Live Transcript */}
      {(meetingState === 'active' || meetingState === 'paused') && (
        <>
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

          {/* Transcript Area */}
          <ScrollArea className="flex-1 p-3">
            <div className="space-y-2">
              {transcript.length === 0 && (
                <div className="text-center text-sm text-muted-foreground py-8">
                  {meetingState === 'active'
                    ? 'Listening... Speak to see the transcript.'
                    : 'Recording paused.'}
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
                    <span className="text-xs font-semibold text-muted-foreground">
                      {entry.speakerLabel}:{' '}
                    </span>
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

          {/* Meeting Controls */}
          <div className="p-3 border-t flex items-center justify-center gap-3">
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
        </>
      )}

      {/* Ended State - Notes */}
      {meetingState === 'ended' && (
        <ScrollArea className="flex-1 p-3">
          <div className="space-y-4">
            {isProcessing ? (
              <div className="flex flex-col items-center justify-center py-8 gap-3">
                <Loader2 className="h-8 w-8 animate-spin text-cyan-600" />
                <p className="text-sm text-muted-foreground">Generating meeting notes...</p>
              </div>
            ) : notes ? (
              <>
                <div className="flex items-center gap-2 mb-2">
                  <FileText className="h-4 w-4 text-cyan-600" />
                  <span className="font-semibold">Meeting Notes</span>
                  <Badge variant="secondary" className="text-xs">
                    {formatDuration(duration)}
                  </Badge>
                </div>

                <div>
                  <h4 className="text-xs font-semibold text-muted-foreground mb-1">Summary</h4>
                  <p className="text-sm">{notes.summary}</p>
                </div>

                {notes.topics.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Topics</h4>
                    <ul className="text-sm list-disc list-inside space-y-0.5">
                      {notes.topics.map((t, i) => (
                        <li key={i}>{t}</li>
                      ))}
                    </ul>
                  </div>
                )}

                {notes.decisions.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">Decisions</h4>
                    <ul className="text-sm list-disc list-inside space-y-0.5">
                      {notes.decisions.map((d, i) => (
                        <li key={i}>{d}</li>
                      ))}
                    </ul>
                  </div>
                )}

                {notes.actionItems.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">
                      Action Items
                    </h4>
                    <ul className="text-sm space-y-1">
                      {notes.actionItems.map((item, i) => (
                        <li key={i} className="flex items-start gap-2">
                          <span className="text-cyan-600 mt-0.5">-</span>
                          <div>
                            <span>{item.description}</span>
                            {item.assignee && (
                              <Badge variant="outline" className="ml-1 text-xs">
                                {item.assignee}
                              </Badge>
                            )}
                            {item.deadline && (
                              <span className="text-xs text-muted-foreground ml-1">
                                (by {item.deadline})
                              </span>
                            )}
                          </div>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}

                {notes.openQuestions.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">
                      Open Questions
                    </h4>
                    <ul className="text-sm list-disc list-inside space-y-0.5">
                      {notes.openQuestions.map((q, i) => (
                        <li key={i}>{q}</li>
                      ))}
                    </ul>
                  </div>
                )}

                {notes.participants.length > 0 && (
                  <div>
                    <h4 className="text-xs font-semibold text-muted-foreground mb-1">
                      Participants
                    </h4>
                    <div className="flex flex-wrap gap-1">
                      {notes.participants.map((p, i) => (
                        <Badge key={i} variant="secondary" className="text-xs">
                          {p}
                        </Badge>
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
              <Button variant="outline" size="sm" onClick={resetMeeting}>
                New Meeting
              </Button>
              {onClose && (
                <Button variant="ghost" size="sm" onClick={onClose}>
                  Close
                </Button>
              )}
            </div>
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

export default MeetingMode;
