// ─── RunAndReviewPanel ────────────────────────────────────────────────────────
//
// Inline panel shown after a workflow run completes. Composes a run summary
// header with the reusable StagingReviewContent table so users can review
// and commit staged records without navigating away.

import { Button } from '@/components/ui/button';
import { CheckCircle2, ExternalLink, RotateCcw, X } from 'lucide-react';
import { StagingReviewContent } from '@/components/workflows/StagingReviewPanel';
import { useNavigate } from 'react-router-dom';

interface RunAndReviewPanelProps {
  runId: string;
  stagedRecords: number;
  workflowName?: string;
  onRunAnother: () => void;
  onClose: () => void;
}

export function RunAndReviewPanel({
  runId,
  stagedRecords,
  workflowName,
  onRunAnother,
  onClose,
}: RunAndReviewPanelProps) {
  const navigate = useNavigate();

  return (
    <div className="border rounded-lg overflow-hidden bg-background">
      {/* Summary header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b bg-green-50/50 dark:bg-green-950/10">
        <CheckCircle2 className="h-4 w-4 text-green-600 shrink-0" />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium">
            Workflow completed
            {workflowName && <span className="text-muted-foreground font-normal"> — {workflowName}</span>}
          </p>
          <p className="text-xs text-muted-foreground">
            {stagedRecords} record{stagedRecords !== 1 ? 's' : ''} staged for review
          </p>
        </div>
        <div className="flex items-center gap-1.5 shrink-0">
          <Button
            size="sm"
            variant="outline"
            className="h-7 text-xs gap-1.5"
            onClick={onRunAnother}
          >
            <RotateCcw className="h-3 w-3" />
            Run Another
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 text-xs gap-1.5 text-muted-foreground"
            onClick={() => navigate('/workflows?tab=staging&run=' + runId)}
          >
            <ExternalLink className="h-3 w-3" />
            Full View
          </Button>
          <Button
            size="sm"
            variant="ghost"
            className="h-7 w-7 p-0 text-muted-foreground"
            onClick={onClose}
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Inline staging review */}
      <div className="h-[500px]">
        <StagingReviewContent
          workflowRunId={runId}
          workflowName={workflowName}
          alwaysEnabled
        />
      </div>
    </div>
  );
}
