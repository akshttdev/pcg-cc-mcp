import NiceModal from '@ebay/nice-modal-react';
import { Bot, DollarSign, Loader2, Plus,Settings, Target, User, Users } from 'lucide-react';
import { useEffect,useMemo, useRef, useState } from 'react';
import { dealCard as dealTid,pipeline as tid } from 'shared/testids';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import {
  type DragEndEvent,
  KanbanBoard,
  KanbanCard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
} from '@/components/ui/shadcn-io/kanban';
import { Skeleton } from '@/components/ui/skeleton';
import { Tooltip, TooltipContent, TooltipProvider,TooltipTrigger } from '@/components/ui/tooltip';
import { useQueryClient } from '@tanstack/react-query';

import { useCreateDeal, useCrmKanban, useCrmPipelineByType, useDeleteDeal,useMoveDeal, useOrgCrmKanban, useOrgCrmPipelineByType, useUpdateDeal } from '@/hooks/useCrmPipeline';
import { resolveApiUrl } from '@/lib/api';
import { crmKeys } from '@/lib/query-keys';
import { useProjectBoardProgress } from '@/hooks/useProjectBoardProgress';
import { formatCurrencyFull } from '@/lib/formatters';
import type { CreateCrmDeal, CrmDealWithContact, PipelineType, UpdateCrmDeal } from '@/types/crm';

import { CrmDealCard } from './CrmDealCard';
import { CrmDealDetailPanel } from './CrmDealDetailPanel';
import { CrmDealForm } from './CrmDealForm';

interface CrmPipelineBoardProps {
  projectId?: string;
  orgId?: string;
  pipelineType: PipelineType;
  title?: string;
  onSettingsClick?: () => void;
}

type StageOwner = {
  label: string;
  type: 'agent' | 'human' | 'team';
};

function getStageOwner(stageName: string, stageConfig?: string): StageOwner | null {
  // Check stage_config first (data-driven)
  if (stageConfig) {
    try {
      const config = JSON.parse(stageConfig);
      if (config.stage_owner) return config.stage_owner;
    } catch { /* fall through to hardcoded */ }
  }

  // Hardcoded fallback
  const name = stageName.toLowerCase();
  // --- 9-Stage Dealflow (Acquisition/Sales) ---
  if (name === 'intel') return { label: 'Scout', type: 'agent' };
  if (name === 'business analysis') return { label: 'Astra', type: 'agent' };
  if (name === 'discovery') return { label: 'Account Manager + Nora', type: 'team' };
  if (name === 'proposal') return { label: 'Cash', type: 'agent' };
  if (name === 'polish') return { label: 'Lux', type: 'agent' };
  if (name === 'present') return { label: 'Account Manager', type: 'human' };
  if (name === 'follow up') return { label: 'Nora + AM', type: 'team' };
  if (name === 'won') return { label: 'Team', type: 'team' };
  if (name === 'lost') return { label: 'Account Manager', type: 'human' };
  // --- Delivery pipeline ---
  if (name === 'onboarding') return { label: 'PM', type: 'human' };
  if (name === 'in production') return { label: 'Team', type: 'team' };
  if (name === 'review') return { label: 'PM + Client', type: 'team' };
  if (name === 'final delivery') return { label: 'PM', type: 'human' };
  // --- Conferences pipeline ---
  if (name === 'researching') return { label: 'Nora', type: 'agent' };
  if (name === 'applied') return { label: 'AM', type: 'human' };
  if (name === 'in discussion') return { label: 'AM', type: 'human' };
  if (name === 'confirmed') return { label: 'Team', type: 'team' };
  // --- Legacy / other ---
  if (name === 'lead') return { label: 'Scout', type: 'agent' };
  if (name === 'build proposal') return { label: 'Cash', type: 'agent' };
  if (name === 'proposal meeting') return { label: 'Account Manager', type: 'human' };
  if (name === 'closed won') return { label: 'Team', type: 'team' };
  return null;
}

