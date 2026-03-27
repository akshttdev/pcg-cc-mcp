import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { crmKeys } from '@/lib/query-keys';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  Brain,
  Building2,
  CheckCircle2,
  AlertCircle,
  ClipboardCheck,
  Loader2,
  ShieldCheck,
  Search,
  CircleDot,
  Phone,
  FileText,
  ExternalLink,
  User,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { cn } from '@/lib/utils';
import { review as tid } from 'shared/testids';
import { handleApiResponse, makeRequest } from '@/lib/api/client';

import { useDealActions } from '../hooks/useDealActions';
import type { CrmDealWithContact } from '@/types/crm';

// ── Task type for deal-linked tasks ─────────────────────────────────────────

interface DealTask {
  id: string;
  title: string;
  status: string;
  assignee_id?: string;
  project_id?: string;
  updated_at: string;
  created_at: string;
}

// ── Stage checklist data ─────────────────────────────────────────────────────

const STAGE_CHECKLIST: Record<string, { item: string; description?: string }[]> = {
  lead: [
    {
      item: 'Person research complete',
      description: '"Who Is" research completed on this contact',
    },
    { item: 'Company research complete', description: 'Company intelligence is available' },
    {
      item: 'Lead quality verified',
      description: 'Account Manager has confirmed lead is worth pursuing',
    },
  ],
  'business analysis': [
    {
      item: 'Business report reviewed',
      description: 'MBA-level analysis report has been read and verified',
    },
    {
      item: 'Market opportunity assessed',
      description: 'We understand their market position and opportunity',
    },
    { item: 'Discovery call scheduled', description: 'Meeting with prospect is booked' },
  ],
  'discovery and analysis': [
    {
      item: 'Discovery call completed',
      description: 'Call notes and transcript are in the system',
    },
    {
      item: 'Pain points identified',
      description: 'We understand what problems they need solved',
    },
    { item: 'Budget range confirmed', description: 'We have a clear budget expectation' },
    {
      item: 'Knowledge is proposal-ready',
      description: 'We have enough to build a strong proposal',
    },
  ],
  'build proposal': [
    {
      item: 'Services defined and scoped',
      description: 'Deliverables are clearly outlined',
    },
    {
      item: 'Pricing approved internally',
      description: 'Pricing reviewed and signed off',
    },
    { item: 'Timeline is realistic', description: 'Delivery schedule is achievable' },
    {
      item: 'Topsi has generated proposal',
      description: 'AI proposal draft has been created',
    },
  ],
  'polish assets': [
    {
      item: 'Presentation deck complete',
      description: 'Slides are polished and client-ready',
    },
    { item: 'Report card accurate', description: 'Data and metrics are verified' },
    {
      item: 'Graphical quality verified',
      description: 'Design quality meets our standards',
    },
    {
      item: 'PM/EP sign-off received',
      description: 'Leadership has approved the proposal',
    },
  ],
  presentation: [
    {
      item: 'Meeting scheduled',
      description: 'Date and time confirmed with client',
    },
    {
      item: 'Decision maker confirmed',
      description: 'The right stakeholders will be present',
    },
    { item: 'Presentation rehearsed', description: 'Team is prepared for the pitch' },
  ],
  'follow up': [
    {
      item: 'Follow-up sent within 24h',
      description: 'Post-presentation email sent with recap and next steps',
    },
    {
      item: 'Objections addressed',
      description: 'Any client concerns have been responded to',
    },
    {
      item: 'Decision timeline confirmed',
      description: 'Client has given a decision timeline',
    },
    {
      item: 'Final terms agreed',
      description: 'Pricing and scope are mutually confirmed',
    },
  ],
};

// ── ReviewTab ────────────────────────────────────────────────────────────────

interface ReviewTabProps {
  deal: CrmDealWithContact;
  stageName?: string;
}

