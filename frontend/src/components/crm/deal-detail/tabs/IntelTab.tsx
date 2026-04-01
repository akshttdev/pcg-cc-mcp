import {
  AlertCircle,
  Brain,
  Building2,
  CheckCircle2,
  ClipboardCheck,
  Clock,
  ExternalLink,
  FileText,
  Loader2,
  RotateCcw,
  Search,
  ThumbsUp,
  User,
} from 'lucide-react';
import { useState } from 'react';
import { Link } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { StatusBadge } from '@/components/ui/status-badge';
import { getStatusInfo } from '@/lib/status-utils';
import type { CrmDealWithContact } from '@/types/crm';

import { useDealActions } from '../hooks/useDealActions';

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

  const loading = researchLoading || reportLoading;

  const companyStatus = deal.company_intelligence_status;
  const companyDone = companyStatus === 'done';
  const companyResearching =
    companyStatus === 'running' || companyStatus === 'queued';

  const hasAnyData =
    deal.intelligence_summary ||
    deal.company_intelligence_summary ||
    companyDone ||
    deal.report_id;

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
    // Prefer contact-targeted API (canonical after unification)
    if (deal.crm_contact_id) {
      triggerResearch(deal.crm_contact_id, true);
    } else if (deal.person_id) {
      triggerResearch(deal.person_id);
    }
  };

  // Research actively in flight with no data yet
  if (isResearching && !hasAnyData) {
    return (
      <div className="p-5 flex flex-col items-center justify-center gap-3 text-center py-12">
        <div className="w-12 h-12 rounded-full bg-blue-50 dark:bg-blue-950/30 flex items-center justify-center">
          <Loader2 className="h-6 w-6 text-blue-500 animate-spin" />
        </div>
        <div>
          <p className="text-sm font-medium">Research in progress</p>
          <p className="text-xs text-muted-foreground mt-1">
            The intel agent is gathering intelligence on this lead.
          </p>
        </div>
      </div>
    );
  }

  // No data at all yet
  if (!hasAnyData) {
    return (
      <div className="p-5 flex flex-col items-center justify-center gap-3 text-center py-12">
        <div className="w-12 h-12 rounded-full bg-muted flex items-center justify-center">
          <Brain className="h-6 w-6 text-muted-foreground/40" />
        </div>
        <div>
          <p className="text-sm text-muted-foreground">
            No intelligence data yet.
          </p>
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
          {(() => {
            const intelStatus = getStatusInfo(status ?? 'idle', 'intelligence');
            return (
              <StatusBadge
                status={intelStatus.variant}
                label={
                  isDone
                    ? 'Intelligence Complete'
                    : isResearching
                      ? 'Research Running'
                      : 'Partial Intelligence'
                }
                icon={intelStatus.icon}
                pulse={isResearching}
              />
            );
          })()}
        </div>
        <div className="flex items-center gap-2">
          {confidencePct != null && (
            <Badge variant="outline" className="text-xs">
              {confidencePct}% confidence
            </Badge>
          )}
          {(deal.research_pass_count ?? 0) > 0 && (
            <span className="text-xs text-muted-foreground flex items-center gap-1">
              <RotateCcw className="h-3 w-3" />
              {deal.research_pass_count} pass
              {(deal.research_pass_count ?? 0) !== 1 ? 'es' : ''}
            </span>
          )}
        </div>
      </div>

      {/* Person Intelligence */}
      {deal.intelligence_summary && (
        <Card className="bg-muted/30 border-border/60">
          <CardHeader className="p-3 pb-2">
            <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide flex items-center gap-1.5">
              <User className="h-3.5 w-3.5" />
              Person Intelligence
              {(deal.crm_contact_id || deal.person_id) && (
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-5 text-xs gap-1 ml-auto px-1.5"
                  asChild
                >
                  <Link
                    to={`/contacts/${deal.crm_contact_id ?? deal.person_id}`}
                  >
                    Profile <ExternalLink className="h-2.5 w-2.5" />
                  </Link>
                </Button>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="p-3 pt-0">
            <p className="text-sm leading-relaxed whitespace-pre-wrap">
              {deal.intelligence_summary}
            </p>
          </CardContent>
        </Card>
      )}

      {/* Company Intelligence */}
      {deal.contact_company && (
        <Card className="bg-muted/30 border-border/60">
          <CardHeader className="p-3 pb-2">
            <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide flex items-center gap-1.5">
              <Building2 className="h-3.5 w-3.5" />
              <span className="truncate">{deal.contact_company}</span>
              <div className="ml-auto flex items-center gap-1.5 shrink-0">
                {companyDone && (
                  <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />
                )}
                {companyResearching && (
                  <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin" />
                )}
                {!companyDone && !companyResearching && (
                  <AlertCircle className="h-3.5 w-3.5 text-muted-foreground/40" />
                )}
                {deal.company_id && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-xs gap-1 px-1.5"
                    asChild
                  >
                    <Link to={`/companies/${deal.company_id}`}>
                      Profile <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                )}
              </div>
            </CardTitle>
          </CardHeader>
          <CardContent className="p-3 pt-0">
            {deal.company_intelligence_summary ? (
              <p className="text-sm leading-relaxed whitespace-pre-wrap">
                {deal.company_intelligence_summary}
              </p>
            ) : (
              <p className="text-xs text-muted-foreground">
                {companyResearching
                  ? 'Company research in progress\u2026'
                  : companyDone
                    ? 'Research complete — view company profile for full details.'
                    : 'No company research yet'}
              </p>
            )}
          </CardContent>
        </Card>
      )}

      {/* Business Analysis Artifact */}
      {deal.report_id && (
        <Card className="border-primary/20 bg-primary/5">
          <CardHeader className="p-3 pb-2">
            <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide flex items-center gap-1.5">
              <FileText className="h-3.5 w-3.5 text-primary" />
              Business Analysis
              <Button
                variant="ghost"
                size="sm"
                className="h-5 text-xs gap-1 ml-auto px-1.5 text-primary"
                asChild
              >
                <Link to={`/business-reports/${deal.report_id}`}>
                  Open Report <ExternalLink className="h-2.5 w-2.5" />
                </Link>
              </Button>
            </CardTitle>
          </CardHeader>
          <CardContent className="p-3 pt-0 space-y-3">
            {/* Review status */}
            <div className="flex items-center gap-2">
              {deal.report_review_status === 'approved' ? (
                <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />
              ) : deal.report_review_status === 'rejected' ? (
                <AlertCircle className="h-4 w-4 text-red-400 shrink-0" />
              ) : (
                <Clock className="h-4 w-4 text-amber-400 shrink-0" />
              )}
              <span className="text-sm font-medium">
                {deal.report_review_status === 'approved'
                  ? 'Report Approved'
                  : deal.report_review_status === 'rejected'
                    ? 'Revision Requested'
                    : 'Pending Human Review'}
              </span>
            </div>

            {/* Review task assignment */}
            {deal.review_task_id &&
              deal.review_task_status !== 'done' &&
              deal.review_task_status !== 'cancelled' && (
                <div className="flex items-start gap-2 text-xs text-muted-foreground bg-background/60 rounded-md px-2.5 py-2">
                  <ClipboardCheck className="h-3.5 w-3.5 shrink-0 text-amber-500 mt-0.5" />
                  <span>
                    Review task
                    {deal.review_task_assignee ? (
                      <>
                        {' '}
                        assigned to{' '}
                        <span className="font-medium text-foreground">
                          {deal.review_task_assignee}
                        </span>
                      </>
                    ) : (
                      ' unassigned'
                    )}{' '}
                    — refine analysis before discovery call
                  </span>
                </div>
              )}

            {/* Approve / Revise */}
            {deal.report_review_status !== 'approved' && (
              <>
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
              </>
            )}
          </CardContent>
        </Card>
      )}

      {/* Re-trigger research */}
      {(deal.crm_contact_id || deal.person_id) && !isResearching && (
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

      {/* Full profile link */}
      {(deal.crm_contact_id || deal.person_id) && (
        <div className="pt-2 border-t">
          <Button
            variant="ghost"
            size="sm"
            className="w-full h-8 text-xs gap-1.5"
            asChild
          >
            <Link to={`/contacts/${deal.crm_contact_id ?? deal.person_id}`}>
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