function StageOwnerBadge({ owner }: { owner: StageOwner }) {
  const Icon = owner.type === 'agent' ? Bot : owner.type === 'team' ? Users : User;
  const typeLabel = owner.type === 'agent' ? 'AI agent' : owner.type === 'team' ? 'Team' : 'Human';
  return (
    <span
      className="inline-flex items-center gap-1 text-xs text-muted-foreground font-normal cursor-default"
      title={`${owner.label} — ${typeLabel}`}
    >
      <Icon className="h-2.5 w-2.5" />
      {owner.label}
    </span>
  );
}

const STAGE_DESCRIPTIONS: Record<string, string> = {
  // 9-Stage Dealflow
  'intel': 'Scout runs Phase I — Who-Is on person + company. Person intel wiki & company intel wiki auto-generated.',
  'business analysis': 'Astra runs Phase II — comprehensive business research. Business Report Artifact generated, discovery call scheduled.',
  'discovery': 'Discovery call with the prospect. Nora auto-links transcript. Out-of-order flow triggers Scout if no intel exists.',
  'proposal': 'Cash generates AI proposal from full knowledge graph — business report, discovery transcript, intel wikis. Operator reviews + approves.',
  'polish': 'Lux generates branded sales deck from approved proposal + org & client brand guides. Operator reviews before presenting.',
  'present': 'Present deck to client on live call, then send invoice and confirm payment.',
  'present & invoice': 'Present deck to client on live call, then send invoice and confirm payment.',
  'follow up': 'Post-presentation follow-up. Nora sends follow-up comms. Close out as Won or Lost.',
  'won': 'Deal closed. Automation: client record + project + tasks from proposal deliverables auto-created.',
  'lost': 'Deal closed lost. Reason recorded for future learning. Deal retained for re-engagement.',
  // Delivery pipeline
  'onboarding': 'Client onboarding and project kickoff.',
  'in production': 'Active production work by the team.',
  'review': 'PM and client reviewing deliverables.',
  'final delivery': 'Packaging and delivering final assets.',
  // Conferences pipeline
  'researching': 'AI researching relevant conferences and events.',
  'applied': 'Application submitted to the conference.',
  'in discussion': 'Active discussions with conference organizers.',
  'confirmed': 'Attendance or sponsorship confirmed.',
  // Legacy fallbacks
  'lead': 'New inbound lead — Scout qualifies and enriches contact data.',
  'build proposal': 'Cash generating AI proposal from full knowledge graph.',
  'proposal meeting': 'Formal presentation of the proposal to the client.',
};