export function ReviewTab({ deal, stageName }: ReviewTabProps) {
  const [checkedItems, setCheckedItems] = useState<Set<string>>(new Set());
  const { advanceDeal, advanceLoading, triggerResearch, researchLoading, completeReviewTask, completeLoading } = useDealActions();

  const hasReviewTask = !!deal.review_task_id;
  const taskDone = deal.review_task_status === 'done';
  const effectiveStage = (stageName || deal.stage || '').toLowerCase();

  const handleAdvance = () => advanceDeal(deal.id, deal.name);
  const handleComplete = () => {
    if (deal.review_task_id) completeReviewTask(deal.review_task_id);
  };

  const handleTriggerResearch = () => {
    if (deal.person_id) triggerResearch(deal.person_id);
  };

  // Fetch all tasks linked to this deal (for completed reviews section)
  const { data: dealTasks = [], isError: tasksError } = useQuery<DealTask[]>({
    queryKey: [...crmKeys.deal(deal.id), 'tasks'],
    queryFn: async () => {
      const response = await makeRequest(`/api/tasks?crm_deal_id=${deal.id}`);
      return handleApiResponse<DealTask[]>(response);
    },
    staleTime: 30_000,
  });

  const completedTasks = dealTasks.filter(t => t.status === 'done' || t.status === 'cancelled');

  const checklist = STAGE_CHECKLIST[effectiveStage] || [];
  const allChecked =
    checklist.length > 0 && checklist.every(({ item }) => checkedItems.has(item));

  return (
    <div className="p-5 space-y-5">
      {/* Current review task status */}
      {hasReviewTask ? (
        <Card
          className={cn(
            'border-border/60',
            taskDone
              ? 'bg-green-50 dark:bg-green-950/20'
              : 'bg-amber-50 dark:bg-amber-950/20'
          )}
        >
          <CardContent className="p-4 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                {taskDone ? (
                  <CheckCircle2 className="h-4 w-4 text-green-500" />
                ) : (
                  <CircleDot className="h-4 w-4 text-amber-500 animate-pulse" />
                )}
                <span className="text-sm font-medium">
                  {taskDone ? 'Review Complete' : 'Review In Progress'}
                </span>
              </div>
              {deal.review_task_assignee && (
                <Badge variant="outline" className="text-xs">
                  {deal.review_task_assignee}
                </Badge>
              )}
            </div>
            {deal.review_task_status && !taskDone && (
              <p className="text-xs text-muted-foreground capitalize">
                Status: {deal.review_task_status.replace(/_/g, ' ')}
              </p>
            )}
            {!taskDone && (
              <Button
                size="sm"
                variant="outline"
                className="w-full gap-1.5"
                onClick={handleComplete}
                disabled={completeLoading}
                data-testid={tid.markComplete}
              >
                {completeLoading ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <CheckCircle2 className="h-3.5 w-3.5" />
                )}
                Mark Review Complete
              </Button>
            )}
            {taskDone && (
              <Button
                size="sm"
                className="w-full gap-1.5"
                onClick={handleAdvance}
                disabled={advanceLoading}
                data-testid={tid.approveAdvance}
              >
                {advanceLoading ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <ShieldCheck className="h-3.5 w-3.5" />
                )}
                Approve & Advance to Next Stage
              </Button>
            )}
          </CardContent>
        </Card>
      ) : (
        <Card className="bg-muted/30 border-border/60">
          <CardContent className="p-4 text-center space-y-2">
            <AlertCircle className="h-6 w-6 mx-auto text-muted-foreground/40" />
            <p className="text-sm text-muted-foreground">No review task for this stage.</p>
            {!['closed won', 'closed lost', 'presentation', 'follow up'].includes(effectiveStage) && (
              <p className="text-xs text-muted-foreground/70">
                A review task is created automatically when entering an active stage.
              </p>
            )}
          </CardContent>
        </Card>
      )}

      {/* Completed review tasks */}
      {tasksError && (
        <p className="text-xs text-red-500">Failed to load completed reviews</p>
      )}
      {completedTasks.length > 0 && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 flex items-center gap-1.5">
            <ClipboardCheck className="h-3 w-3" /> Completed Reviews
          </h4>
          <div className="space-y-1.5">
            {completedTasks.map((task) => (
              <Link
                key={task.id}
                to={task.project_id ? `/projects/${task.project_id}/tasks/${task.id}` : '/my-tasks'}
              >
                <Card className="bg-green-50/50 dark:bg-green-950/10 border-border/40 hover:bg-green-50 dark:hover:bg-green-950/20 transition-colors cursor-pointer">
                  <CardContent className="p-2.5 flex items-center gap-2">
                    <CheckCircle2 className="h-3.5 w-3.5 text-green-500 shrink-0" />
                    <span className="text-xs font-medium flex-1 truncate">{task.title}</span>
                    <span className="text-xs text-muted-foreground shrink-0">
                      {formatDistanceToNow(new Date(task.updated_at), { addSuffix: true })}
                    </span>
                    <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
                  </CardContent>
                </Card>
              </Link>
            ))}
          </div>
        </div>
      )}

      {/* Stage review checklist */}
      {checklist.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              Stage Checklist
            </h4>
            <span className="text-xs text-muted-foreground">
              {checkedItems.size}/{checklist.length}
            </span>
          </div>
          <div className="space-y-2">
            {checklist.map(({ item, description }) => {
              const checked = checkedItems.has(item);
              return (
                <button
                  key={item}
                  type="button"
                  className={cn(
                    'w-full flex items-start gap-2.5 p-2.5 rounded-lg border text-left transition-all',
                    checked
                      ? 'bg-green-50 dark:bg-green-950/20 border-green-200 dark:border-green-800'
                      : 'bg-muted/30 border-border/50 hover:bg-muted/60'
                  )}
                  onClick={() =>
                    setCheckedItems((prev) => {
                      const next = new Set(prev);
                      if (next.has(item)) next.delete(item);
                      else next.add(item);
                      return next;
                    })
                  }
                >
                  <div
                    className={cn(
                      'w-4 h-4 rounded border-2 shrink-0 mt-0.5 flex items-center justify-center transition-all',
                      checked ? 'bg-green-500 border-green-500' : 'border-muted-foreground/30'
                    )}
                  >
                    {checked && <CheckCircle2 className="h-3 w-3 text-white" />}
                  </div>
                  <div>
                    <p
                      className={cn(
                        'text-sm font-medium',
                        checked && 'line-through text-muted-foreground'
                      )}
                    >
                      {item}
                    </p>
                    {description && (
                      <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
                    )}
                  </div>
                </button>
              );
            })}
          </div>
          {allChecked && (
            <div className="mt-3 p-2.5 rounded-lg bg-green-50 dark:bg-green-950/20 border border-green-200 dark:border-green-800 text-center">
              <p className="text-xs font-medium text-green-700 dark:text-green-400 flex items-center justify-center gap-1.5">
                <CheckCircle2 className="h-3.5 w-3.5" />
                All items checked — ready to advance
              </p>
            </div>
          )}
        </div>
      )}

      {/* Quick research trigger */}
      {deal.person_id && (
        <Button
          variant="outline"
          size="sm"
          className="w-full gap-1.5"
          onClick={handleTriggerResearch}
          disabled={researchLoading}
        >
          {researchLoading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Search className="h-3.5 w-3.5" />
          )}
          Request Additional Research
        </Button>
      )}

      {/* Intel snapshot */}
      {(deal.intelligence_summary || deal.company_intelligence_status === 'done') && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Intel Snapshot
          </h4>
          <div className="grid grid-cols-1 gap-2">
            {deal.intelligence_summary && (
              <Card className="bg-muted/20 border-border/50">
                <CardContent className="p-3">
                  <div className="flex items-center justify-between gap-1.5 mb-1.5">
                    <div className="flex items-center gap-1.5">
                      <Brain className="h-3.5 w-3.5 text-blue-500" />
                      <span className="text-xs font-medium">Person Intel</span>
                    </div>
                    {deal.person_id && (
                      <Link to={`/people/${deal.person_id}?tab=reports`} className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-0.5">
                        <ExternalLink className="h-3 w-3" /> Intel
                      </Link>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground leading-relaxed line-clamp-3">
                    {deal.intelligence_summary}
                  </p>
                </CardContent>
              </Card>
            )}
            {deal.company_intelligence_status === 'done' && (
              <Card className="bg-muted/20 border-border/50">
                <CardContent className="p-3">
                  <div className="flex items-center justify-between gap-1.5 mb-1.5">
                    <div className="flex items-center gap-1.5">
                      <Building2 className="h-3.5 w-3.5 text-purple-500" />
                      <span className="text-xs font-medium">
                        Company Intel — {deal.contact_company}
                      </span>
                    </div>
                    {deal.company_id && (
                      <Link to={`/companies/${deal.company_id}`} className="text-xs text-muted-foreground hover:text-foreground flex items-center gap-0.5">
                        <ExternalLink className="h-3 w-3" /> View
                      </Link>
                    )}
                  </div>
                  <p className="text-xs text-muted-foreground">Company research complete.</p>
                </CardContent>
              </Card>
            )}
          </div>
        </div>
      )}

      {/* Data Sources */}
      <DataSourcesSection deal={deal} />
    </div>
  );
}

