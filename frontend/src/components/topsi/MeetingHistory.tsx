import { useState, useEffect } from 'react';
import { resolveApiUrl } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Clock,
  Users,
  FileText,
  ChevronDown,
  ChevronRight,
  Share2,
  Loader2,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';

interface MeetingSession {
  id: string;
  projectId: string;
  title: string;
  status: string;
  startedBy: string;
  startedAt: string;
  endedAt?: string;
  durationSeconds?: number;
  participantCount?: number;
  notes?: string;
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
  const [expandedData, setExpandedData] = useState<{
    transcript?: MeetingSegment[];
    notes?: MeetingNotes;
  } | null>(null);
  const [isLoadingDetails, setIsLoadingDetails] = useState(false);
  const [shareUserId, setShareUserId] = useState('');

  const getAuthHeaders = (): Record<string, string> => {
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    const token =
      localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
    if (token) headers['Authorization'] = `Bearer ${token}`;
    return headers;
  };

  const fetchMeetings = async () => {
    setIsLoading(true);
    try {
      const params = new URLSearchParams();
      if (projectId) params.set('projectId', projectId);
      const url = resolveApiUrl(`/api/topsi/meeting/list${params.toString() ? '?' + params.toString() : ''}`);
      const res = await fetch(url, {
        headers: getAuthHeaders(),
        credentials: 'include',
      });
      if (res.ok) {
        const data = await res.json();
        setMeetings(data.meetings || []);
      }
    } catch {
      // Silently handle - meetings list might not be populated yet
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    fetchMeetings();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [projectId]);

  const toggleExpand = async (meetingId: string) => {
    if (expandedId === meetingId) {
      setExpandedId(null);
      setExpandedData(null);
      return;
    }

    setExpandedId(meetingId);
    setIsLoadingDetails(true);

    try {
      const [transcriptRes, notesRes] = await Promise.all([
        fetch(resolveApiUrl(`/api/topsi/meeting/transcript/${meetingId}`), {
          headers: getAuthHeaders(),
          credentials: 'include',
        }),
        fetch(resolveApiUrl(`/api/topsi/meeting/notes/${meetingId}`), {
          headers: getAuthHeaders(),
          credentials: 'include',
        }),
      ]);

      const data: { transcript?: MeetingSegment[]; notes?: MeetingNotes } = {};

      if (transcriptRes.ok) {
        const transcriptData = await transcriptRes.json();
        data.transcript = transcriptData.segments;
      }

      if (notesRes.ok) {
        const notesData = await notesRes.json();
        data.notes = notesData.notes;
      }

      setExpandedData(data);
    } catch {
      toast.error('Failed to load meeting details');
    } finally {
      setIsLoadingDetails(false);
    }
  };

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

  const formatDuration = (seconds?: number) => {
    if (!seconds) return '--';
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
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return dateStr;
    }
  };

