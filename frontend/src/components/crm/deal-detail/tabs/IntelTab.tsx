import { useState } from 'react';
import { Link } from 'react-router-dom';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  Brain,
  Building2,
  ExternalLink,
  CheckCircle2,
  AlertCircle,
  Loader2,
  RotateCcw,
  ThumbsUp,
  Clock,
  Search,
} from 'lucide-react';
import { useDealActions } from '../hooks/useDealActions';
import type { CrmDealWithContact } from '@/types/crm';

// ── IntelTab ─────────────────────────────────────────────────────────────────

interface IntelTabProps {
  deal: CrmDealWithContact;
}

export function IntelTab({ deal }: IntelTabProps) {
  const [revisionNotes, setRevisionNotes] = useState('');
  const [showRevisionInput, setShowRevisionInput] = useState(false);

  const {
    triggerResearch,
    researchLoading,
    approveReport,
    requestRevision,
    reportLoading,
  } = useDealActions();

  const status = deal.intelligence_status;
  const isResearching = status === 'running' || status === 'queued';
  const isDone = status === 'done';
  const confidencePct =
    isDone && deal.intelligence_confidence != null
      ? Math.round(deal.intelligence_confidence * 100)
      : null;

  // Combine loading states — the original used a single `loading` for all actions
  const loading = researchLoading || reportLoading;

  const handleApprove = () => {
    if (deal.report_id) approveReport(deal.report_id);
  };

  const handleRequestRevision = async () => {
    if (!deal.report_id) return;
    const ok = await requestRevision(deal.report_id, revisionNotes);
    if (ok) {
      setShowRevisionInput(false);
      setRevisionNotes('');
    }
  };

  const handleTriggerResearch = () => {
    if (deal.person_id) triggerResearch(deal.person_id);
  };

  if (isResearching) {
    return (
      <div className="p-5 flex flex-col items-center justify-center gap-3 text-center py-12">
        <div className="w-12 h-12 rounded-full bg-blue-50 dark:bg-blue-950/30 flex items-center justify-center">
          <Loader2 className="h-6 w-6 text-blue-500 animate-spin" />
        </div>
        <div>
          <p className="text-sm font-medium">Research in progress</p>
          <p className="text-xs text-muted-foreground mt-1">
            Nora is gathering intelligence on this lead.
          </p>
        </div>
      </div>
    );
  }

  if (!isDone && !deal.intelligence_summary) {
    return (
      <div className="p-5 flex flex-col items-center justify-center gap-3 text-center py-12">
        <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center">
          <Brain className="h-6 w-6 text-muted-foreground/40" />
        </div>
        <div>
          <p className="text-sm text-muted-foreground">No intelligence data yet.</p>
          {deal.person_id && (
            <Button
              variant="outline"
              size="sm"
              className="mt-3 gap-1.5"
              onClick={handleTriggerResearch}
              disabled={loading}
            >
              {loading ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <Search className="h-3.5 w-3.5" />
              )}
              Trigger Research
            </Button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="p-5 space-y-5">
      {/* Status header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {isDone ? (
            <CheckCircle2 className="h-4 w-4 text-green-500" />
          ) : (
            <AlertCircle className="h-4 w-4 text-amber-400" />
          )}
          <span className="text-sm font-medium">
            {isDone ? 'Intelligence Complete' : 'Partial Intelligence'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          {confidencePct != null && (
            <Badge variant="outline" className="text-xs">
              {confidencePct}% confidence
            </Badge>
          )}
          {(deal.research_pass_count ?? 0) > 0 && (
            <span className="text-[11px] text-muted-foreground flex items-center gap-1">
              <RotateCcw className="h-3 w-3" />
              {deal.research_pass_count} pass
              {(deal.research_pass_count ?? 0) !== 1 ? 'es' : ''}
            </span>
          )}
        </div>
      </div>

      {/* Summary */}
      {deal.intelligence_summary && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            AI Profile Summary
          </h4>
          <div className="p-3 rounded-lg bg-muted/30 border border-border/50">
            <p className="text-sm leading-relaxed whitespace-pre-wrap">
              {deal.intelligence_summary}
            </p>
          </div>
        </div>
      )}

      {/* Business report */}
      {deal.report_id && (
        <div>
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              Business Report
            </h4>
            <Button variant="ghost" size="sm" className="h-7 text-xs gap-1" asChild>
              <Link to={`/business-reports/${deal.report_id}`}>
                Open Report <ExternalLink className="h-3 w-3" />
              </Link>
            </Button>
          </div>
          {deal.report_review_status && (
            <Card className="bg-muted/30 border-border/60">
              <CardContent className="p-3 space-y-3">
                <div className="flex items-center gap-2">
                  {deal.report_review_status === 'approved' ? (
                    <CheckCircle2 className="h-4 w-4 text-green-500" />
                  ) : deal.report_review_status === 'rejected' ? (
                    <AlertCircle className="h-4 w-4 text-red-400" />
                  ) : (
                    <Clock className="h-4 w-4 text-amber-400" />
                  )}
                  <span className="text-sm font-medium">
                    {deal.report_review_status === 'approved'
                      ? 'Report Approved'
                      : deal.report_review_status === 'rejected'
                        ? 'Revision Requested'
                        : 'Pending Review'}
                  </span>
                </div>
                {deal.report_review_status !== 'approved' && (
                  <div className="flex gap-2">
                    <Button
                      size="sm"
                      className="h-7 text-xs flex-1 gap-1"
                      onClick={handleApprove}
                      disabled={loading}
                    >
                      <ThumbsUp className="h-3 w-3" />
                      Approve
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-7 text-xs flex-1 gap-1"
                      onClick={() => setShowRevisionInput(!showRevisionInput)}
                      disabled={loading}
                    >
                      <RotateCcw className="h-3 w-3" />
                      Revise
                    </Button>
                  </div>
                )}
                {showRevisionInput && (
                  <div className="space-y-2">
                    <textarea
                      className="w-full text-xs border rounded p-2 min-h-[60px] resize-none focus:outline-none focus:ring-1 focus:ring-primary bg-background"
                      placeholder="Describe what needs to be revised..."
                      value={revisionNotes}
                      onChange={(e) => setRevisionNotes(e.target.value)}
                    />
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 text-xs w-full"
                      onClick={handleRequestRevision}
                      disabled={loading || !revisionNotes.trim()}
                    >
                      Submit Revision Request
                    </Button>
                  </div>
                )}
              </CardContent>
            </Card>
          )}
        </div>
      )}

      {/* Company intel */}
      {deal.contact_company && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Company Intelligence
          </h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3">
              <div className="flex items-center gap-2">
                <Building2 className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="text-sm font-medium">{deal.contact_company}</span>
                {deal.company_intelligence_status === 'done' && (
                  <CheckCircle2 className="h-3.5 w-3.5 text-green-500 ml-auto" />
                )}
                {(deal.company_intelligence_status === 'running' ||
                  deal.company_intelligence_status === 'queued') && (
                  <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin ml-auto" />
                )}
                {(!deal.company_intelligence_status ||
                  deal.company_intelligence_status === 'idle') && (
                  <AlertCircle className="h-3.5 w-3.5 text-muted-foreground/40 ml-auto" />
                )}
              </div>
              <p className="text-xs text-muted-foreground mt-1.5">
                {deal.company_intelligence_status === 'done'
                  ? 'Company research complete'
                  : deal.company_intelligence_status === 'running' ||
                      deal.company_intelligence_status === 'queued'
                    ? 'Company research in progress\u2026'
                    : 'No company research yet'}
              </p>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Re-trigger */}
      {deal.person_id && !isResearching && (
        <Button
          variant="outline"
          size="sm"
          className="w-full gap-1.5"
          onClick={handleTriggerResearch}
          disabled={loading}
        >
          {loading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Search className="h-3.5 w-3.5" />
          )}
          {isDone ? 'Run Additional Research' : 'Trigger Research'}
        </Button>
      )}

      {deal.person_id && (
        <div className="pt-2 border-t">
          <Button variant="ghost" size="sm" className="w-full h-8 text-xs gap-1.5" asChild>
            <Link to={`/people/${deal.person_id}`}>
              <Brain className="h-3.5 w-3.5" />
              View Full Intelligence Profile
              <ExternalLink className="h-3 w-3 ml-auto" />
            </Link>
          </Button>
        </div>
      )}
    </div>
  );
}
