import { useState } from 'react';
import { useNavigate, Link } from 'react-router-dom';
import { useQueryClient } from '@tanstack/react-query';
import { AskTopsiButton } from '@/components/topsi/AskTopsiButton';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from '@/components/ui/sheet';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Calendar,
  Mail,
  Building2,
  Edit,
  Trash2,
  ExternalLink,
  FolderKanban,
  Tag,
  BarChart3,
  Rocket,
  ListTodo,
  FileText,
  Workflow,
  Brain,
  CheckCircle2,
  AlertCircle,
  Loader2,
  RotateCcw,
  ThumbsUp,
  Clock,
  ChevronRight,
  ShieldCheck,
  Search,
  TrendingUp,
  DollarSign,
  User,
  CheckSquare,
  CircleDot,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { useDealClient } from '@/hooks/useCrmPipeline';
import { CrmActivityTimeline } from './CrmActivityTimeline';
import { DealConvertDialog } from './DealConvertDialog';
import { reportsApi, crmDealsApi } from '@/lib/api';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact, CrmPipelineStage } from '@/types/crm';

interface CrmDealDetailPanelProps {
  deal: CrmDealWithContact | null;
  isOpen: boolean;
  onClose: () => void;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  orgId?: string;
  projectId?: string;
  stageName?: string;
  allStages?: CrmPipelineStage[];
}

export function CrmDealDetailPanel({
  deal,
  isOpen,
  onClose,
  onEdit,
  onDelete,
  orgId,
  projectId,
  stageName,
  allStages,
}: CrmDealDetailPanelProps) {
  const [activeTab, setActiveTab] = useState('overview');
  const [convertOpen, setConvertOpen] = useState(false);

  if (!deal) return null;

  const effectiveStageName = stageName || deal.stage || '';

  return (
    <>
      <Sheet open={isOpen} onOpenChange={(open) => !open && onClose()}>
        <SheetContent className="w-full sm:max-w-xl overflow-hidden flex flex-col p-0">
          <PanelContent
            deal={deal}
            activeTab={activeTab}
            onTabChange={setActiveTab}
            onEdit={onEdit}
            onDelete={onDelete}
            onConvert={() => setConvertOpen(true)}
            orgId={orgId}
            projectId={projectId}
            stageName={effectiveStageName}
            allStages={allStages}
          />
        </SheetContent>
      </Sheet>

      <DealConvertDialog
        deal={deal}
        open={convertOpen}
        onOpenChange={setConvertOpen}
        orgId={orgId}
      />
    </>
  );
}

// ── Panel Content ─────────────────────────────────────────────────────────────

