import { useNavigate, Link } from 'react-router-dom';
import { AskTopsiButton } from '@/components/topsi/AskTopsiButton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import {
  Calendar,
  Mail,
  Building2,
  ExternalLink,
  FolderKanban,
  Tag,
  BarChart3,
  Rocket,
  ListTodo,
  FileText,
  Workflow,
  CheckCircle2,
  Clock,
  TrendingUp,
  DollarSign,
  User,
  CheckSquare,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact } from '@/types/crm';

// ── MetricCard (local helper) ────────────────────────────────────────────────

function MetricCard({
  label,
  value,
  icon: Icon,
  accent,
  sub,
}: {
  label: string;
  value: string;
  icon: React.ElementType;
  accent: string;
  sub?: React.ReactNode;
}) {
  return (
    <Card className="bg-muted/30 border-border/60">
      <CardContent className="p-3">
        <div className="flex items-center gap-1.5 mb-1">
          <Icon className="h-3.5 w-3.5" style={{ color: accent }} />
          <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide">
            {label}
          </span>
        </div>
        <p className="text-sm font-semibold">{value}</p>
        {sub}
      </CardContent>
    </Card>
  );
}

// ── OverviewTab ──────────────────────────────────────────────────────────────

interface OverviewTabProps {
  deal: CrmDealWithContact;
  stageColor: string;
  onConvert: () => void;
  orgId?: string;
}

