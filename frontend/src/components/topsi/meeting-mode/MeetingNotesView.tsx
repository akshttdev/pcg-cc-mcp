// ── Meeting Notes View (Ended State) ────────────────────────────────────────
//
// Displays generated meeting notes after a meeting ends, with publish action.

import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import {
  FileText,
  Loader2,
  Network,
  RefreshCw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { MeetingNotes, MeetingRole } from './types';
import { formatDuration } from './types';

interface MeetingNotesViewProps {
  notes: MeetingNotes | null;
  duration: number;
  transcriptLength: number;
  meetingRole: MeetingRole;
  isProcessing: boolean;

  // Knowledge graph publish
  isPublishing: boolean;
  publishedKgId: string | null;
  onPublishToKnowledgeGraph: () => void;

  // Actions
  onResetMeeting: () => void;
  onClose?: () => void;
}

export function MeetingNotesView({
  notes,
  duration,
  transcriptLength,
  meetingRole,
  isProcessing,
  isPublishing,
  publishedKgId,
  onPublishToKnowledgeGraph,
  onResetMeeting,
  onClose,
}: MeetingNotesViewProps) {
  return (
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
              {meetingRole === 'observer' ? 'You left the meeting.' : `Meeting ended. ${transcriptLength} segments recorded.`}
            </p>
          </div>
        )}

        <div className="flex flex-wrap items-center justify-center gap-2 pt-2">
          {notes && !publishedKgId && (
            <Button
              variant="outline"
              size="sm"
              onClick={onPublishToKnowledgeGraph}
              disabled={isPublishing}
            >
              {isPublishing ? (
                <><RefreshCw className="h-3.5 w-3.5 mr-1 animate-spin" />Publishing…</>
              ) : (
                <><Network className="h-3.5 w-3.5 mr-1" />Publish to Knowledge Graph</>
              )}
            </Button>
          )}
          {publishedKgId && (
            <span className="text-xs text-green-600 flex items-center gap-1">
              <Network className="h-3.5 w-3.5" />
              Published to KG
            </span>
          )}
          <Button variant="outline" size="sm" onClick={onResetMeeting}>New Meeting</Button>
          {onClose && <Button variant="ghost" size="sm" onClick={onClose}>Close</Button>}
        </div>
      </div>
    </ScrollArea>
  );
}
