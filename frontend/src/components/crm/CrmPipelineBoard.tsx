import { useState, useMemo } from 'react';
import {
  DndContext,
  DragOverlay,
  closestCorners,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
  type DragStartEvent,
  type DragOverEvent,
  useDroppable,
} from '@dnd-kit/core';
import { ScrollArea, ScrollBar } from '@/components/ui/scroll-area';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Plus, DollarSign, Settings, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import NiceModal from '@ebay/nice-modal-react';
import { useCrmKanban, useCrmPipelineByType, useOrgCrmPipelineByType, useOrgCrmKanban, useMoveDeal, useCreateDeal, useUpdateDeal, useDeleteDeal } from '@/hooks/useCrmPipeline';
import { useProjectBoardProgress } from '@/hooks/useProjectBoardProgress';
import { CrmDealCard } from './CrmDealCard';
import { CrmDealForm } from './CrmDealForm';
import { CrmDealDetailPanel } from './CrmDealDetailPanel';
import type { PipelineType, CrmDealWithContact, CrmPipelineStage, CreateCrmDeal, UpdateCrmDeal } from '@/types/crm';

interface CrmPipelineBoardProps {
  projectId?: string;
  orgId?: string;
  pipelineType: PipelineType;
  title?: string;
  onSettingsClick?: () => void;
}

// Droppable column component
function DroppableColumn({
  stage,
  children,
  isOver,
}: {
  stage: CrmPipelineStage;
  children: React.ReactNode;
  isOver?: boolean;
}) {
  const { setNodeRef } = useDroppable({
    id: stage.id,
    data: {
      type: 'stage',
      stage,
    },
  });

  return (
    <div
      ref={setNodeRef}
      className={cn(
        'flex flex-col gap-2 min-h-[200px] p-2 rounded-lg transition-all duration-200',
        isOver && 'bg-info/5 ring-2 ring-info/20 scale-[1.01]'
      )}
    >
      {children}
    </div>
  );
}

