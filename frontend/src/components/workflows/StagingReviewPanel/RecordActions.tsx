import { useCallback } from 'react';
import { Check, X, Edit3, RotateCcw } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { WorkflowStagingRecord } from '@/lib/api';

interface RecordActionsProps {
  record: WorkflowStagingRecord;
  onEdit: (record: WorkflowStagingRecord) => void;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
  onRetry: (id: string) => void;
  retryPending: boolean;
  onStopPropagation: (e: React.MouseEvent) => void;
}

export function RecordActions({
  record,
  onEdit,
  onApprove,
  onReject,
  onRetry,
  retryPending,
  onStopPropagation,
}: RecordActionsProps) {
  const handleEdit = useCallback(() => onEdit(record), [onEdit, record]);
  const handleApprove = useCallback(() => onApprove(record.id), [onApprove, record.id]);
  const handleReject = useCallback(() => onReject(record.id), [onReject, record.id]);
  const handleRetry = useCallback(() => onRetry(record.id), [onRetry, record.id]);

  return (
    <div className="flex items-center gap-0.5 justify-end" onClick={onStopPropagation}>
      {record.status === 'pending_review' && (
        <>
          <button onClick={handleEdit} className="p-1 rounded hover:bg-muted" title="Edit">
            <Edit3 className="h-3 w-3 text-muted-foreground" />
          </button>
          <button onClick={handleApprove} className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900" title="Approve & Commit">
            <Check className="h-3 w-3 text-green-600" />
          </button>
          <button onClick={handleReject} className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900" title="Reject">
            <X className="h-3 w-3 text-red-500" />
          </button>
        </>
      )}
      {record.status === 'error' && (
        <>
          <button onClick={handleEdit} className="p-1 rounded hover:bg-muted" title="Edit">
            <Edit3 className="h-3 w-3 text-muted-foreground" />
          </button>
          <button onClick={handleRetry} className="p-1 rounded hover:bg-amber-100 dark:hover:bg-amber-900" title="Retry" disabled={retryPending}>
            <RotateCcw className={cn('h-3 w-3 text-amber-600', retryPending && 'animate-spin')} />
          </button>
        </>
      )}
    </div>
  );
}