export function OverviewTab({ deal, stageColor, onConvert, orgId }: OverviewTabProps) {
  const navigate = useNavigate();
  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;
  const taskPct = taskTotal > 0 ? Math.round((taskDone / taskTotal) * 100) : 0;

  let tags: string[] = [];
  if (deal.tags) {
    try {
      tags = JSON.parse(deal.tags);
    } catch {
      tags = deal.tags
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean);
    }
  }

  let sourceInfo: { dataSourceId?: string; workflowRunId?: string } | null = null;
  if (deal.custom_fields) {
    try {
      const cf =
        typeof deal.custom_fields === 'string'
          ? JSON.parse(deal.custom_fields)
          : deal.custom_fields;
      if (cf.source_data_source_id || cf.source_workflow_run_id) {
        sourceInfo = {
          dataSourceId: cf.source_data_source_id,
          workflowRunId: cf.source_workflow_run_id,
        };
      }
    } catch {
      /* ignore */
    }
  }

  return (
    <div className="p-5 space-y-5">
      {/* Description */}
      {deal.description && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Description
          </h4>
          <p className="text-sm whitespace-pre-wrap leading-relaxed">{deal.description}</p>
        </div>
      )}

      {/* Key metrics grid */}
      <div className="grid grid-cols-2 gap-3">
        <MetricCard
          label="Probability"
          value={deal.probability > 0 ? `${deal.probability}%` : '\u2014'}
          icon={TrendingUp}
          accent={stageColor}
          sub={
            deal.probability > 0 ? (
              <div className="mt-1.5 h-1 bg-muted rounded-full overflow-hidden">
                <div
                  className="h-full rounded-full"
                  style={{ width: `${deal.probability}%`, backgroundColor: stageColor }}
                />
              </div>
            ) : undefined
          }
        />
        <MetricCard
          label="Currency"
          value={deal.currency || 'USD'}
          icon={DollarSign}
          accent={stageColor}
        />
        {taskTotal > 0 && (
          <MetricCard
            label="Task Progress"
            value={`${taskDone} / ${taskTotal}`}
            icon={CheckSquare}
            accent={stageColor}
            sub={
              <div className="mt-1.5 h-1 bg-muted rounded-full overflow-hidden">
                <div
                  className={cn(
                    'h-full rounded-full',
                    taskPct === 100 ? 'bg-green-500' : 'bg-blue-500'
                  )}
                  style={{ width: `${taskPct}%` }}
                />
              </div>
            }
          />
        )}
        {(deal.deliverable_count ?? 0) > 0 && (
          <MetricCard
            label="Deliverables"
            value={String(deal.deliverable_count)}
            icon={FileText}
            accent={stageColor}
          />
        )}
      </div>

      {/* Linked Project */}
      {deal.project_name && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Linked Project
          </h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2.5">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <FolderKanban className="h-4 w-4 text-primary shrink-0" />
                  <span className="text-sm font-medium truncate">{deal.project_name}</span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs shrink-0 gap-1"
                  onClick={() => navigate(`/projects/${deal.project_id}/tasks`)}
                >
                  Open <ExternalLink className="h-3 w-3" />
                </Button>
              </div>
              {taskTotal > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <ListTodo className="h-3 w-3" />
                      {taskDone}/{taskTotal} tasks done
                    </span>
                    <span
                      className={cn(
                        'font-medium',
                        taskPct === 100 ? 'text-green-600' : 'text-muted-foreground'
                      )}
                    >
                      {taskPct}%
                    </span>
                  </div>
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className={cn(
                        'h-full rounded-full transition-all',
                        taskPct === 100 ? 'bg-green-500' : 'bg-blue-500'
                      )}
                      style={{ width: `${taskPct}%` }}
                    />
                  </div>
                  {(deal.deliverable_count ?? 0) > 0 && (
                    <p className="text-[11px] text-muted-foreground flex items-center gap-1">
                      <FileText className="h-3 w-3" />
                      {deal.deliverable_count} deliverable
                      {(deal.deliverable_count ?? 0) !== 1 ? 's' : ''}
                    </p>
                  )}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {/* Contact */}
      <div>
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Contact
        </h4>
        <Card className="bg-muted/30 border-border/60">
          <CardContent className="p-3 space-y-1.5">
            {deal.contact_name && (
              <div className="flex items-center gap-2 text-sm">
                <User className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="font-medium">{deal.contact_name}</span>
                {deal.person_id && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-[10px] px-1 ml-auto gap-0.5"
                    asChild
                  >
                    <Link to={`/people/${deal.person_id}`}>
                      View profile <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                )}
              </div>
            )}
            {deal.contact_email && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Mail className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">{deal.contact_email}</span>
              </div>
            )}
            {deal.contact_company && (
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <Building2 className="h-3.5 w-3.5 shrink-0" />
                <span className="truncate">{deal.contact_company}</span>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Timeline */}
      <div>
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
          Timeline
        </h4>
        <div className="space-y-1.5 text-sm">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Calendar className="h-3.5 w-3.5 shrink-0" />
            <span className="text-muted-foreground/70">Created</span>
            <span className="ml-auto text-foreground">
              {new Date(deal.created_at).toLocaleDateString()}
            </span>
          </div>
          {deal.last_activity_at && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <BarChart3 className="h-3.5 w-3.5 shrink-0" />
              <span className="text-muted-foreground/70">Last activity</span>
              <span className="ml-auto text-foreground">
                {formatDistanceToNow(new Date(deal.last_activity_at), { addSuffix: true })}
              </span>
            </div>
          )}
          {deal.expected_close_date && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Clock className="h-3.5 w-3.5 shrink-0" />
              <span className="text-muted-foreground/70">Expected close</span>
              <span className="ml-auto text-foreground">
                {new Date(deal.expected_close_date).toLocaleDateString('en-US', {
                  month: 'short',
                  day: 'numeric',
                  year: 'numeric',
                })}
              </span>
            </div>
          )}
          {deal.actual_close_date && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-500" />
              <span className="text-muted-foreground/70">Closed</span>
              <span className="ml-auto text-foreground">
                {new Date(deal.actual_close_date).toLocaleDateString()}
              </span>
            </div>
          )}
        </div>
      </div>

      {/* Tags */}
      {tags.length > 0 && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Tags
          </h4>
          <div className="flex flex-wrap gap-1.5">
            {tags.map((tag) => (
              <Badge key={tag} variant="secondary" className="text-xs gap-1">
                <Tag className="h-3 w-3" />
                {tag}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Win/Lost reason */}
      {deal.win_reason && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Win Reason
          </h4>
          <p className="text-sm">{deal.win_reason}</p>
        </div>
      )}
      {deal.lost_reason && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Lost Reason
          </h4>
          <p className="text-sm">{deal.lost_reason}</p>
        </div>
      )}

      {/* Source provenance */}
      {sourceInfo && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">
            Source
          </h4>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Workflow className="h-3.5 w-3.5 shrink-0" />
            {orgId ? (
              <button type="button" className="text-primary hover:underline text-left" onClick={() => {}}>
                Imported via workflow
              </button>
            ) : (
              <span>Imported via workflow</span>
            )}
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="space-y-2 pt-2 border-t">
        <Button size="sm" className="w-full gap-1.5" onClick={onConvert}>
          <Rocket className="h-3.5 w-3.5" />
          Convert to Project
        </Button>
        <AskTopsiButton
          entityType="crm_deal"
          entityId={deal.id}
          entityName={deal.name}
          className="w-full"
        />
      </div>
    </div>
  );
}
