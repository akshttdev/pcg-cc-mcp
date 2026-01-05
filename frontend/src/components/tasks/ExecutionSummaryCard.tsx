import { Card } from '../ui/card';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { Textarea } from '../ui/textarea';
import { FileEdit, FilePlus, FileX, Clock, Star, AlertTriangle, CheckCircle, XCircle, CircleDashed, MessageSquare, Bookmark } from 'lucide-react';
import type { ExecutionSummary, CompletionStatus } from 'shared/types';
import { cn } from '@/lib/utils';
import { useMemo, useState, useCallback } from 'react';
import { useUpdateExecutionSummaryFeedback } from '@/hooks';

interface ExecutionSummaryCardProps {
  summary: ExecutionSummary | null;
  className?: string;
}

const completionStatusConfig: Record<CompletionStatus, {
  label: string;
  variant: 'default' | 'secondary' | 'destructive' | 'outline';
  icon: React.ComponentType<{ className?: string }>;
}> = {
  full: { label: 'Completed', variant: 'default', icon: CheckCircle },
  partial: { label: 'Partial', variant: 'secondary', icon: CircleDashed },
  blocked: { label: 'Blocked', variant: 'outline', icon: AlertTriangle },
  failed: { label: 'Failed', variant: 'destructive', icon: XCircle },
};

function formatDuration(ms: number | bigint): string {
  const milliseconds = typeof ms === 'bigint' ? Number(ms) : ms;
  if (milliseconds < 1000) return `${milliseconds}ms`;
  if (milliseconds < 60000) return `${(milliseconds / 1000).toFixed(1)}s`;
  const minutes = Math.floor(milliseconds / 60000);
  const seconds = Math.round((milliseconds % 60000) / 1000);
  return `${minutes}m ${seconds}s`;
}

interface InteractiveStarRatingProps {
  rating: number | null;
  onRate?: (rating: number) => void;
  disabled?: boolean;
}

function InteractiveStarRating({ rating, onRate, disabled }: InteractiveStarRatingProps) {
  const [hoverRating, setHoverRating] = useState<number | null>(null);
  const displayRating = hoverRating ?? rating ?? 0;

  return (
    <div className="flex items-center gap-0.5">
      {[1, 2, 3, 4, 5].map((star) => (
        <button
          key={star}
          type="button"
          disabled={disabled}
          className={cn(
            'p-0.5 rounded transition-colors',
            !disabled && 'hover:bg-muted cursor-pointer',
            disabled && 'cursor-default'
          )}
          onMouseEnter={() => !disabled && setHoverRating(star)}
          onMouseLeave={() => setHoverRating(null)}
          onClick={() => onRate?.(star)}
        >
          <Star
            className={cn(
              'h-4 w-4 transition-colors',
              star <= displayRating
                ? 'fill-yellow-400 text-yellow-400'
                : 'text-muted-foreground'
            )}
          />
        </button>
      ))}
    </div>
  );
}

