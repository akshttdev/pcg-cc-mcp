import { useState, useEffect, useRef, useCallback } from 'react';
import { resolveApiUrl } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Clock,
  Users,
  FileText,
  ChevronDown,
  ChevronRight,
  Share2,
  Loader2,
  Download,
  Play,
  Pause,
  SkipBack,
  Sparkles,
  MessageSquare,
  CheckCircle2,
  ListTodo,
  HelpCircle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

interface MeetingSession {
  id: string;
  projectId: string;
  projectName?: string;
  title: string;
  status: string;
  startedBy: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds?: number;
  participantCount?: number;
  segmentCount?: number;
  notes?: Record<string, unknown>;
}

interface MeetingSegment {
  id: string;
  meetingSessionId: string;
  segmentIndex: number;
  speakerLabel?: string;
  text: string;
  confidence?: number;
  startTimeMs: number;
  endTimeMs: number;
  isTopsiAddressed: boolean;
}

interface MeetingNotes {
  summary: string;
  topics: string[];
  decisions: string[];
  actionItems: { description: string; assignee?: string; deadline?: string; priority?: string }[];
  openQuestions: string[];
  participants: string[];
}

interface MeetingHistoryProps {
  projectId?: string;
  className?: string;
}

export function MeetingHistory({ projectId, className }: MeetingHistoryProps) {
  const [meetings, setMeetings] = useState<MeetingSession[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [transcriptMap, setTranscriptMap] = useState<Record<string, MeetingSegment[]>>({});
  const [notesMap, setNotesMap] = useState<Record<string, MeetingNotes>>({});
  const [loadingDetails, setLoadingDetails] = useState<Record<string, boolean>>({});
  const [generatingSummary, setGeneratingSummary] = useState<Record<string, boolean>>({});
  const [shareUserId, setShareUserId] = useState('');
  const [playingId, setPlayingId] = useState<string | null>(null);
  const [playSegmentIdx, setPlaySegmentIdx] = useState(0);
  const speechRef = useRef<SpeechSynthesisUtterance | null>(null);

  const getAuthHeaders = useCallback((): Record<string, string> => {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    const token =
      localStorage.getItem('session_id') || sessionStorage.getItem('session_id') ||
      localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    return headers;
  }, []);

  const fetchMeetings = useCallback(async () => {
    setIsLoading(true);
    try {
      const params = new URLSearchParams();
      if (projectId) params.set('projectId', projectId);
      const url = resolveApiUrl(`/api/topsi/meeting/list${params.toString() ? '?' + params.toString() : ''}`);
      const res = await fetch(url, { headers: getAuthHeaders(), credentials: 'include' });
      if (res.ok) {
        const data = await res.json();
        setMeetings(data.meetings || []);
      }
    } catch {
      // silent
    } finally {
      setIsLoading(false);
    }
  }, [projectId, getAuthHeaders]);

  useEffect(() => {
    fetchMeetings();
  }, [fetchMeetings]);

  const loadDetails = useCallback(async (meetingId: string) => {
    if (transcriptMap[meetingId] !== undefined) return; // already loaded
    setLoadingDetails(prev => ({ ...prev, [meetingId]: true }));
    try {
      const [transcriptRes, notesRes] = await Promise.all([
        fetch(resolveApiUrl(`/api/topsi/meeting/transcript/${meetingId}`), {
          headers: getAuthHeaders(), credentials: 'include',
        }),
        fetch(resolveApiUrl(`/api/topsi/meeting/notes/${meetingId}`), {
          headers: getAuthHeaders(), credentials: 'include',
        }),
      ]);
      if (transcriptRes.ok) {
        const d = await transcriptRes.json();
        setTranscriptMap(prev => ({ ...prev, [meetingId]: d.segments || [] }));
      } else {
        setTranscriptMap(prev => ({ ...prev, [meetingId]: [] }));
      }
      if (notesRes.ok) {
        const d = await notesRes.json();
        if (d.notes) setNotesMap(prev => ({ ...prev, [meetingId]: d.notes }));
      }
    } catch {
      setTranscriptMap(prev => ({ ...prev, [meetingId]: [] }));
    } finally {
      setLoadingDetails(prev => ({ ...prev, [meetingId]: false }));
    }
  }, [transcriptMap, getAuthHeaders]);

  const toggleExpand = (meetingId: string) => {
    if (expandedId === meetingId) {
      setExpandedId(null);
      stopPlayback();
    } else {
      setExpandedId(meetingId);
      loadDetails(meetingId);
    }
  };

  const generateSummary = async (meetingId: string) => {
    setGeneratingSummary(prev => ({ ...prev, [meetingId]: true }));
    try {
      const res = await fetch(resolveApiUrl(`/api/topsi/meeting/notes/${meetingId}`), {
        method: 'POST',
        headers: getAuthHeaders(),
        credentials: 'include',
      });
      if (res.ok) {
        const d = await res.json();
        if (d.notes) {
          setNotesMap(prev => ({ ...prev, [meetingId]: d.notes }));
          toast.success('Summary generated');
        }
      } else {
        toast.error('Failed to generate summary');
      }
    } catch {
      toast.error('Failed to generate summary');
    } finally {
      setGeneratingSummary(prev => ({ ...prev, [meetingId]: false }));
    }
  };

  // ── Playback via Web Speech API ───────────────────────────────────────────
  const stopPlayback = useCallback(() => {
    window.speechSynthesis?.cancel();
    setPlayingId(null);
    setPlaySegmentIdx(0);
    speechRef.current = null;
  }, []);

  const startPlayback = useCallback((meetingId: string, startIdx = 0) => {
    const segs = transcriptMap[meetingId] || [];
    if (!segs.length || !window.speechSynthesis) {
      toast.error('No transcript to play or speech not supported');
      return;
    }
    window.speechSynthesis.cancel();

    let idx = startIdx;
    const playNext = () => {
      if (idx >= segs.length) {
        setPlayingId(null);
        setPlaySegmentIdx(0);
        return;
      }
      const seg = segs[idx];
      const utter = new SpeechSynthesisUtterance(
        `${seg.speakerLabel ? seg.speakerLabel + ': ' : ''}${seg.text}`
      );
      utter.rate = 1.1;
      if (seg.speakerLabel?.toLowerCase() === 'nora') {
        // slightly different voice for Nora
        const voices = window.speechSynthesis.getVoices();
        const femaleVoice = voices.find(v => v.name.toLowerCase().includes('female') || v.name.toLowerCase().includes('samantha') || v.name.toLowerCase().includes('karen'));
        if (femaleVoice) utter.voice = femaleVoice;
        utter.pitch = 1.1;
      }
      utter.onend = () => { idx++; setPlaySegmentIdx(idx); playNext(); };
      utter.onerror = () => { setPlayingId(null); };
      speechRef.current = utter;
      window.speechSynthesis.speak(utter);
    };

    setPlayingId(meetingId);
    setPlaySegmentIdx(startIdx);
    playNext();
  }, [transcriptMap]);

  // ── Download transcript ───────────────────────────────────────────────────
  const downloadTranscript = useCallback((meeting: MeetingSession) => {
    const segs = transcriptMap[meeting.id] || [];
    const notes = notesMap[meeting.id];
    const lines: string[] = [
      `Meeting: ${meeting.title}`,
      `Date: ${new Date(meeting.startedAt).toLocaleString()}`,
      meeting.endedAt ? `Ended: ${new Date(meeting.endedAt).toLocaleString()}` : '',
      `Project: ${meeting.projectName || meeting.projectId}`,
      `Started by: ${meeting.startedBy}`,
      `Segments: ${segs.length}`,
      '',
    ];

    if (notes?.summary) {
      lines.push('── SUMMARY ─────────────────────────────────────');
      lines.push(notes.summary);
      lines.push('');
    }

    if (notes?.actionItems?.length) {
      lines.push('── ACTION ITEMS ─────────────────────────────────');
      notes.actionItems.forEach(item => {
        lines.push(`• ${item.description}${item.assignee ? ` [${item.assignee}]` : ''}`);
      });
      lines.push('');
    }

    if (segs.length) {
      lines.push('── TRANSCRIPT ───────────────────────────────────');
      segs.forEach(seg => {
        const label = seg.speakerLabel ? `[${seg.speakerLabel}]` : '[Speaker]';
        lines.push(`${label} ${seg.text}`);
      });
    }

    const blob = new Blob([lines.filter(l => l !== undefined).join('\n')], { type: 'text/plain' });
    const a = document.createElement('a');
    a.href = URL.createObjectURL(blob);
    a.download = `${meeting.title.replace(/[^a-z0-9]/gi, '_')}_${meeting.id.slice(0, 8)}.txt`;
    a.click();
    URL.revokeObjectURL(a.href);
  }, [transcriptMap, notesMap]);

  // ── Share ────────────────────────────────────────────────────────────────
  const shareMeeting = async (meetingId: string) => {
    if (!shareUserId.trim()) return;
    try {
      const res = await fetch(resolveApiUrl(`/api/topsi/meeting/share/${meetingId}`), {
        method: 'POST',
        headers: getAuthHeaders(),
        credentials: 'include',
        body: JSON.stringify({ userIds: [shareUserId.trim()] }),
      });
      if (res.ok) {
        toast.success('Meeting shared');
        setShareUserId('');
      } else {
        toast.error('Failed to share meeting');
      }
    } catch {
      toast.error('Failed to share meeting');
    }
  };

  // ── Helpers ──────────────────────────────────────────────────────────────
  const formatDuration = (seconds?: number) => {
    if (!seconds) return null;
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  };

  const formatDate = (dateStr: string) => {
    try {
      return new Date(dateStr).toLocaleDateString(undefined, {
        month: 'short', day: 'numeric', year: 'numeric',
        hour: '2-digit', minute: '2-digit',
      });
    } catch { return dateStr; }
  };

  const isNora = (m: MeetingSession) => m.startedBy === 'nora';

  // ── Render ───────────────────────────────────────────────────────────────
  return (
    <div className={cn('flex flex-col gap-2', className)}>
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-sm">Call Logs</h3>
        <Button variant="outline" size="sm" onClick={fetchMeetings} disabled={isLoading} className="h-7 text-xs">
          {isLoading ? <Loader2 className="h-3 w-3 animate-spin" /> : 'Refresh'}
        </Button>
      </div>

      {meetings.length === 0 ? (
        <div className="text-center py-8 text-sm text-muted-foreground">
          {isLoading ? 'Loading...' : 'No call logs found.'}
        </div>
      ) : (
        <ScrollArea className="flex-1">
          <div className="space-y-2 pr-1">
            {meetings.map((meeting) => {
              const isExpanded = expandedId === meeting.id;
              const segs = transcriptMap[meeting.id] || [];
              const notes = notesMap[meeting.id];
              const isLoadingThis = loadingDetails[meeting.id];
              const isPlaying = playingId === meeting.id;
              const hasTranscript = (meeting.segmentCount ?? 0) > 0;
              const dur = formatDuration(meeting.durationSeconds);

              return (
                <div key={meeting.id} className="border rounded-lg overflow-hidden">
                  {/* Header row */}
                  <button
                    className="w-full flex items-start justify-between p-3 hover:bg-muted/40 transition-colors text-left"
                    onClick={() => toggleExpand(meeting.id)}
                  >
                    <div className="flex items-start gap-2 min-w-0">
                      <span className="mt-0.5 shrink-0">
                        {isExpanded
                          ? <ChevronDown className="h-4 w-4 text-muted-foreground" />
                          : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
                      </span>
                      <div className="min-w-0">
                        <div className="text-sm font-medium truncate">{meeting.title}</div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          {formatDate(meeting.startedAt)}
                          {meeting.projectName && (
                            <span className="text-muted-foreground/60"> · {meeting.projectName}</span>
                          )}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-1.5 shrink-0 ml-2 mt-0.5 flex-wrap justify-end">
                      {dur && (
                        <span className="flex items-center gap-0.5 text-[11px] text-muted-foreground">
                          <Clock className="h-3 w-3" />{dur}
                        </span>
                      )}
                      {hasTranscript && (
                        <span className="flex items-center gap-0.5 text-[11px] text-muted-foreground">
                          <MessageSquare className="h-3 w-3" />{meeting.segmentCount}
                        </span>
                      )}
                      {(meeting.participantCount ?? 0) > 0 && (
                        <span className="flex items-center gap-0.5 text-[11px] text-muted-foreground">
                          <Users className="h-3 w-3" />{meeting.participantCount}
                        </span>
                      )}
                      <Badge
                        variant={meeting.status === 'ended' ? 'secondary' : 'default'}
                        className="text-[10px] h-4 px-1"
                      >
                        {isNora(meeting) ? '🤖 ' : ''}{meeting.status}
                      </Badge>
                    </div>
                  </button>

                  {/* Expanded panel */}
                  {isExpanded && (
                    <div className="border-t bg-muted/20">
                      {isLoadingThis ? (
                        <div className="flex items-center justify-center py-6">
                          <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                        </div>
                      ) : (
                        <Tabs defaultValue={notes ? 'summary' : (hasTranscript ? 'transcript' : 'share')} className="w-full">
                          {/* Tab bar + action buttons */}
                          <div className="flex items-center justify-between px-3 pt-2 pb-0 border-b">
                            <TabsList className="h-7 bg-transparent gap-0 p-0">
                              <TabsTrigger value="summary" className="h-7 text-xs px-3 rounded-none border-b-2 border-transparent data-[state=active]:border-foreground data-[state=active]:bg-transparent">
                                Summary
                              </TabsTrigger>
                              <TabsTrigger value="transcript" className="h-7 text-xs px-3 rounded-none border-b-2 border-transparent data-[state=active]:border-foreground data-[state=active]:bg-transparent">
                                Transcript {segs.length > 0 ? `(${segs.length})` : ''}
                              </TabsTrigger>
                              <TabsTrigger value="share" className="h-7 text-xs px-3 rounded-none border-b-2 border-transparent data-[state=active]:border-foreground data-[state=active]:bg-transparent">
                                Share
                              </TabsTrigger>
                            </TabsList>
                            {/* Global action buttons */}
                            <div className="flex items-center gap-1">
                              {hasTranscript && (
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="h-6 px-2 text-xs"
                                  title="Download transcript"
                                  onClick={(e) => { e.stopPropagation(); downloadTranscript(meeting); }}
                                >
                                  <Download className="h-3 w-3" />
                                </Button>
                              )}
                              {hasTranscript && (
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className={cn('h-6 px-2 text-xs', isPlaying && 'text-cyan-600')}
                                  title={isPlaying ? 'Stop playback' : 'Play transcript aloud'}
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    if (isPlaying) stopPlayback();
                                    else startPlayback(meeting.id);
                                  }}
                                >
                                  {isPlaying ? <Pause className="h-3 w-3" /> : <Play className="h-3 w-3" />}
                                </Button>
                              )}
                            </div>
                          </div>

                          {/* Summary tab */}
                          <TabsContent value="summary" className="p-3 mt-0 space-y-3">
                            {notes ? (
                              <>
                                {notes.summary && (
                                  <div className="space-y-1">
                                    <p className="text-xs text-muted-foreground font-semibold uppercase tracking-wide">Overview</p>
                                    <p className="text-sm leading-relaxed">{notes.summary}</p>
                                  </div>
                                )}
                                {notes.topics?.length > 0 && (
                                  <div className="space-y-1">
                                    <p className="text-xs text-muted-foreground font-semibold uppercase tracking-wide">Topics</p>
                                    <div className="flex flex-wrap gap-1">
                                      {notes.topics.map((t, i) => (
                                        <Badge key={i} variant="outline" className="text-xs">{t}</Badge>
                                      ))}
                                    </div>
                                  </div>
                                )}
                                {notes.decisions?.length > 0 && (
                                  <div className="space-y-1">
                                    <p className="text-xs text-muted-foreground font-semibold uppercase tracking-wide flex items-center gap-1">
                                      <CheckCircle2 className="h-3 w-3" />Decisions
                                    </p>
                                    <ul className="space-y-0.5">
                                      {notes.decisions.map((d, i) => (
                                        <li key={i} className="text-xs flex gap-1.5"><span className="text-green-500 shrink-0">✓</span>{d}</li>
                                      ))}
                                    </ul>
                                  </div>
                                )}
                                {notes.actionItems?.length > 0 && (
                                  <div className="space-y-1">
                                    <p className="text-xs text-muted-foreground font-semibold uppercase tracking-wide flex items-center gap-1">
                                      <ListTodo className="h-3 w-3" />Action Items
                                    </p>
                                    <ul className="space-y-1">
                                      {notes.actionItems.map((item, i) => (
                                        <li key={i} className="text-xs flex items-start gap-1.5">
                                          <span className="text-cyan-500 shrink-0">→</span>
                                          <span>{item.description}</span>
                                          {item.assignee && (
                                            <Badge variant="outline" className="text-[10px] ml-auto shrink-0">{item.assignee}</Badge>
                                          )}
                                        </li>
                                      ))}
                                    </ul>
                                  </div>
                                )}
                                {notes.openQuestions?.length > 0 && (
                                  <div className="space-y-1">
                                    <p className="text-xs text-muted-foreground font-semibold uppercase tracking-wide flex items-center gap-1">
                                      <HelpCircle className="h-3 w-3" />Open Questions
                                    </p>
                                    <ul className="space-y-0.5">
                                      {notes.openQuestions.map((q, i) => (
                                        <li key={i} className="text-xs flex gap-1.5"><span className="text-yellow-500 shrink-0">?</span>{q}</li>
                                      ))}
                                    </ul>
                                  </div>
                                )}
                                <Button
                                  variant="ghost"
                                  size="sm"
                                  className="h-6 text-xs text-muted-foreground w-full"
                                  onClick={() => generateSummary(meeting.id)}
                                  disabled={generatingSummary[meeting.id]}
                                >
                                  {generatingSummary[meeting.id]
                                    ? <><Loader2 className="h-3 w-3 mr-1 animate-spin" />Regenerating…</>
                                    : <><Sparkles className="h-3 w-3 mr-1" />Regenerate Summary</>}
                                </Button>
                              </>
                            ) : (
                              <div className="text-center py-4 space-y-3">
                                {hasTranscript ? (
                                  <>
                                    <p className="text-xs text-muted-foreground">No summary yet.</p>
                                    <Button
                                      size="sm"
                                      className="h-7 text-xs"
                                      onClick={() => generateSummary(meeting.id)}
                                      disabled={generatingSummary[meeting.id]}
                                    >
                                      {generatingSummary[meeting.id]
                                        ? <><Loader2 className="h-3 w-3 mr-1 animate-spin" />Generating…</>
                                        : <><Sparkles className="h-3 w-3 mr-1" />Generate Summary</>}
                                    </Button>
                                  </>
                                ) : (
                                  <p className="text-xs text-muted-foreground">No transcript captured — summary unavailable.</p>
                                )}
                              </div>
                            )}
                          </TabsContent>

                          {/* Transcript tab */}
                          <TabsContent value="transcript" className="mt-0">
                            {segs.length === 0 ? (
                              <div className="text-center py-6 text-xs text-muted-foreground px-3">
                                No transcript segments were captured for this session.
                              </div>
                            ) : (
                              <>
                                {/* Playback progress bar */}
                                {isPlaying && (
                                  <div className="px-3 pt-2 flex items-center gap-2">
                                    <span className="text-xs text-cyan-600 animate-pulse">▶ Playing…</span>
                                    <div className="flex-1 h-1 bg-muted rounded-full overflow-hidden">
                                      <div
                                        className="h-full bg-cyan-500 transition-all"
                                        style={{ width: `${(playSegmentIdx / segs.length) * 100}%` }}
                                      />
                                    </div>
                                    <span className="text-[10px] text-muted-foreground">{playSegmentIdx}/{segs.length}</span>
                                    <Button
                                      variant="ghost"
                                      size="sm"
                                      className="h-5 px-1"
                                      onClick={() => startPlayback(meeting.id, 0)}
                                    >
                                      <SkipBack className="h-3 w-3" />
                                    </Button>
                                  </div>
                                )}
                                <ScrollArea className="max-h-72">
                                  <div className="p-3 space-y-1">
                                    {segs.map((seg, i) => {
                                      const isCurrentSeg = isPlaying && i === playSegmentIdx;
                                      const isNoraSeg = seg.speakerLabel?.toLowerCase() === 'nora';
                                      return (
                                        <div
                                          key={seg.id}
                                          className={cn(
                                            'text-xs p-2 rounded-md transition-colors',
                                            isCurrentSeg && 'ring-1 ring-cyan-400 bg-cyan-50 dark:bg-cyan-950',
                                            !isCurrentSeg && isNoraSeg && 'bg-blue-50/60 dark:bg-blue-950/30',
                                            !isCurrentSeg && !isNoraSeg && 'bg-muted/50',
                                          )}
                                        >
                                          <span className={cn(
                                            'font-semibold mr-1',
                                            isNoraSeg ? 'text-blue-600 dark:text-blue-400' : 'text-muted-foreground'
                                          )}>
                                            {seg.speakerLabel || 'Speaker'}:
                                          </span>
                                          {seg.text}
                                        </div>
                                      );
                                    })}
                                  </div>
                                </ScrollArea>
                              </>
                            )}
                          </TabsContent>

                          {/* Share tab */}
                          <TabsContent value="share" className="p-3 mt-0 space-y-3">
                            <div className="space-y-1">
                              <p className="text-xs font-semibold text-muted-foreground">Share with a team member</p>
                              <p className="text-xs text-muted-foreground">Enter a user ID or username to grant access to this call log.</p>
                            </div>
                            <div className="flex items-center gap-2">
                              <Share2 className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                              <input
                                type="text"
                                placeholder="User ID or username"
                                value={shareUserId}
                                onChange={(e) => setShareUserId(e.target.value)}
                                onKeyDown={(e) => e.key === 'Enter' && shareMeeting(meeting.id)}
                                className="flex-1 text-xs px-2 py-1.5 border rounded bg-background"
                              />
                              <Button
                                size="sm"
                                className="h-7 text-xs"
                                onClick={() => shareMeeting(meeting.id)}
                                disabled={!shareUserId.trim()}
                              >
                                Share
                              </Button>
                            </div>
                            {hasTranscript && (
                              <div className="pt-2 border-t">
                                <p className="text-xs font-semibold text-muted-foreground mb-2">Export</p>
                                <Button
                                  variant="outline"
                                  size="sm"
                                  className="h-7 text-xs w-full"
                                  onClick={() => downloadTranscript(meeting)}
                                >
                                  <Download className="h-3 w-3 mr-1.5" />
                                  Download Transcript (.txt)
                                </Button>
                              </div>
                            )}
                            <div className="pt-1">
                              <p className="text-xs text-muted-foreground font-mono break-all select-all opacity-50">
                                ID: {meeting.id}
                              </p>
                            </div>
                          </TabsContent>
                        </Tabs>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

export default MeetingHistory;
