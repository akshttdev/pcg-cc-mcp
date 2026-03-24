import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  AlertCircle, CheckCircle,
  RotateCcw, ThumbsUp,
} from 'lucide-react';
import type { BusinessReportRecord } from '@/lib/api';

interface ReviewBannerProps {
  report: BusinessReportRecord;
  onApprove: () => void;
  onRequestRevision: (notes: string) => void;
  isLoading: boolean;
}

export function ReviewBanner({
  report,
  onApprove,
  onRequestRevision,
  isLoading,
}: ReviewBannerProps) {
  const [showRevisionDialog, setShowRevisionDialog] = useState(false);
  const [revisionNotes, setRevisionNotes] = useState('');

  if (report.review_status === 'approved') {
    return (
      <div className="flex items-center gap-2 px-4 py-3 rounded-xl border border-emerald-800 bg-emerald-950/30 text-emerald-400 text-sm">
        <CheckCircle className="w-4 h-4 shrink-0" />
        <span className="font-medium">Approved — Proposal Generated</span>
        {report.reviewed_at && (
          <span className="text-xs text-emerald-600 ml-auto">
            {new Date(report.reviewed_at).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })}
          </span>
        )}
      </div>
    );
  }

  if (report.review_status === 'rejected') {
    return (
      <div className="px-4 py-3 rounded-xl border border-amber-800 bg-amber-950/30 space-y-1">
        <div className="flex items-center gap-2 text-amber-400 text-sm">
          <RotateCcw className="w-4 h-4 shrink-0" />
          <span className="font-medium">Revision Requested</span>
        </div>
        {report.review_notes && (
          <p className="text-xs text-amber-300/70 ml-6">{report.review_notes}</p>
        )}
      </div>
    );
  }

  // pending_review
  return (
    <div className="rounded-xl border border-amber-700 bg-amber-950/20 p-4 space-y-3">
      <div className="flex items-center gap-2 text-amber-400">
        <AlertCircle className="w-4 h-4 shrink-0" />
        <p className="text-sm font-medium">This analysis requires human review before generating a proposal</p>
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          className="gap-1.5 bg-emerald-700 hover:bg-emerald-600 text-white border-0"
          disabled={isLoading}
          onClick={onApprove}
        >
          <ThumbsUp className="w-3.5 h-3.5" />
          Approve & Generate Proposal
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="gap-1.5 border-amber-700 text-amber-400 hover:bg-amber-950/40"
          disabled={isLoading}
          onClick={() => setShowRevisionDialog(true)}
        >
          <RotateCcw className="w-3.5 h-3.5" />
          Request Revision
        </Button>
      </div>

      {showRevisionDialog && (
        <div className="mt-3 space-y-2">
          <textarea
            value={revisionNotes}
            onChange={(e) => setRevisionNotes(e.target.value)}
            placeholder="Describe what needs to be revised or researched further..."
            className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 placeholder-slate-500 resize-none focus:outline-none focus:border-amber-600"
            rows={3}
          />
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              className="bg-amber-700 hover:bg-amber-600 text-white border-0"
              disabled={isLoading || !revisionNotes.trim()}
              onClick={() => {
                onRequestRevision(revisionNotes.trim());
                setShowRevisionDialog(false);
              }}
            >
              Send Revision Request
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="text-slate-400"
              onClick={() => { setShowRevisionDialog(false); setRevisionNotes(''); }}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