// ── DataSourcesSection ───────────────────────────────────────────────────────

function DataSourcesSection({ deal }: { deal: CrmDealWithContact }) {
  const { data: callLogs = [], isLoading } = useQuery<CallLogSummary[]>({
    queryKey: crmKeys.callLogsDeal(deal.id),
    queryFn: async () => {
      const res = await fetch(`/api/communications/calls?crm_deal_id=${deal.id}&limit=10`, { credentials: 'include' });
      const json = await res.json();
      return json.data ?? [];
    },
    staleTime: 60_000,
  });

  const hasSources = deal.report_id || deal.person_id || callLogs.length > 0;
  if (!hasSources && !isLoading) return null;

  return (
    <div>
      <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
        Data Sources
      </h4>
      <div className="space-y-2">
        {/* Business Report */}
        {deal.report_id && (
          <Link to={`/business-reports/${deal.report_id}`}>
            <Card className="bg-muted/20 border-border/50 hover:bg-muted/40 transition-colors cursor-pointer">
              <CardContent className="p-3">
                <div className="flex items-center gap-2">
                  <FileText className="h-3.5 w-3.5 text-indigo-500 shrink-0" />
                  <span className="text-xs font-medium flex-1">Business Analysis Report</span>
                  <ExternalLink className="h-3 w-3 text-muted-foreground" />
                </div>
                {deal.report_status && (
                  <p className="text-xs text-muted-foreground mt-1 capitalize">
                    Status: {deal.report_status.replace(/_/g, ' ')}
                  </p>
                )}
              </CardContent>
            </Card>
          </Link>
        )}

        {/* Person Profile */}
        {deal.person_id && (
          <Link to={`/people/${deal.person_id}`}>
            <Card className="bg-muted/20 border-border/50 hover:bg-muted/40 transition-colors cursor-pointer">
              <CardContent className="p-3">
                <div className="flex items-center gap-2">
                  <User className="h-3.5 w-3.5 text-blue-500 shrink-0" />
                  <span className="text-xs font-medium flex-1">
                    {deal.contact_name ?? 'Contact Profile'}
                  </span>
                  <ExternalLink className="h-3 w-3 text-muted-foreground" />
                </div>
                {deal.intelligence_status && (
                  <p className="text-xs text-muted-foreground mt-1 capitalize">
                    Intel: {deal.intelligence_status}
                  </p>
                )}
              </CardContent>
            </Card>
          </Link>
        )}

        {/* Call Transcripts */}
        {isLoading && (
          <div className="h-10 bg-muted/20 rounded-lg animate-pulse" />
        )}
        {callLogs.map((log) => (
          <Card key={log.id} className="bg-muted/20 border-border/50">
            <CardContent className="p-3">
              <div className="flex items-center gap-2">
                <Phone className="h-3.5 w-3.5 text-green-500 shrink-0" />
                <span className="text-xs font-medium flex-1 truncate">
                  {log.caller_name ?? log.from_number}
                </span>
                {log.created_at && (
                  <span className="text-xs text-muted-foreground shrink-0">
                    {formatDistanceToNow(new Date(log.created_at), { addSuffix: true })}
                  </span>
                )}
              </div>
              {log.summary && (
                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{log.summary}</p>
              )}
              {log.transcription_status === 'completed' && (
                <Badge variant="outline" className="text-xs px-1.5 py-0 mt-1.5 text-green-600">
                  Transcript available
                </Badge>
              )}
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

interface CallLogSummary {
  id: string;
  caller_name?: string;
  from_number: string;
  summary?: string;
  transcription_status?: string;
  created_at: string;
}