export function ExecutionSummaryCard({ summary, className }: ExecutionSummaryCardProps) {
  const [showNotesInput, setShowNotesInput] = useState(false);
  const [notesValue, setNotesValue] = useState(summary?.human_notes ?? '');
  const updateFeedback = useUpdateExecutionSummaryFeedback(summary?.id);

  const handleRate = useCallback((rating: number) => {
    if (!summary) return;
    updateFeedback.mutate({ human_rating: rating });
  }, [summary, updateFeedback]);

  const handleSaveNotes = useCallback(() => {
    if (!summary) return;
    updateFeedback.mutate({ human_notes: notesValue || null });
    setShowNotesInput(false);
  }, [summary, notesValue, updateFeedback]);

  const handleToggleReference = useCallback(() => {
    if (!summary) return;
    updateFeedback.mutate({ is_reference_example: !summary.is_reference_example });
  }, [summary, updateFeedback]);

  if (!summary) {
    return null;
  }

  const statusConfig = completionStatusConfig[summary.completion_status];
  const StatusIcon = statusConfig.icon;

  const toolsList = useMemo(() => {
    if (!summary.tools_used) return [];
    try {
      const parsed = JSON.parse(summary.tools_used);
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  }, [summary.tools_used]);

  return (
    <Card className={cn('p-4 space-y-3', className)}>
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-foreground">Execution Summary</h3>
        <Badge variant={statusConfig.variant} className="flex items-center gap-1">
          <StatusIcon className="h-3 w-3" />
          {statusConfig.label}
        </Badge>
      </div>

      {/* File changes */}
      <div className="grid grid-cols-3 gap-2 text-xs">
        <div className="flex items-center gap-1.5 text-muted-foreground">
          <FileEdit className="h-3.5 w-3.5 text-blue-500" />
          <span>{summary.files_modified} modified</span>
        </div>
        <div className="flex items-center gap-1.5 text-muted-foreground">
          <FilePlus className="h-3.5 w-3.5 text-green-500" />
          <span>{summary.files_created} created</span>
        </div>
        <div className="flex items-center gap-1.5 text-muted-foreground">
          <FileX className="h-3.5 w-3.5 text-red-500" />
          <span>{summary.files_deleted} deleted</span>
        </div>
      </div>

      {/* Timing */}
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Clock className="h-3.5 w-3.5" />
        <span>{formatDuration(summary.execution_time_ms)}</span>
      </div>

      {/* Commands failed warning */}
      {summary.commands_failed > 0 && (
        <div className="flex items-center gap-1.5 text-xs text-yellow-600 dark:text-yellow-500">
          <AlertTriangle className="h-3.5 w-3.5" />
          <span>{summary.commands_failed} command{summary.commands_failed !== 1 ? 's' : ''} failed</span>
        </div>
      )}

      {/* Tools used */}
      {toolsList.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {toolsList.slice(0, 5).map((tool) => (
            <Badge key={tool} variant="outline" className="text-xs">
              {tool}
            </Badge>
          ))}
          {toolsList.length > 5 && (
            <Badge variant="outline" className="text-xs">
              +{toolsList.length - 5} more
            </Badge>
          )}
        </div>
      )}

      {/* Error or blocker summary */}
      {(summary.error_summary || summary.blocker_summary) && (
        <div className="text-xs text-muted-foreground bg-muted/50 rounded p-2">
          {summary.error_summary || summary.blocker_summary}
        </div>
      )}

      {/* Human Feedback Section */}
      <div className="border-t pt-3 space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Rate this execution:</span>
          <InteractiveStarRating
            rating={summary.human_rating}
            onRate={handleRate}
            disabled={updateFeedback.isPending}
          />
        </div>

        {/* Reference example toggle */}
        <div className="flex items-center justify-between">
          <span className="text-xs text-muted-foreground">Reference example:</span>
          <Button
            variant={summary.is_reference_example ? 'default' : 'ghost'}
            size="sm"
            className="h-6 px-2"
            onClick={handleToggleReference}
            disabled={updateFeedback.isPending}
          >
            <Bookmark className={cn(
              'h-3 w-3 mr-1',
              summary.is_reference_example && 'fill-current'
            )} />
            {summary.is_reference_example ? 'Saved' : 'Save'}
          </Button>
        </div>

        {/* Notes section */}
        {showNotesInput ? (
          <div className="space-y-2">
            <Textarea
              placeholder="Add notes about this execution..."
              value={notesValue}
              onChange={(e) => setNotesValue(e.target.value)}
              className="text-xs min-h-[60px]"
            />
            <div className="flex gap-2">
              <Button
                size="sm"
                className="h-6 text-xs"
                onClick={handleSaveNotes}
                disabled={updateFeedback.isPending}
              >
                Save
              </Button>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 text-xs"
                onClick={() => {
                  setShowNotesInput(false);
                  setNotesValue(summary.human_notes ?? '');
                }}
              >
                Cancel
              </Button>
            </div>
          </div>
        ) : summary.human_notes ? (
          <div
            className="text-xs text-muted-foreground border-l-2 border-primary pl-2 cursor-pointer hover:bg-muted/50 rounded p-1"
            onClick={() => setShowNotesInput(true)}
          >
            {summary.human_notes}
          </div>
        ) : (
          <Button
            variant="ghost"
            size="sm"
            className="h-6 text-xs w-full justify-start"
            onClick={() => setShowNotesInput(true)}
          >
            <MessageSquare className="h-3 w-3 mr-1" />
            Add notes
          </Button>
        )}
      </div>
    </Card>
  );
}

// Compact version for task cards
export function ExecutionSummaryBadge({
  filesModified,
  filesCreated,
  completionStatus,
  commandsFailed,
}: {
  filesModified: number;
  filesCreated: number;
  completionStatus: CompletionStatus;
  commandsFailed: number;
}) {
  const totalChanges = filesModified + filesCreated;
  const statusConfig = completionStatusConfig[completionStatus];
  const StatusIcon = statusConfig.icon;

  return (
    <div className="flex items-center gap-2 text-xs">
      <div className="flex items-center gap-1 text-muted-foreground">
        <FileEdit className="h-3 w-3" />
        <span>{totalChanges}</span>
      </div>
      {commandsFailed > 0 && (
        <div className="flex items-center gap-1 text-yellow-600">
          <AlertTriangle className="h-3 w-3" />
          <span>{commandsFailed}</span>
        </div>
      )}
      <StatusIcon className={cn(
        'h-3 w-3',
        completionStatus === 'full' && 'text-green-500',
        completionStatus === 'partial' && 'text-yellow-500',
        completionStatus === 'blocked' && 'text-orange-500',
        completionStatus === 'failed' && 'text-red-500',
      )} />
    </div>
  );
}