  return (
    <div className={cn('flex flex-col', className)}>
      <div className="flex items-center justify-between mb-4">
        <h3 className="font-semibold">Meeting History</h3>
        <Button variant="outline" size="sm" onClick={fetchMeetings} disabled={isLoading}>
          {isLoading ? <Loader2 className="h-3 w-3 animate-spin" /> : 'Refresh'}
        </Button>
      </div>

      {meetings.length === 0 ? (
        <div className="text-center py-8 text-sm text-muted-foreground">
          {isLoading ? 'Loading meetings...' : 'No past meetings found.'}
        </div>
      ) : (
        <ScrollArea className="flex-1">
          <div className="space-y-2">
            {meetings.map((meeting) => (
              <div key={meeting.id} className="border rounded-lg">
                {/* Meeting Header */}
                <button
                  className="w-full flex items-center justify-between p-3 hover:bg-muted/50 transition-colors"
                  onClick={() => toggleExpand(meeting.id)}
                >
                  <div className="flex items-center gap-2">
                    {expandedId === meeting.id ? (
                      <ChevronDown className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <ChevronRight className="h-4 w-4 text-muted-foreground" />
                    )}
                    <div className="text-left">
                      <div className="text-sm font-medium">{meeting.title}</div>
                      <div className="text-xs text-muted-foreground">
                        {formatDate(meeting.startedAt)}
                      </div>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <Clock className="h-3 w-3" />
                      {formatDuration(meeting.durationSeconds)}
                    </div>
                    <div className="flex items-center gap-1 text-xs text-muted-foreground">
                      <Users className="h-3 w-3" />
                      {meeting.participantCount || 0}
                    </div>
                    <Badge
                      variant={meeting.status === 'ended' ? 'secondary' : 'default'}
                      className="text-xs"
                    >
                      {meeting.status}
                    </Badge>
                  </div>
                </button>

                {/* Expanded Details */}
                {expandedId === meeting.id && (
                  <div className="border-t p-3 space-y-3">
                    {isLoadingDetails ? (
                      <div className="flex items-center justify-center py-4">
                        <Loader2 className="h-4 w-4 animate-spin" />
                      </div>
                    ) : (
                      <>
                        {/* Notes */}
                        {expandedData?.notes && (
                          <div className="space-y-2">
                            <div className="flex items-center gap-1 text-xs font-semibold text-muted-foreground">
                              <FileText className="h-3 w-3" />
                              Notes
                            </div>
                            <p className="text-sm">{expandedData.notes.summary}</p>

                            {expandedData.notes.actionItems.length > 0 && (
                              <div>
                                <span className="text-xs font-semibold text-muted-foreground">
                                  Action Items:
                                </span>
                                <ul className="text-xs mt-1 space-y-0.5">
                                  {expandedData.notes.actionItems.map((item, i) => (
                                    <li key={i} className="flex items-start gap-1">
                                      <span className="text-cyan-600">-</span>
                                      <span>{item.description}</span>
                                      {item.assignee && (
                                        <Badge variant="outline" className="text-[10px]">
                                          {item.assignee}
                                        </Badge>
                                      )}
                                    </li>
                                  ))}
                                </ul>
                              </div>
                            )}
                          </div>
                        )}

                        {/* Transcript */}
                        {expandedData?.transcript && expandedData.transcript.length > 0 && (
                          <div className="space-y-1">
                            <span className="text-xs font-semibold text-muted-foreground">
                              Transcript ({expandedData.transcript.length} segments)
                            </span>
                            <ScrollArea className="max-h-48">
                              <div className="space-y-1">
                                {expandedData.transcript.map((seg) => (
                                  <div
                                    key={seg.id}
                                    className={cn(
                                      'text-xs p-1.5 rounded',
                                      seg.isTopsiAddressed ? 'bg-cyan-50 dark:bg-cyan-950' : 'bg-muted'
                                    )}
                                  >
                                    {seg.speakerLabel && (
                                      <span className="font-semibold text-muted-foreground">
                                        {seg.speakerLabel}:{' '}
                                      </span>
                                    )}
                                    {seg.text}
                                  </div>
                                ))}
                              </div>
                            </ScrollArea>
                          </div>
                        )}

                        {/* Share */}
                        <div className="flex items-center gap-2 pt-2 border-t">
                          <Share2 className="h-3 w-3 text-muted-foreground" />
                          <input
                            type="text"
                            placeholder="User ID to share with"
                            value={shareUserId}
                            onChange={(e) => setShareUserId(e.target.value)}
                            className="flex-1 text-xs px-2 py-1 border rounded"
                          />
                          <Button
                            variant="outline"
                            size="sm"
                            className="h-6 text-xs"
                            onClick={() => shareMeeting(meeting.id)}
                            disabled={!shareUserId.trim()}
                          >
                            Share
                          </Button>
                        </div>
                      </>
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}

export default MeetingHistory;
