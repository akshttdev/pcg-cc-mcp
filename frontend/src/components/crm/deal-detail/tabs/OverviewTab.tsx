import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { formatDistanceToNow } from 'date-fns';
import {
  BarChart3,
  Building2,
  Calendar,
  CheckCircle2,
  CheckSquare,
  Clock,
  DollarSign,
  ExternalLink,
  FileText,
  FolderKanban,
  ListTodo,
  Mail,
  MessageSquare,
  Phone,
  Rocket,
  Tag,
  TrendingUp,
  User,
  Users,
  Video,
  Workflow,
  Zap,
} from 'lucide-react';
import { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { toast } from 'sonner';

import { AskTopsiButton } from '@/components/topsi/AskTopsiButton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import { organizationsApi } from '@/lib/api';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys, organizationKeys } from '@/lib/query-keys';
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

// ── CallSchedulingSection ────────────────────────────────────────────────────

const CALL_METHODS = ['Phone', 'Video', 'In-Person'] as const;
const CALL_STATUSES = ['scheduled', 'completed', 'cancelled'] as const;

function CallSchedulingSection({
  deal,
  customFields,
  invalidateKanban,
}: {
  deal: CrmDealWithContact;
  customFields: Record<string, unknown>;
  invalidateKanban: () => void;
}) {
  const [editing, setEditing] = useState<'discovery' | 'presentation' | null>(null);

  const saveMutation = useMutation({
    mutationFn: (fields: Record<string, unknown>) => {
      const merged = { ...customFields, ...fields };
      return crmDealsApi.updateDeal(deal.id, {
        custom_fields: merged,
      });
    },
    onSuccess: () => {
      toast.success('Call schedule updated');
      setEditing(null);
      invalidateKanban();
    },
    onError: () => toast.error('Failed to save call schedule'),
  });

  const renderCallRow = (
    type: 'discovery' | 'presentation',
    label: string,
    icon: React.ElementType,
  ) => {
    const Icon = icon;
    const dateKey = `${type}_call_date`;
    const methodKey = `${type}_call_method`;
    const statusKey = `${type}_call_status`;
    const date = customFields[dateKey] as string | undefined;
    const method = (customFields[methodKey] as string) || 'Video';
    const status = (customFields[statusKey] as string) || 'scheduled';
    const isEditing = editing === type;

    if (isEditing) {
      return (
        <Card className="bg-muted/30 border-primary/30">
          <CardContent className="p-3 space-y-2">
            <div className="flex items-center gap-2 text-sm font-medium">
              <Icon className="h-3.5 w-3.5" />
              {label}
            </div>
            <div className="grid grid-cols-2 gap-2">
              <div>
                <label className="text-[10px] text-muted-foreground uppercase">Date</label>
                <input
                  type="date"
                  defaultValue={date || ''}
                  className="w-full h-8 px-2 text-sm border rounded bg-background"
                  id={`${type}-date`}
                />
              </div>
              <div>
                <label className="text-[10px] text-muted-foreground uppercase">Method</label>
                <select
                  defaultValue={method}
                  className="w-full h-8 px-2 text-sm border rounded bg-background"
                  id={`${type}-method`}
                >
                  {CALL_METHODS.map((m) => (
                    <option key={m} value={m}>{m}</option>
                  ))}
                </select>
              </div>
            </div>
            <div>
              <label className="text-[10px] text-muted-foreground uppercase">Status</label>
              <select
                defaultValue={status}
                className="w-full h-8 px-2 text-sm border rounded bg-background"
                id={`${type}-status`}
              >
                {CALL_STATUSES.map((s) => (
                  <option key={s} value={s}>{s.charAt(0).toUpperCase() + s.slice(1)}</option>
                ))}
              </select>
            </div>
            <div className="flex gap-1.5">
              <Button
                size="sm"
                className="h-7 text-xs"
                disabled={saveMutation.isPending}
                onClick={() => {
                  const dateEl = document.getElementById(`${type}-date`) as HTMLInputElement;
                  const methodEl = document.getElementById(`${type}-method`) as HTMLSelectElement;
                  const statusEl = document.getElementById(`${type}-status`) as HTMLSelectElement;
                  saveMutation.mutate({
                    [dateKey]: dateEl?.value || null,
                    [methodKey]: methodEl?.value || 'Video',
                    [statusKey]: statusEl?.value || 'scheduled',
                  });
                }}
              >
                Save
              </Button>
              <Button size="sm" variant="ghost" className="h-7 text-xs" onClick={() => setEditing(null)}>
                Cancel
              </Button>
            </div>
          </CardContent>
        </Card>
      );
    }

    return (
      <div
        className="flex items-center justify-between text-sm cursor-pointer hover:bg-muted/50 rounded px-2 py-1.5 -mx-2 transition-colors"
        onClick={() => setEditing(type)}
      >
        <div className="flex items-center gap-2">
          <Icon className="h-3.5 w-3.5 text-muted-foreground" />
          <span>{label}</span>
        </div>
        <div className="flex items-center gap-2">
          {method && date && (
            <span className="text-[10px] text-muted-foreground">{method}</span>
          )}
          <Badge
            variant="outline"
            className={cn(
              'text-[10px]',
              status === 'completed'
                ? 'text-green-500 border-green-500/30'
                : status === 'cancelled'
                  ? 'text-red-500 border-red-500/30'
                  : 'text-muted-foreground'
            )}
          >
            {date || 'Not scheduled'}
          </Badge>
        </div>
      </div>
    );
  };

  return (
    <div>
      <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 flex items-center gap-1.5">
        <Phone className="h-3 w-3" /> Call Scheduling
      </h4>
      <Card className="bg-muted/30 border-border/60">
        <CardContent className="p-3 space-y-1">
          {renderCallRow('discovery', 'Discovery Call', Phone)}
          {renderCallRow('presentation', 'Presentation Call', Video)}
        </CardContent>
      </Card>
    </div>
  );
}

// ── OverviewTab ──────────────────────────────────────────────────────────────

interface OverviewTabProps {
  deal: CrmDealWithContact;
  stageColor: string;
  onConvert: () => void;
  orgId?: string;
}

export function OverviewTab({
  deal,
  stageColor,
  onConvert,
  orgId,
}: OverviewTabProps) {
  const navigate = useNavigate();
  const taskTotal = deal.task_total ?? 0;
  const currentStage = (deal.stage ?? '').toLowerCase().replace(/\s+/g, '_');

  const effectiveOrgId = orgId || deal.organization_id;
  const { data: orgData } = useQuery({
    queryKey: organizationKeys.orgData(effectiveOrgId!),
    queryFn: () => organizationsApi.getById(effectiveOrgId!),
    enabled: !!effectiveOrgId,
    staleTime: 5 * 60 * 1000,
  });
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

  let sourceInfo: { dataSourceId?: string; workflowRunId?: string } | null =
    null;
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

  const qc = useQueryClient();
  const [contextText, setContextText] = useState(deal.description ?? '');
  const [editingContext, setEditingContext] = useState(false);

  const invalidateKanban = () => {
    qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
    qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
    qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
  };

  const saveContext = useMutation({
    mutationFn: () =>
      crmDealsApi.updateDeal(deal.id, { description: contextText }),
    onSuccess: () => {
      toast.success('Context saved');
      setEditingContext(false);
      invalidateKanban();
    },
    onError: () => toast.error('Failed to save context — please try again.'),
  });

  const toggleExpedite = useMutation({
    mutationFn: () =>
      crmDealsApi.updateDeal(deal.id, { expedited: deal.expedited ? 0 : 1 }),
    onSuccess: () => {
      invalidateKanban();
    },
  });

  // Parse call scheduling from custom_fields
  const customFields = (() => {
    try {
      return typeof deal.custom_fields === 'string'
        ? JSON.parse(deal.custom_fields)
        : (deal.custom_fields ?? {});
    } catch {
      return {};
    }
  })();

  return (
    <div className="p-5 space-y-5">
      {/* Operator Context — "Tell us about this lead" */}
      <div>
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2 flex items-center gap-1.5">
          <MessageSquare className="h-3 w-3" /> Operator Context
        </h4>
        {editingContext ? (
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2">
              <Textarea
                value={contextText}
                onChange={(e) => setContextText(e.target.value)}
                placeholder="Who is this person? What does their company do? What's the opportunity? Add any background, relationship context, or budget signals…"
                className="min-h-[100px] text-sm"
              />
              <div className="flex gap-1.5">
                <Button
                  size="sm"
                  className="h-7 text-xs gap-1"
                  onClick={() => saveContext.mutate()}
                  disabled={saveContext.isPending}
                >
                  Save Context
                </Button>
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 text-xs"
                  onClick={() => {
                    setEditingContext(false);
                    setContextText(deal.description ?? '');
                  }}
                >
                  Cancel
                </Button>
              </div>
            </CardContent>
          </Card>
        ) : deal.description ? (
          <Card
            className="bg-muted/30 border-border/60 cursor-pointer hover:border-primary/40 transition-colors"
            onClick={() => setEditingContext(true)}
          >
            <CardContent className="p-3">
              <p className="text-sm whitespace-pre-wrap leading-relaxed">
                {deal.description}
              </p>
              <p className="text-[10px] text-muted-foreground mt-2">
                Click to edit
              </p>
            </CardContent>
          </Card>
        ) : (
          <Card
            className="bg-muted/30 border-dashed border-amber-500/30 cursor-pointer hover:border-amber-500/60 transition-colors"
            onClick={() => setEditingContext(true)}
          >
            <CardContent className="p-3 text-center">
              <p className="text-sm text-muted-foreground">
                No context yet — add notes about this lead
              </p>
              <p className="text-xs text-amber-500 mt-1">
                Required before advancing from Intel
              </p>
            </CardContent>
          </Card>
        )}
      </div>

      {/* Expedite Toggle */}
      <div className="flex items-center justify-between px-1">
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <Zap className="h-3 w-3" />
          <span>Expedite</span>
        </div>
        <Switch
          checked={!!deal.expedited}
          onCheckedChange={() => toggleExpedite.mutate()}
        />
      </div>

      {/* Call Scheduling (Discovery / Proposal / Present stages) */}
      {(currentStage === 'discovery' ||
        currentStage === 'proposal' ||
        currentStage === 'present') && (
        <CallSchedulingSection deal={deal} customFields={customFields} invalidateKanban={invalidateKanban} />
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
                  style={{
                    width: `${deal.probability}%`,
                    backgroundColor: stageColor,
                  }}
                />
              </div>
            ) : undefined
          }
        />
        <MetricCard
          label="Deal Value"
          value={
            deal.amount
              ? deal.amount.toLocaleString('en-US', {
                  style: 'currency',
                  currency: deal.currency || 'USD',
                  maximumFractionDigits: 0,
                })
              : '—'
          }
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
                  <span className="text-sm font-medium truncate">
                    {deal.project_name}
                  </span>
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
                        taskPct === 100
                          ? 'text-green-600'
                          : 'text-muted-foreground'
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
                {deal.company_id && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-[10px] px-1 ml-auto gap-0.5"
                    asChild
                  >
                    <Link to={`/companies/${deal.company_id}`}>
                      Co. Profile <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Organization */}
      {effectiveOrgId && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">
            Organization
          </h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <Users className="h-3.5 w-3.5 text-primary shrink-0" />
                  <span className="text-sm font-medium truncate">
                    {orgData?.name ?? 'Loading…'}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-6 text-[10px] px-1.5 shrink-0 gap-0.5"
                  asChild
                >
                  <Link to={`/organizations/${effectiveOrgId}`}>
                    Open <ExternalLink className="h-2.5 w-2.5" />
                  </Link>
                </Button>
              </div>
              {deal.contact_company && (
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <Building2 className="h-3 w-3 shrink-0" />
                  {deal.company_id ? (
                    <Link
                      to={`/companies/${deal.company_id}`}
                      className="truncate hover:text-primary transition-colors"
                    >
                      {deal.contact_company}
                    </Link>
                  ) : (
                    <span className="truncate">{deal.contact_company}</span>
                  )}
                  {deal.company_id && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-5 text-[10px] px-1 ml-auto gap-0.5 shrink-0"
                      asChild
                    >
                      <Link to={`/companies/${deal.company_id}`}>
                        Profile <ExternalLink className="h-2.5 w-2.5" />
                      </Link>
                    </Button>
                  )}
                </div>
              )}
              {deal.project_id && (
                <div className="flex items-center gap-2 text-xs text-muted-foreground">
                  <FolderKanban className="h-3 w-3 shrink-0" />
                  <span className="truncate">
                    {deal.project_name ?? 'Linked project'}
                  </span>
                  <Button
                    variant="ghost"
                    size="sm"
                    className="h-5 text-[10px] px-1 ml-auto gap-0.5 shrink-0"
                    asChild
                  >
                    <Link to={`/organizations/${effectiveOrgId}`}>
                      CRM <ExternalLink className="h-2.5 w-2.5" />
                    </Link>
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}

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
                {formatDistanceToNow(new Date(deal.last_activity_at), {
                  addSuffix: true,
                })}
              </span>
            </div>
          )}
          {deal.expected_close_date && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <Clock className="h-3.5 w-3.5 shrink-0" />
              <span className="text-muted-foreground/70">Expected close</span>
              <span className="ml-auto text-foreground">
                {new Date(deal.expected_close_date).toLocaleDateString(
                  'en-US',
                  {
                    month: 'short',
                    day: 'numeric',
                    year: 'numeric',
                  }
                )}
              </span>
            </div>
          )}
          {deal.actual_close_date && (deal.won_at || deal.lost_reason || ['closed won', 'closed lost', 'closed_won', 'closed_lost'].includes(currentStage)) && (
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
            <span>Imported via workflow</span>
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="space-y-2 pt-2 border-t">
        {deal.project_id ? (
          <Button
            size="sm"
            variant="outline"
            className="w-full gap-1.5"
            onClick={() => navigate(`/projects/${deal.project_id}/tasks`)}
          >
            <FolderKanban className="h-3.5 w-3.5" />
            View Project Board
          </Button>
        ) : (
          <Button size="sm" className="w-full gap-1.5" onClick={onConvert}>
            <Rocket className="h-3.5 w-3.5" />
            Convert to Project
          </Button>
        )}
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