export function CrmPipelineBoard({
  projectId,
  orgId,
  pipelineType,
  title,
  onSettingsClick,
}: CrmPipelineBoardProps) {
  const [selectedDeal, setSelectedDeal] = useState<CrmDealWithContact | null>(null);
  const [selectedDealStage, setSelectedDealStage] = useState<string>('');
  const [formOpen, setFormOpen] = useState(false);
  const [editingDeal, setEditingDeal] = useState<CrmDealWithContact | undefined>();
  const [initialStageId, setInitialStageId] = useState<string | undefined>();

  const isOrgMode = !!orgId;
  const queryClient = useQueryClient();

  // SSE subscription for real-time pipeline updates
  const eventSourceRef = useRef<EventSource | null>(null);
  useEffect(() => {
    const url = resolveApiUrl(`/api/events/pipeline?org_id=${orgId || ''}`);
    const es = new EventSource(url);
    eventSourceRef.current = es;

    es.addEventListener('pipeline_changed', () => {
      queryClient.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      queryClient.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
    });

    return () => {
      es.close();
      eventSourceRef.current = null;
    };
  }, [orgId, queryClient]);

  const projectPipeline = useCrmPipelineByType(projectId || '', pipelineType);
  const orgPipeline = useOrgCrmPipelineByType(orgId || '', pipelineType);
  const { data: pipeline, isLoading: isPipelineLoading } = isOrgMode ? orgPipeline : projectPipeline;

  const projectKanban = useCrmKanban(!isOrgMode ? pipeline?.id : undefined);
  const orgKanbanResult = useOrgCrmKanban(orgId || '', isOrgMode ? pipeline?.id : undefined);
  const { data: kanbanData, isLoading: isKanbanLoading, isRefetching } =
    isOrgMode ? orgKanbanResult : projectKanban;

  // Sync selectedDeal with latest kanban data (e.g. after proposal generation refetch)
  useEffect(() => {
    if (!selectedDeal || !kanbanData?.stages) return;
    for (const stageData of kanbanData.stages) {
      const updated: CrmDealWithContact | undefined = stageData.deals.find((d) => d.id === selectedDeal.id);
      if (updated && updated !== selectedDeal) {
        setSelectedDeal(updated);
        return;
      }
    }
  }, [kanbanData, selectedDeal]);

  const moveDeal = useMoveDeal();
  const createDeal = useCreateDeal();
  const updateDeal = useUpdateDeal();
  const deleteDeal = useDeleteDeal();

  const { getProgressForStage } = useProjectBoardProgress(
    pipelineType === 'delivery' ? projectId : undefined
  );

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || !kanbanData) return;

    const dealId = active.id.toString();
    const targetStageId = over.id.toString();

    let deal: CrmDealWithContact | undefined;
    for (const stageData of kanbanData.stages) {
      deal = stageData.deals.find((d) => d.id === dealId);
      if (deal) break;
    }

    if (!deal || deal.crm_stage_id === targetStageId) return;

    const targetStage = kanbanData.stages.find((s) => s.stage.id === targetStageId);
    const newPosition = targetStage?.deals.length ?? 0;

    moveDeal.mutate({ dealId, data: { stage_id: targetStageId, position: newPosition } });
  };

  const handleAddDeal = (stageId?: string) => {
    setEditingDeal(undefined);
    setInitialStageId(stageId);
    setFormOpen(true);
  };

  const handleEditDeal = (deal: CrmDealWithContact) => {
    setEditingDeal(deal);
    setInitialStageId(undefined);
    setFormOpen(true);
  };

  const handleDeleteDeal = async (deal: CrmDealWithContact) => {
    const result = await NiceModal.show('confirm', {
      title: 'Delete Deal',
      message: `Are you sure you want to delete "${deal.name}"? This action cannot be undone.`,
      confirmText: 'Delete',
      variant: 'destructive',
    });

    if (result === 'confirmed') {
      try {
        await deleteDeal.mutateAsync(deal.id);
        toast.success(`"${deal.name}" has been deleted.`);
      } catch {
        toast.error('Failed to delete deal.');
      }
    }
  };

  const handleFormSubmit = async (data: CreateCrmDeal | UpdateCrmDeal) => {
    if (editingDeal) {
      await updateDeal.mutateAsync({ id: editingDeal.id, data: data as UpdateCrmDeal });
      toast.success('Deal updated.');
    } else {
      await createDeal.mutateAsync(data as CreateCrmDeal);
    }
  };

  const stages = useMemo(() => kanbanData?.stages.map((s) => s.stage) ?? [], [kanbanData]);

  const totalDeals = useMemo(
    () => kanbanData?.stages.reduce((sum, s) => sum + s.deals.length, 0) ?? 0,
    [kanbanData]
  );

  const totalAmount = useMemo(
    () => kanbanData?.stages.reduce((sum, s) => sum + s.total_amount, 0) ?? 0,
    [kanbanData]
  );

  if (isPipelineLoading || (isKanbanLoading && !kanbanData)) {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center justify-between p-4 border-b">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-9 w-24" />
        </div>
        <div className="flex-1 flex divide-x border-x mt-4 mx-4 rounded-lg overflow-hidden">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="flex-1 p-3 space-y-3">
              <Skeleton className="h-10 rounded-lg" />
              <Skeleton className="h-28 rounded-lg" />
              <Skeleton className="h-20 rounded-lg" />
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (!pipeline || !kanbanData) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-muted-foreground gap-3 py-12">
        <p className="text-lg font-medium">No pipeline configured</p>
        <p className="text-sm text-center max-w-md">
          This organization doesn&apos;t have a deal pipeline set up yet.
          Create one to start tracking deals through your sales process.
        </p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between p-5 border-b glass-strong shrink-0">
        <div className="flex items-center gap-3">
          <div className="section-header-icon !w-8 !h-8 !rounded-lg">
            <DollarSign className="h-4 w-4" />
          </div>
          <div>
            <h1 className="text-lg sm:text-xl font-bold">{title || kanbanData.pipeline_name}</h1>
            <div className="flex items-center gap-3 text-sm text-muted-foreground">
              <span>{totalDeals} deals</span>
              <span className="flex items-center gap-1">
                <DollarSign className="h-3.5 w-3.5" />
                {formatCurrencyFull(totalAmount)}
              </span>
              {isRefetching && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={() => handleAddDeal()} size="sm" data-testid={tid.addDeal}>
            Add Deal
          </Button>
          {onSettingsClick && (
            <IconButton
              variant="outline" className="h-8 w-8" onClick={onSettingsClick}
              icon={Settings}
              label="Pipeline settings"
              data-testid={tid.settings}
            />
          )}
        </div>
      </div>

      {/* Kanban Board */}
      <ScrollArea className="flex-1">
        <TooltipProvider delayDuration={300}>
        <KanbanProvider onDragEnd={handleDragEnd}>
          {kanbanData.stages.map((stageData) => {
            const stage = stageData.stage;
            const owner = getStageOwner(stage.name, stage.stage_config);
            const progress = pipelineType === 'delivery'
              ? getProgressForStage(stage.name)
              : undefined;

            return (
              <KanbanBoard key={stage.id} id={stage.id} data-testid={tid.stageColumn(stage.name)}>
                <KanbanHeader>
                  <div
                    className="sticky top-0 z-20 flex shrink-0 flex-col gap-1 p-3 border-b border-dashed bg-background"
                    style={{
                      backgroundImage: `linear-gradient(${stage.color}08, ${stage.color}08)`,
                    }}
                  >
                    <div className="flex items-center gap-2">
                      <div
                        className="h-2 w-2 rounded-full shrink-0"
                        style={{
                          backgroundColor: stage.color,
                          boxShadow: `0 0 6px 1px ${stage.color}60`,
                        }}
                      />
                      {STAGE_DESCRIPTIONS[stage.name.toLowerCase()] ? (
                          <Tooltip>
                            <TooltipTrigger asChild>
                              <p className="m-0 text-sm font-medium flex-1 truncate cursor-help border-b border-dashed border-muted-foreground/30">
                                {stage.name}
                              </p>
                            </TooltipTrigger>
                            <TooltipContent side="bottom" className="max-w-[220px] text-xs">
                              <p>{STAGE_DESCRIPTIONS[stage.name.toLowerCase()]}</p>
                              {owner && (
                                <p className="mt-1 text-muted-foreground">Managed by: {owner.label}</p>
                              )}
                            </TooltipContent>
                          </Tooltip>
                      ) : (
                        <p className="m-0 text-sm font-medium flex-1 truncate">{stage.name}</p>
                      )}
                      <span className="text-xs text-muted-foreground tabular-nums">
                        {stageData.deals.length}
                      </span>
                    </div>
                    <div className="flex items-center justify-between pl-4">
                      {owner ? <StageOwnerBadge owner={owner} /> : <span />}
                      {stageData.total_amount > 0 && (
                        <span className="text-xs text-muted-foreground">
                          {formatCurrencyFull(stageData.total_amount)}
                        </span>
                      )}
                    </div>
                  </div>
                </KanbanHeader>
                <KanbanCards>
                  {stageData.deals.map((deal, index) => {
                    const boardProgressInfo = progress && progress.totalAssets > 0
                      ? {
                          boardName: progress.boardName,
                          completedAssets: progress.completedAssets,
                          totalAssets: progress.totalAssets,
                          percentage: progress.percentage,
                        }
                      : undefined;

                    return (
                      <KanbanCard
                        key={deal.id}
                        id={deal.id}
                        name={deal.name}
                        index={index}
                        parent={stage.id}
                        data-testid={dealTid.card(deal.id)}
                        onClick={() => { setSelectedDeal(deal); setSelectedDealStage(stage.name); }}
                        isOpen={selectedDeal?.id === deal.id}
                        className="mx-2 my-1.5 p-0 rounded-lg border border-border/60 hover:border-border hover:shadow-sm transition-all"
                      >
                        <CrmDealCard
                          deal={deal}
                          stageName={stage.name}
                          stageColor={stage.color}
                          onEdit={handleEditDeal}
                          onDelete={handleDeleteDeal}
                          onMoveTo={(d, stageId) => {
                            const targetSD = kanbanData.stages.find((s) => s.stage.id === stageId);
                            moveDeal.mutate({ dealId: d.id, data: { stage_id: stageId, position: targetSD?.deals.length ?? 0 } });
                          }}
                          stages={stages.map((s) => ({ id: s.id, name: s.name }))}
                          boardProgress={boardProgressInfo}
                        />
                      </KanbanCard>
                    );
                  })}
                  {stageData.deals.length === 0 && totalDeals === 0 && stageData.stage.id === kanbanData.stages[0]?.stage.id ? (
                    <div className="mx-2 my-3 p-4 rounded-lg border border-dashed border-primary/30 bg-primary/5 text-center space-y-2">
                      <Target className="h-8 w-8 mx-auto text-primary/40" />
                      <p className="text-sm font-medium">Start your pipeline</p>
                      <p className="text-xs text-muted-foreground">
                        Add your first deal to start tracking prospects through your sales process.
                      </p>
                      <Button size="sm" variant="default" onClick={() => handleAddDeal(stageData.stage.id)}>
                        <Plus className="h-3.5 w-3.5 mr-1" /> Add Deal
                      </Button>
                    </div>
                  ) : stageData.deals.length === 0 ? (
                    <div className="mx-2 my-3 py-8 rounded-lg border border-dashed border-border/40 text-center text-xs text-muted-foreground/40">
                      No deals
                    </div>
                  ) : null}
                  <div className="px-2 py-1.5">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="w-full h-7 text-xs text-muted-foreground hover:text-foreground justify-start gap-1.5"
                      onClick={() => handleAddDeal(stage.id)}
                    >
                      + Add deal
                    </Button>
                  </div>
                </KanbanCards>
              </KanbanBoard>
            );
          })}
        </KanbanProvider>
        </TooltipProvider>
        <ScrollBar orientation="horizontal" />
      </ScrollArea>

      {/* Deal Form Dialog */}
      {pipeline && (
        <CrmDealForm
          open={formOpen}
          onOpenChange={setFormOpen}
          organizationId={orgId || pipeline.organization_id || ''}
          pipelineId={pipeline.id}
          stages={stages}
          deal={editingDeal}
          initialStageId={initialStageId}
          onSubmit={handleFormSubmit}
        />
      )}

      {/* Deal Detail Panel */}
      <CrmDealDetailPanel
        deal={selectedDeal}
        isOpen={!!selectedDeal}
        onClose={() => setSelectedDeal(null)}
        onEdit={(deal) => {
          setSelectedDeal(null);
          handleEditDeal(deal);
        }}
        onDelete={(deal) => {
          setSelectedDeal(null);
          handleDeleteDeal(deal);
        }}
        orgId={orgId}
        projectId={projectId || pipeline?.project_id}
        stageName={selectedDealStage}
        allStages={stages}
        onMoveTo={(deal, stageId) => {
          const targetSD = kanbanData.stages.find((s) => s.stage.id === stageId);
          moveDeal.mutate({ dealId: deal.id, data: { stage_id: stageId, position: targetSD?.deals.length ?? 0 } });
        }}
      />
    </div>
  );
}