function PanelContent({
  deal,
  activeTab,
  onTabChange,
  onEdit,
  onDelete,
  onConvert,
  orgId,
  projectId,
  stageName,
  allStages,
}: {
  deal: CrmDealWithContact;
  activeTab: string;
  onTabChange: (tab: string) => void;
  onEdit: (deal: CrmDealWithContact) => void;
  onDelete: (deal: CrmDealWithContact) => void;
  onConvert: () => void;
  orgId?: string;
  projectId?: string;
  stageName: string;
  allStages?: CrmPipelineStage[];
}) {
  const initials = deal.contact_name
    ? deal.contact_name.split(' ').map((n) => n[0]).join('').toUpperCase().slice(0, 2)
    : deal.name.slice(0, 2).toUpperCase();

  const stageColor = allStages?.find(
    (s) => s.name.toLowerCase() === stageName.toLowerCase()
  )?.color || '#6B7280';

  const formatAmount = (amount: number | undefined | null) => {
    if (!amount) return null;
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: deal.currency || 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  };

  const formattedAmount = formatAmount(deal.amount);
  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;
  const intelDone = deal.intelligence_status === 'done';
  const hasActiveReview = deal.review_task_id && deal.review_task_status !== 'done' && deal.review_task_status !== 'cancelled';

  return (
    <>
      {/* Stage color top bar */}
      <div className="h-1 w-full shrink-0" style={{ backgroundColor: stageColor }} />

      {/* Header */}
      <SheetHeader className="px-5 pt-4 pb-3 border-b shrink-0">
        <div className="flex items-start gap-3">
          <Avatar className="h-11 w-11 shrink-0 mt-0.5">
            {deal.contact_avatar_url && (
              <AvatarImage src={deal.contact_avatar_url} alt={deal.contact_name || deal.name} />
            )}
            <AvatarFallback className="text-sm font-semibold" style={{
              backgroundColor: `${stageColor}22`,
              color: stageColor,
            }}>
              {initials}
            </AvatarFallback>
          </Avatar>

          <div className="min-w-0 flex-1">
            <SheetTitle className="text-base leading-tight">{deal.name}</SheetTitle>
            <SheetDescription className="text-xs mt-0.5 flex items-center gap-1.5 flex-wrap">
              {deal.contact_name && <span>{deal.contact_name}</span>}
              {deal.contact_company && (
                <>
                  {deal.contact_name && <span className="text-muted-foreground/40">·</span>}
                  <span className="flex items-center gap-0.5">
                    <Building2 className="h-3 w-3" />
                    {deal.contact_company}
                  </span>
                </>
              )}
            </SheetDescription>
          </div>

          <div className="flex items-center gap-1 shrink-0">
            <Button variant="ghost" size="icon" className="h-7 w-7" onClick={() => onEdit(deal)}>
              <Edit className="h-3.5 w-3.5" />
            </Button>
            <Button variant="ghost" size="icon" className="h-7 w-7 text-destructive hover:text-destructive" onClick={() => onDelete(deal)}>
              <Trash2 className="h-3.5 w-3.5" />
            </Button>
          </div>
        </div>

        {/* Metric pills row */}
        <div className="flex items-center gap-2 mt-2 flex-wrap">
          {formattedAmount && (
            <span className="inline-flex items-center gap-0.5 text-xs font-semibold text-green-600">
              <DollarSign className="h-3.5 w-3.5" />
              {formattedAmount}
            </span>
          )}
          {deal.probability > 0 && (
            <span className="inline-flex items-center gap-0.5 text-[11px] text-muted-foreground">
              <TrendingUp className="h-3 w-3" />
              {deal.probability}% probability
            </span>
          )}
          {taskTotal > 0 && (
            <span className={cn(
              'inline-flex items-center gap-0.5 text-[11px]',
              taskDone === taskTotal ? 'text-green-600 font-medium' : 'text-muted-foreground'
            )}>
              <CheckSquare className="h-3 w-3" />
              {taskDone}/{taskTotal} tasks
            </span>
          )}
          {deal.expected_close_date && (
            <span className="inline-flex items-center gap-0.5 text-[11px] text-muted-foreground">
              <Calendar className="h-3 w-3" />
              {new Date(deal.expected_close_date).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
            </span>
          )}
        </div>
      </SheetHeader>

      {/* Pipeline Stage Stepper */}
      <PipelineStepper currentStage={stageName} allStages={allStages} />

      {/* Tabs */}
      <Tabs value={activeTab} onValueChange={onTabChange} className="flex-1 flex flex-col min-h-0">
        <TabsList className="mx-5 mt-3 mb-0 h-9 bg-transparent p-0 border-b rounded-none justify-start gap-0 w-auto shrink-0">
          {[
            { value: 'overview', label: 'Overview' },
            { value: 'intel', label: 'Intel', dot: intelDone ? 'green' : undefined },
            { value: 'review', label: 'Review', dot: hasActiveReview ? 'amber' : undefined },
            { value: 'projects', label: 'Projects' },
            { value: 'activity', label: 'Activity' },
          ].map(({ value, label, dot }) => (
            <TabsTrigger
              key={value}
              value={value}
              className="relative h-9 rounded-none px-3 text-xs font-medium border-b-2 border-transparent data-[state=active]:border-primary data-[state=active]:text-foreground data-[state=active]:shadow-none bg-transparent"
            >
              {label}
              {dot && (
                <span className={cn(
                  'absolute top-1.5 right-1 w-1.5 h-1.5 rounded-full',
                  dot === 'green' ? 'bg-green-500' : 'bg-amber-500 animate-pulse'
                )} />
              )}
            </TabsTrigger>
          ))}
        </TabsList>

        <div className="flex-1 min-h-0 overflow-hidden">
          <TabsContent value="overview" className="h-full m-0">
            <ScrollArea className="h-full">
              <OverviewTab deal={deal} stageColor={stageColor} onConvert={onConvert} orgId={orgId} />
            </ScrollArea>
          </TabsContent>

          <TabsContent value="intel" className="h-full m-0">
            <ScrollArea className="h-full">
              <IntelTab deal={deal} />
            </ScrollArea>
          </TabsContent>

          <TabsContent value="review" className="h-full m-0">
            <ScrollArea className="h-full">
              <ReviewTab deal={deal} stageName={stageName} />
            </ScrollArea>
          </TabsContent>

          <TabsContent value="projects" className="h-full m-0">
            <ScrollArea className="h-full">
              <ProjectsTab deal={deal} orgId={orgId} />
            </ScrollArea>
          </TabsContent>

          <TabsContent value="activity" className="h-full m-0">
            <ScrollArea className="h-full">
              <ActivityTab deal={deal} projectId={projectId} />
            </ScrollArea>
          </TabsContent>
        </div>
      </Tabs>
    </>
  );
}

// ── Pipeline Stage Stepper ────────────────────────────────────────────────────

const FALLBACK_STAGES = [
  { name: 'Lead', color: '#6B7280' },
  { name: 'Business Analysis', color: '#3B82F6' },
  { name: 'Discovery', color: '#8B5CF6' },
  { name: 'Build Proposal', color: '#F59E0B' },
  { name: 'Polish', color: '#EC4899' },
  { name: 'Proposal Meeting', color: '#EF4444' },
  { name: 'Closed Won', color: '#22C55E' },
  { name: 'Closed Lost', color: '#9CA3AF' },
];

function PipelineStepper({
  currentStage,
  allStages,
}: {
  currentStage: string;
  allStages?: CrmPipelineStage[];
}) {
  const stages = allStages && allStages.length > 0
    ? allStages.map((s) => ({ name: s.name, color: s.color }))
    : FALLBACK_STAGES;

  const activeStages = stages.filter((s) => s.name.toLowerCase() !== 'closed lost');
  const isClosedLost = currentStage.toLowerCase() === 'closed lost';
  const currentIndex = stages.findIndex((s) => s.name.toLowerCase() === currentStage.toLowerCase());

  if (currentIndex === -1 && !isClosedLost) return null;

  return (
    <div className="px-5 py-2.5 border-b bg-muted/20 shrink-0">
      <div className="flex items-center gap-0">
        {activeStages.map((stage, i) => {
          const isCompleted = i < currentIndex;
          const isCurrent = i === currentIndex;

          return (
            <div key={stage.name} className="flex items-center flex-1 min-w-0">
              <div className="flex flex-col items-center flex-1 min-w-0">
                <div
                  className={cn('w-full h-1 rounded-sm transition-all', !isCurrent && (isClosedLost ? 'bg-muted' : isCompleted ? 'bg-green-500' : 'bg-muted'))}
                  style={isCurrent && !isClosedLost ? { backgroundColor: stage.color } : undefined}
                />
                {isCurrent && (
                  <span className="text-[9px] font-medium mt-0.5 text-center truncate max-w-full leading-none"
                    style={{ color: stage.color }}>
                    {stage.name}
                  </span>
                )}
              </div>
              {i < activeStages.length - 1 && (
                <ChevronRight className="h-2.5 w-2.5 text-muted-foreground/20 shrink-0" />
              )}
            </div>
          );
        })}
      </div>
      {isClosedLost && (
        <p className="text-[10px] text-muted-foreground text-center mt-1 font-medium">Closed Lost</p>
      )}
    </div>
  );
}

// ── Overview Tab ──────────────────────────────────────────────────────────────

function OverviewTab({
  deal,
  stageColor,
  onConvert,
  orgId,
}: {
  deal: CrmDealWithContact;
  stageColor: string;
  onConvert: () => void;
  orgId?: string;
}) {
  const navigate = useNavigate();
  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;
  const taskPct = taskTotal > 0 ? Math.round((taskDone / taskTotal) * 100) : 0;

  let tags: string[] = [];
  if (deal.tags) {
    try { tags = JSON.parse(deal.tags); } catch { tags = deal.tags.split(',').map(t => t.trim()).filter(Boolean); }
  }

  let sourceInfo: { dataSourceId?: string; workflowRunId?: string } | null = null;
  if (deal.custom_fields) {
    try {
      const cf = typeof deal.custom_fields === 'string' ? JSON.parse(deal.custom_fields) : deal.custom_fields;
      if (cf.source_data_source_id || cf.source_workflow_run_id) {
        sourceInfo = { dataSourceId: cf.source_data_source_id, workflowRunId: cf.source_workflow_run_id };
      }
    } catch { /* ignore */ }
  }

  return (
    <div className="p-5 space-y-5">

      {/* Description */}
      {deal.description && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Description</h4>
          <p className="text-sm whitespace-pre-wrap leading-relaxed">{deal.description}</p>
        </div>
      )}

      {/* Key metrics grid */}
      <div className="grid grid-cols-2 gap-3">
        <MetricCard
          label="Probability"
          value={deal.probability > 0 ? `${deal.probability}%` : '—'}
          icon={TrendingUp}
          accent={stageColor}
          sub={deal.probability > 0 ? (
            <div className="mt-1.5 h-1 bg-muted rounded-full overflow-hidden">
              <div className="h-full rounded-full" style={{ width: `${deal.probability}%`, backgroundColor: stageColor }} />
            </div>
          ) : undefined}
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
                  className={cn('h-full rounded-full', taskPct === 100 ? 'bg-green-500' : 'bg-blue-500')}
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
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Linked Project</h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3 space-y-2.5">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <FolderKanban className="h-4 w-4 text-primary shrink-0" />
                  <span className="text-sm font-medium truncate">{deal.project_name}</span>
                </div>
                <Button variant="ghost" size="sm" className="h-7 text-xs shrink-0 gap-1"
                  onClick={() => navigate(`/projects/${deal.project_id}/tasks`)}>
                  Open <ExternalLink className="h-3 w-3" />
                </Button>
              </div>
              {taskTotal > 0 && (
                <div className="space-y-1.5">
                  <div className="flex items-center justify-between text-xs text-muted-foreground">
                    <span className="flex items-center gap-1"><ListTodo className="h-3 w-3" />{taskDone}/{taskTotal} tasks done</span>
                    <span className={cn('font-medium', taskPct === 100 ? 'text-green-600' : 'text-muted-foreground')}>{taskPct}%</span>
                  </div>
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div
                      className={cn('h-full rounded-full transition-all', taskPct === 100 ? 'bg-green-500' : 'bg-blue-500')}
                      style={{ width: `${taskPct}%` }}
                    />
                  </div>
                  {(deal.deliverable_count ?? 0) > 0 && (
                    <p className="text-[11px] text-muted-foreground flex items-center gap-1">
                      <FileText className="h-3 w-3" />
                      {deal.deliverable_count} deliverable{(deal.deliverable_count ?? 0) !== 1 ? 's' : ''}
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
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Contact</h4>
        <Card className="bg-muted/30 border-border/60">
          <CardContent className="p-3 space-y-1.5">
            {deal.contact_name && (
              <div className="flex items-center gap-2 text-sm">
                <User className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                <span className="font-medium">{deal.contact_name}</span>
                {deal.person_id && (
                  <Button variant="ghost" size="sm" className="h-5 text-[10px] px-1 ml-auto gap-0.5" asChild>
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
        <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Timeline</h4>
        <div className="space-y-1.5 text-sm">
          <div className="flex items-center gap-2 text-muted-foreground">
            <Calendar className="h-3.5 w-3.5 shrink-0" />
            <span className="text-muted-foreground/70">Created</span>
            <span className="ml-auto text-foreground">{new Date(deal.created_at).toLocaleDateString()}</span>
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
                {new Date(deal.expected_close_date).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })}
              </span>
            </div>
          )}
          {deal.actual_close_date && (
            <div className="flex items-center gap-2 text-muted-foreground">
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-500" />
              <span className="text-muted-foreground/70">Closed</span>
              <span className="ml-auto text-foreground">{new Date(deal.actual_close_date).toLocaleDateString()}</span>
            </div>
          )}
        </div>
      </div>

      {/* Tags */}
      {tags.length > 0 && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Tags</h4>
          <div className="flex flex-wrap gap-1.5">
            {tags.map((tag) => (
              <Badge key={tag} variant="secondary" className="text-xs gap-1">
                <Tag className="h-3 w-3" />{tag}
              </Badge>
            ))}
          </div>
        </div>
      )}

      {/* Win/Lost reason */}
      {deal.win_reason && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Win Reason</h4>
          <p className="text-sm">{deal.win_reason}</p>
        </div>
      )}
      {deal.lost_reason && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Lost Reason</h4>
          <p className="text-sm">{deal.lost_reason}</p>
        </div>
      )}

      {/* Source provenance */}
      {sourceInfo && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-1.5">Source</h4>
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Workflow className="h-3.5 w-3.5 shrink-0" />
            {orgId ? (
              <button type="button" className="text-primary hover:underline text-left"
                onClick={() => {}}>Imported via workflow</button>
            ) : <span>Imported via workflow</span>}
          </div>
        </div>
      )}

      {/* Actions */}
      <div className="space-y-2 pt-2 border-t">
        <Button size="sm" className="w-full gap-1.5" onClick={onConvert}>
          <Rocket className="h-3.5 w-3.5" />Convert to Project
        </Button>
        <AskTopsiButton entityType="crm_deal" entityId={deal.id} entityName={deal.name} className="w-full" />
      </div>
    </div>
  );
}

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
          <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide">{label}</span>
        </div>
        <p className="text-sm font-semibold">{value}</p>
        {sub}
      </CardContent>
    </Card>
  );
}