export function CrmPipelineBoard({
  projectId,
  orgId,
  pipelineType,
  title,
  onSettingsClick,
}: CrmPipelineBoardProps) {
  const [activeDeal, setActiveDeal] = useState<CrmDealWithContact | null>(null);
  const [selectedDeal, setSelectedDeal] = useState<CrmDealWithContact | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editingDeal, setEditingDeal] = useState<CrmDealWithContact | undefined>();
  const [initialStageId, setInitialStageId] = useState<string | undefined>();

  const isOrgMode = !!orgId;

  // Fetch pipeline by type — org-scoped or project-scoped
  const projectPipeline = useCrmPipelineByType(
    projectId || '',
    pipelineType
  );
  const orgPipeline = useOrgCrmPipelineByType(
    orgId || '',
    pipelineType
  );
  const { data: pipeline, isLoading: isPipelineLoading } = isOrgMode
    ? orgPipeline
    : projectPipeline;

  // Fetch Kanban data once we have the pipeline — org-scoped or project-scoped
  const projectKanban = useCrmKanban(
    !isOrgMode ? pipeline?.id : undefined
  );
  const orgKanbanResult = useOrgCrmKanban(
    orgId || '',
    isOrgMode ? pipeline?.id : undefined
  );
  const {
    data: kanbanData,
    isLoading: isKanbanLoading,
    isRefetching,
  } = isOrgMode ? orgKanbanResult : projectKanban;

  // Mutations
  const moveDeal = useMoveDeal();
  const createDeal = useCreateDeal();
  const updateDeal = useUpdateDeal();
  const deleteDeal = useDeleteDeal();

  // Board progress for delivery pipeline deals
  const { getProgressForStage } = useProjectBoardProgress(
    pipelineType === 'delivery' ? projectId : undefined
  );

  // Configure sensors for drag and drop
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: {
        distance: 5, // 5px movement required before drag starts
      },
    }),
    useSensor(KeyboardSensor)
  );

  // Find the stage a deal is currently over
  const [overId, setOverId] = useState<string | null>(null);

  const handleDragStart = (event: DragStartEvent) => {
    const { active } = event;
    const deal = active.data.current?.deal as CrmDealWithContact;
    setActiveDeal(deal);
  };

  const handleDragOver = (event: DragOverEvent) => {
    setOverId(event.over?.id?.toString() ?? null);
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    setActiveDeal(null);
    setOverId(null);

    if (!over || !kanbanData) return;

    const dealId = active.id.toString();
    const targetStageId = over.id.toString();

    // Find the deal
    let deal: CrmDealWithContact | undefined;
    for (const stageData of kanbanData.stages) {
      deal = stageData.deals.find((d) => d.id === dealId);
      if (deal) break;
    }

    if (!deal) return;

    // If dropped on the same stage, don't do anything
    if (deal.crm_stage_id === targetStageId) return;

    // Calculate new position (add to end of target stage)
    const targetStage = kanbanData.stages.find((s) => s.stage.id === targetStageId);
    const newPosition = targetStage?.deals.length ?? 0;

    // Move the deal
    moveDeal.mutate({
      dealId,
      data: {
        stage_id: targetStageId,
        position: newPosition,
      },
    });
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
      } catch (error) {
        toast.error('Failed to delete deal.');
      }
    }
  };

  const handleFormSubmit = async (data: CreateCrmDeal | UpdateCrmDeal) => {
    if (editingDeal) {
      await updateDeal.mutateAsync({ id: editingDeal.id, data: data as UpdateCrmDeal });
      toast.success('The deal has been updated successfully.');
    } else {
      await createDeal.mutateAsync(data as CreateCrmDeal);
      toast.success('The new deal has been added to the pipeline.');
    }
  };

  // Extract stages for the form
  const stages = useMemo(() => {
    return kanbanData?.stages.map((s) => s.stage) ?? [];
  }, [kanbanData]);

  // Calculate totals
  const totalDeals = useMemo(() => {
    return kanbanData?.stages.reduce((sum, s) => sum + s.deals.length, 0) ?? 0;
  }, [kanbanData]);

  const totalAmount = useMemo(() => {
    return kanbanData?.stages.reduce((sum, s) => sum + s.total_amount, 0) ?? 0;
  }, [kanbanData]);

  const formatCurrency = (amount: number) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      minimumFractionDigits: 0,
      maximumFractionDigits: 0,
    }).format(amount);
  };

  // Loading state
  if (isPipelineLoading || (isKanbanLoading && !kanbanData)) {
    return (
      <div className="h-full flex flex-col">
        <div className="flex items-center justify-between p-4 border-b">
          <Skeleton className="h-8 w-48" />
          <Skeleton className="h-9 w-24" />
        </div>
        <div className="flex-1 p-4">
          <div className="flex gap-4 h-full">
            {[1, 2, 3, 4].map((i) => (
              <div key={i} className="w-72 shrink-0">
                <Skeleton className="h-12 mb-3 rounded-lg" />
                <div className="space-y-2">
                  <Skeleton className="h-32 rounded-lg" />
                  <Skeleton className="h-24 rounded-lg" />
                </div>
              </div>
            ))}
          </div>
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
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between p-4 border-b glass-strong">
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
            <Plus className="h-4 w-4 mr-2" />
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
      <DndContext
        sensors={sensors}
        collisionDetection={closestCorners}
        onDragStart={handleDragStart}
        onDragOver={handleDragOver}
        onDragEnd={handleDragEnd}
      >
        <ScrollArea className="flex-1 p-4">
          <div className="flex gap-4 h-full pb-4">
            {kanbanData.stages.map((stageData) => (
              <div
                key={stageData.stage.id}
                className="pipeline-column flex flex-col"
              >
                <div className="pipeline-column-header">
                  <div className="flex items-center gap-2">
                    <div
                      className="w-2.5 h-2.5 rounded-full shadow-sm"
                      style={{
                        backgroundColor: stageData.stage.color,
                        boxShadow: `0 0 6px 1px ${stageData.stage.color}40`,
                      }}
                    />
                    <span className="text-sm font-medium">
                      {stageData.stage.name}
                    </span>
                    <Badge variant="secondary" className="text-xs h-5 px-1.5">
                      {stageData.deals.length}
                    </Badge>
                  </div>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
                    onClick={() => handleAddDeal(stageData.stage.id)}
                  >
                    <Plus className="h-3 w-3" />
                  </Button>
                </div>
                {stageData.total_amount > 0 && (
                  <div className="text-xs text-muted-foreground px-3 py-1 border-b border-border/20">
                    {formatCurrency(stageData.total_amount)}
                  </div>
                )}
                <div className="p-2 flex-1 overflow-y-auto">
                  <DroppableColumn stage={stageData.stage} isOver={overId === stageData.stage.id}>
                    {stageData.deals.map((deal) => {
                      const progress = pipelineType === 'delivery'
                        ? getProgressForStage(stageData.stage.name)
                        : undefined;
                      const boardProgressInfo = progress
                        ? {
                            boardName: progress.boardName,
                            completedAssets: progress.completedAssets,
                            totalAssets: progress.totalAssets,
                            percentage: progress.percentage,
                          }
                        : undefined;
                      return (
                        <div key={deal.id} className="group">
                          <CrmDealCard
                            deal={deal}
                            onClick={setSelectedDeal}
                            onEdit={handleEditDeal}
                            onDelete={handleDeleteDeal}
                            isDragging={activeDeal?.id === deal.id}
                            boardProgress={boardProgressInfo}
                          />
                        </div>
                      );
                    })}
                    {stageData.deals.length === 0 && (
                      <div className="text-center py-8 text-sm text-muted-foreground/50">
                        Drop deals here
                      </div>
                    )}
                  </DroppableColumn>
                </div>
              </div>
            ))}
          </div>
          <ScrollBar orientation="horizontal" />
        </ScrollArea>

        {/* Drag Overlay */}
        <DragOverlay>
          {activeDeal && (
            <div className="opacity-80">
              <CrmDealCard deal={activeDeal} isDragging />
            </div>
          )}
        </DragOverlay>
      </DndContext>

      {/* Deal Form Dialog */}
      {pipeline && (
        <CrmDealForm
          open={formOpen}
          onOpenChange={setFormOpen}
          projectId={projectId || pipeline.project_id || ''}
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
      />
    </div>
  );
}
