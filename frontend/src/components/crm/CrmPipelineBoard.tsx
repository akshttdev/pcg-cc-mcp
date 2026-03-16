import { useState, useMemo } from 'react';
import {
  KanbanBoard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
  KanbanCard,
  type DragEndEvent,
} from '@/components/ui/shadcn-io/kanban';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { DollarSign, Settings, Loader2, Bot, User, Users } from 'lucide-react';
import { toast } from 'sonner';
import NiceModal from '@ebay/nice-modal-react';
import { useCrmKanban, useCrmPipelineByType, useOrgCrmPipelineByType, useOrgCrmKanban, useMoveDeal, useCreateDeal, useUpdateDeal, useDeleteDeal } from '@/hooks/useCrmPipeline';
import { useProjectBoardProgress } from '@/hooks/useProjectBoardProgress';
import { CrmDealCard } from './CrmDealCard';
import { CrmDealForm } from './CrmDealForm';
import { CrmDealDetailPanel } from './CrmDealDetailPanel';
import type { PipelineType, CrmDealWithContact, CreateCrmDeal, UpdateCrmDeal } from '@/types/crm';

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

function getStageOwner(stageName: string): StageOwner | null {
  const name = stageName.toLowerCase();
  // --- Clients pipeline (8-stage) ---
  if (name === 'lead') return { label: 'Nora', type: 'agent' };
  if (name === 'business analysis') return { label: 'Nora', type: 'agent' };
  if (name === 'discovery') return { label: 'Account Manager', type: 'human' };
  if (name === 'build proposal') return { label: 'Topsi + PM', type: 'team' };
  if (name === 'polish') return { label: 'PM / EP', type: 'human' };
  if (name === 'proposal meeting') return { label: 'Account Manager', type: 'human' };
  // --- Acquisition pipeline (sales) ---
  if (name === 'research') return { label: 'Nora', type: 'agent' };
  if (name === 'analysis done') return { label: 'Account Manager', type: 'human' };
  if (name === 'proposal') return { label: 'AM + Topsi', type: 'team' };
  if (name === 'sent') return { label: 'Account Manager', type: 'human' };
  if (name === 'negotiation') return { label: 'Account Manager', type: 'human' };
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
  return null;
}

function StageOwnerBadge({ owner }: { owner: StageOwner }) {
  const Icon = owner.type === 'agent' ? Bot : owner.type === 'team' ? Users : User;
  return (
    <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground font-normal">
      <Icon className="h-2.5 w-2.5" />
      {owner.label}
    </span>
  );
}

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

  const projectPipeline = useCrmPipelineByType(projectId || '', pipelineType);
  const orgPipeline = useOrgCrmPipelineByType(orgId || '', pipelineType);
  const { data: pipeline, isLoading: isPipelineLoading } = isOrgMode ? orgPipeline : projectPipeline;

  const projectKanban = useCrmKanban(!isOrgMode ? pipeline?.id : undefined);
  const orgKanbanResult = useOrgCrmKanban(orgId || '', isOrgMode ? pipeline?.id : undefined);
  const { data: kanbanData, isLoading: isKanbanLoading, isRefetching } =
    isOrgMode ? orgKanbanResult : projectKanban;

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
      toast.success('Deal added to pipeline.');
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

  const formatCurrency = (amount: number) =>
    new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);

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
      <div className="h-full flex items-center justify-center text-muted-foreground">
        Pipeline not found
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      {/* Header */}
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between p-4 border-b glass-strong shrink-0">
        <div className="flex items-center gap-3">
          <div className="section-header-icon !w-8 !h-8 !rounded-lg">
            <DollarSign className="h-4 w-4" />
          </div>
          <div>
            <h1 className="text-lg sm:text-xl font-semibold">{title || kanbanData.pipeline_name}</h1>
            <div className="flex items-center gap-3 text-sm text-muted-foreground">
              <span>{totalDeals} deals</span>
              <span className="flex items-center gap-1">
                <DollarSign className="h-3.5 w-3.5" />
                {formatCurrency(totalAmount)}
              </span>
              {isRefetching && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
            </div>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={() => handleAddDeal()} size="sm">
            Add Deal
          </Button>
          {onSettingsClick && (
            <Button variant="outline" size="icon" className="h-8 w-8" onClick={onSettingsClick}>
              <Settings className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>

      {/* Kanban Board */}
      <ScrollArea className="flex-1">
        <KanbanProvider onDragEnd={handleDragEnd}>
          {kanbanData.stages.map((stageData) => {
            const stage = stageData.stage;
            const owner = getStageOwner(stage.name);
            const progress = pipelineType === 'delivery'
              ? getProgressForStage(stage.name)
              : undefined;

            return (
              <KanbanBoard key={stage.id} id={stage.id}>
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
                      <p className="m-0 text-sm font-medium flex-1 truncate">{stage.name}</p>
                      <span className="text-xs text-muted-foreground tabular-nums">
                        {stageData.deals.length}
                      </span>
                    </div>
                    <div className="flex items-center justify-between pl-4">
                      {owner ? <StageOwnerBadge owner={owner} /> : <span />}
                      {stageData.total_amount > 0 && (
                        <span className="text-[10px] text-muted-foreground">
                          {formatCurrency(stageData.total_amount)}
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
                          boardProgress={boardProgressInfo}
                        />
                      </KanbanCard>
                    );
                  })}
                  {stageData.deals.length === 0 && (
                    <div className="mx-2 my-3 py-8 rounded-lg border border-dashed border-border/40 text-center text-xs text-muted-foreground/40">
                      No deals
                    </div>
                  )}
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
      />
    </div>
  );
}