// ── Review Tab ────────────────────────────────────────────────────────────────

function ReviewTab({ deal, stageName }: { deal: CrmDealWithContact; stageName?: string }) {
  const [advanceLoading, setAdvanceLoading] = useState(false);
  const [researchLoading, setResearchLoading] = useState(false);
  const [checkedItems, setCheckedItems] = useState<Set<string>>(new Set());
  const queryClient = useQueryClient();

  const hasReviewTask = !!deal.review_task_id;
  const taskDone = deal.review_task_status === 'done';
  const effectiveStage = (stageName || deal.stage || '').toLowerCase();

  const handleAdvance = async () => {
    setAdvanceLoading(true);
    try {
      await crmDealsApi.advanceDeal(deal.id);
      toast.success('Deal advanced to next stage');
      queryClient.invalidateQueries({ queryKey: ['kanban'] });
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Failed to advance deal');
    } finally {
      setAdvanceLoading(false);
    }
  };

  const handleTriggerResearch = async () => {
    if (!deal.person_id) return;
    setResearchLoading(true);
    try {
      const response = await fetch(`/api/persons/${deal.person_id}/research`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      });
      if (!response.ok) throw new Error('Research trigger failed');
      toast.success('Research triggered — Nora is gathering intel');
    } catch {
      toast.error('Failed to trigger research');
    } finally {
      setResearchLoading(false);
    }
  };

  const stageChecklist: Record<string, { item: string; description?: string }[]> = {
    'lead': [
      { item: 'Person research complete', description: 'Nora has run "Who Is" research on this contact' },
      { item: 'Company research complete', description: 'Company intelligence is available' },
      { item: 'Lead quality verified', description: 'Account Manager has confirmed lead is worth pursuing' },
    ],
    'business analysis': [
      { item: 'Business report reviewed', description: 'MBA-level analysis report has been read and verified' },
      { item: 'Market opportunity assessed', description: 'We understand their market position and opportunity' },
      { item: 'Discovery call scheduled', description: 'Meeting with prospect is booked' },
    ],
    'discovery': [
      { item: 'Discovery notes captured', description: 'Call notes are complete in the system' },
      { item: 'Pain points identified', description: 'We understand what problems they need solved' },
      { item: 'Budget range confirmed', description: 'We have a clear budget expectation' },
      { item: 'Knowledge is proposal-ready', description: 'We have enough to build a strong proposal' },
    ],
    'build proposal': [
      { item: 'Services defined and scoped', description: 'Deliverables are clearly outlined' },
      { item: 'Pricing approved internally', description: 'Pricing reviewed and signed off' },
      { item: 'Timeline is realistic', description: 'Delivery schedule is achievable' },
      { item: 'Topsi has generated proposal', description: 'AI proposal draft has been created' },
    ],
    'polish': [
      { item: 'Presentation deck complete', description: 'Slides are polished and client-ready' },
      { item: 'Report card accurate', description: 'Data and metrics are verified' },
      { item: 'Graphical quality verified', description: 'Design quality meets our standards' },
      { item: 'PM/EP sign-off received', description: 'Leadership has approved the proposal' },
    ],
    'proposal meeting': [
      { item: 'Meeting scheduled', description: 'Date and time confirmed with client' },
      { item: 'Decision maker confirmed', description: 'The right stakeholders will be present' },
      { item: 'Presentation rehearsed', description: 'Team is prepared for the pitch' },
    ],
  };

  const checklist = stageChecklist[effectiveStage] || [];
  const allChecked = checklist.length > 0 && checklist.every(({ item }) => checkedItems.has(item));

  return (
    <div className="p-5 space-y-5">
      {/* Current review task status */}
      {hasReviewTask ? (
        <Card className={cn('border-border/60', taskDone ? 'bg-green-50 dark:bg-green-950/20' : 'bg-amber-50 dark:bg-amber-950/20')}>
          <CardContent className="p-4 space-y-3">
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                {taskDone
                  ? <CheckCircle2 className="h-4 w-4 text-green-500" />
                  : <CircleDot className="h-4 w-4 text-amber-500 animate-pulse" />
                }
                <span className="text-sm font-medium">
                  {taskDone ? 'Review Complete' : 'Review In Progress'}
                </span>
              </div>
              {deal.review_task_assignee && (
                <Badge variant="outline" className="text-xs">{deal.review_task_assignee}</Badge>
              )}
            </div>
            {deal.review_task_status && !taskDone && (
              <p className="text-xs text-muted-foreground capitalize">
                Status: {deal.review_task_status.replace(/_/g, ' ')}
              </p>
            )}
            {taskDone && (
              <Button size="sm" className="w-full gap-1.5" onClick={handleAdvance} disabled={advanceLoading}>
                {advanceLoading
                  ? <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  : <ShieldCheck className="h-3.5 w-3.5" />
                }
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
            {!['closed won', 'closed lost', 'proposal meeting'].includes(effectiveStage) && (
              <p className="text-xs text-muted-foreground/70">A review task is created automatically when entering an active stage.</p>
            )}
          </CardContent>
        </Card>
      )}

      {/* Stage review checklist */}
      {checklist.length > 0 && (
        <div>
          <div className="flex items-center justify-between mb-3">
            <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
              Stage Checklist
            </h4>
            <span className="text-[10px] text-muted-foreground">
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
                  onClick={() => setCheckedItems(prev => {
                    const next = new Set(prev);
                    if (next.has(item)) next.delete(item); else next.add(item);
                    return next;
                  })}
                >
                  <div className={cn(
                    'w-4 h-4 rounded border-2 shrink-0 mt-0.5 flex items-center justify-center transition-all',
                    checked ? 'bg-green-500 border-green-500' : 'border-muted-foreground/30'
                  )}>
                    {checked && <CheckCircle2 className="h-3 w-3 text-white" />}
                  </div>
                  <div>
                    <p className={cn('text-sm font-medium', checked && 'line-through text-muted-foreground')}>{item}</p>
                    {description && <p className="text-[11px] text-muted-foreground mt-0.5">{description}</p>}
                  </div>
                </button>
              );
            })}
          </div>
          {allChecked && (
            <div className="mt-3 p-2.5 rounded-lg bg-green-50 dark:bg-green-950/20 border border-green-200 dark:border-green-800 text-center">
              <p className="text-xs font-medium text-green-700 dark:text-green-400 flex items-center justify-center gap-1.5">
                <CheckCircle2 className="h-3.5 w-3.5" />All items checked — ready to advance
              </p>
            </div>
          )}
        </div>
      )}

      {/* Quick research trigger */}
      {deal.person_id && (
        <Button variant="outline" size="sm" className="w-full gap-1.5" onClick={handleTriggerResearch} disabled={researchLoading}>
          {researchLoading
            ? <Loader2 className="h-3.5 w-3.5 animate-spin" />
            : <Search className="h-3.5 w-3.5" />
          }
          Request Additional Research
        </Button>
      )}

      {/* Intel snapshot */}
      {(deal.intelligence_summary || deal.company_intelligence_status === 'done') && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Intel Snapshot</h4>
          <div className="grid grid-cols-1 gap-2">
            {deal.intelligence_summary && (
              <Card className="bg-muted/20 border-border/50">
                <CardContent className="p-3">
                  <div className="flex items-center gap-1.5 mb-1.5">
                    <Brain className="h-3.5 w-3.5 text-blue-500" />
                    <span className="text-xs font-medium">Person Intel</span>
                  </div>
                  <p className="text-[11px] text-muted-foreground leading-relaxed line-clamp-3">
                    {deal.intelligence_summary}
                  </p>
                </CardContent>
              </Card>
            )}
            {deal.company_intelligence_status === 'done' && (
              <Card className="bg-muted/20 border-border/50">
                <CardContent className="p-3">
                  <div className="flex items-center gap-1.5 mb-1.5">
                    <Building2 className="h-3.5 w-3.5 text-purple-500" />
                    <span className="text-xs font-medium">Company Intel — {deal.contact_company}</span>
                  </div>
                  <p className="text-[11px] text-muted-foreground">Company research complete.</p>
                </CardContent>
              </Card>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

// ── Intel Tab ─────────────────────────────────────────────────────────────────

function IntelTab({ deal }: { deal: CrmDealWithContact }) {
  const [revisionNotes, setRevisionNotes] = useState('');
  const [showRevisionInput, setShowRevisionInput] = useState(false);
  const [loading, setLoading] = useState(false);

  const status = deal.intelligence_status;
  const isResearching = status === 'running' || status === 'queued';
  const isDone = status === 'done';
  const confidencePct = isDone && deal.intelligence_confidence != null
    ? Math.round(deal.intelligence_confidence * 100)
    : null;

  const handleApprove = async () => {
    if (!deal.report_id) return;
    setLoading(true);
    try {
      await reportsApi.approve(deal.report_id);
      toast.success('Report approved');
    } catch { toast.error('Failed to approve report'); }
    finally { setLoading(false); }
  };

  const handleRequestRevision = async () => {
    if (!deal.report_id) return;
    setLoading(true);
    try {
      await reportsApi.requestRevision(deal.report_id, revisionNotes);
      toast.success('Revision requested');
      setShowRevisionInput(false);
      setRevisionNotes('');
    } catch { toast.error('Failed to request revision'); }
    finally { setLoading(false); }
  };

  const handleTriggerResearch = async () => {
    if (!deal.person_id) return;
    setLoading(true);
    try {
      await fetch(`/api/persons/${deal.person_id}/research`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
      });
      toast.success('Research triggered');
    } catch { toast.error('Failed to trigger research'); }
    finally { setLoading(false); }
  };

  if (isResearching) {
    return (
      <div className="p-5 flex flex-col items-center justify-center gap-3 text-center py-12">
        <div className="w-12 h-12 rounded-full bg-blue-50 dark:bg-blue-950/30 flex items-center justify-center">
          <Loader2 className="h-6 w-6 text-blue-500 animate-spin" />
        </div>
        <div>
          <p className="text-sm font-medium">Research in progress</p>
          <p className="text-xs text-muted-foreground mt-1">Nora is gathering intelligence on this lead.</p>
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
            <Button variant="outline" size="sm" className="mt-3 gap-1.5" onClick={handleTriggerResearch} disabled={loading}>
              {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Search className="h-3.5 w-3.5" />}
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
          {isDone
            ? <CheckCircle2 className="h-4 w-4 text-green-500" />
            : <AlertCircle className="h-4 w-4 text-amber-400" />
          }
          <span className="text-sm font-medium">
            {isDone ? 'Intelligence Complete' : 'Partial Intelligence'}
          </span>
        </div>
        <div className="flex items-center gap-2">
          {confidencePct != null && (
            <Badge variant="outline" className="text-xs">{confidencePct}% confidence</Badge>
          )}
          {(deal.research_pass_count ?? 0) > 0 && (
            <span className="text-[11px] text-muted-foreground flex items-center gap-1">
              <RotateCcw className="h-3 w-3" />
              {deal.research_pass_count} pass{(deal.research_pass_count ?? 0) !== 1 ? 'es' : ''}
            </span>
          )}
        </div>
      </div>

      {/* Summary */}
      {deal.intelligence_summary && (
        <div>
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">AI Profile Summary</h4>
          <div className="p-3 rounded-lg bg-muted/30 border border-border/50">
            <p className="text-sm leading-relaxed whitespace-pre-wrap">{deal.intelligence_summary}</p>
          </div>
        </div>
      )}

      {/* Business report */}
      {deal.report_id && (
        <div>
          <div className="flex items-center justify-between mb-2">
            <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">Business Report</h4>
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
                  {deal.report_review_status === 'approved'
                    ? <CheckCircle2 className="h-4 w-4 text-green-500" />
                    : deal.report_review_status === 'rejected'
                    ? <AlertCircle className="h-4 w-4 text-red-400" />
                    : <Clock className="h-4 w-4 text-amber-400" />
                  }
                  <span className="text-sm font-medium">
                    {deal.report_review_status === 'approved' ? 'Report Approved'
                      : deal.report_review_status === 'rejected' ? 'Revision Requested'
                      : 'Pending Review'}
                  </span>
                </div>
                {deal.report_review_status !== 'approved' && (
                  <div className="flex gap-2">
                    <Button size="sm" className="h-7 text-xs flex-1 gap-1" onClick={handleApprove} disabled={loading}>
                      <ThumbsUp className="h-3 w-3" />Approve
                    </Button>
                    <Button variant="outline" size="sm" className="h-7 text-xs flex-1 gap-1"
                      onClick={() => setShowRevisionInput(!showRevisionInput)} disabled={loading}>
                      <RotateCcw className="h-3 w-3" />Revise
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
                    <Button size="sm" variant="outline" className="h-7 text-xs w-full"
                      onClick={handleRequestRevision} disabled={loading || !revisionNotes.trim()}>
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
          <h4 className="text-xs font-semibold text-muted-foreground uppercase tracking-wide mb-2">Company Intelligence</h4>
          <Card className="bg-muted/30 border-border/60">
            <CardContent className="p-3">
              <div className="flex items-center gap-2">
                <Building2 className="h-4 w-4 text-muted-foreground shrink-0" />
                <span className="text-sm font-medium">{deal.contact_company}</span>
                {deal.company_intelligence_status === 'done' && <CheckCircle2 className="h-3.5 w-3.5 text-green-500 ml-auto" />}
                {(deal.company_intelligence_status === 'running' || deal.company_intelligence_status === 'queued') && (
                  <Loader2 className="h-3.5 w-3.5 text-blue-500 animate-spin ml-auto" />
                )}
                {(!deal.company_intelligence_status || deal.company_intelligence_status === 'idle') && (
                  <AlertCircle className="h-3.5 w-3.5 text-muted-foreground/40 ml-auto" />
                )}
              </div>
              <p className="text-xs text-muted-foreground mt-1.5">
                {deal.company_intelligence_status === 'done' ? 'Company research complete'
                  : (deal.company_intelligence_status === 'running' || deal.company_intelligence_status === 'queued') ? 'Company research in progress…'
                  : 'No company research yet'}
              </p>
            </CardContent>
          </Card>
        </div>
      )}

      {/* Re-trigger */}
      {deal.person_id && !isResearching && (
        <Button variant="outline" size="sm" className="w-full gap-1.5" onClick={handleTriggerResearch} disabled={loading}>
          {loading ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Search className="h-3.5 w-3.5" />}
          {isDone ? 'Run Additional Research' : 'Trigger Research'}
        </Button>
      )}

      {deal.person_id && (
        <div className="pt-2 border-t">
          <Button variant="ghost" size="sm" className="w-full h-8 text-xs gap-1.5" asChild>
            <Link to={`/people/${deal.person_id}`}>
              <Brain className="h-3.5 w-3.5" />View Full Intelligence Profile
              <ExternalLink className="h-3 w-3 ml-auto" />
            </Link>
          </Button>
        </div>
      )}
    </div>
  );
}

// ── Projects Tab ──────────────────────────────────────────────────────────────

function ProjectsTab({ deal, orgId }: { deal: CrmDealWithContact; orgId?: string }) {
  const navigate = useNavigate();
  const { client, projects, isLoading } = useDealClient(orgId, deal);

  if (!deal.contact_company) {
    return (
      <div className="p-5 text-center py-12">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">No company linked to this deal.</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="p-5 space-y-3">
        <Skeleton className="h-4 w-32" />
        <Skeleton className="h-20 rounded-lg" />
        <Skeleton className="h-20 rounded-lg" />
      </div>
    );
  }

  if (!client) {
    return (
      <div className="p-5 text-center py-12">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">No client found for "{deal.contact_company}".</p>
      </div>
    );
  }

  if (projects.length === 0) {
    return (
      <div className="p-5 text-center py-12">
        <FolderKanban className="h-8 w-8 mx-auto mb-2 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">No projects for {client.name} yet.</p>
      </div>
    );
  }

  return (
    <div className="p-5 space-y-3">
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <Building2 className="h-3.5 w-3.5" />
        <span className="font-medium">{client.name}</span>
        <Badge variant="secondary" className="text-[10px]">{projects.length} project{projects.length !== 1 ? 's' : ''}</Badge>
      </div>
      <div className="space-y-2">
        {projects.map((project) => (
          <Card key={project.id} className="hover:shadow-sm transition-shadow border-border/60">
            <CardContent className="p-3">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <FolderKanban className="h-4 w-4 text-primary shrink-0" />
                  <p className="text-sm font-medium truncate">{project.name}</p>
                </div>
                <Button variant="ghost" size="sm" className="shrink-0 h-7 text-xs gap-1"
                  onClick={() => navigate(`/projects/${project.id}/tasks`)}>
                  Open <ExternalLink className="h-3 w-3" />
                </Button>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── Activity Tab ──────────────────────────────────────────────────────────────

function ActivityTab({ deal, projectId }: { deal: CrmDealWithContact; projectId?: string }) {
  const resolvedProjectId = projectId || deal.organization_id;
  return (
    <div className="p-5">
      <CrmActivityTimeline projectId={resolvedProjectId} dealId={deal.id} limit={30} />
    </div>
  );
}
