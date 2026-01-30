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
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { Plus, DollarSign, Settings, Loader2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { toast } from 'sonner';
import NiceModal from '@ebay/nice-modal-react';
import { useCrmKanban, useCrmPipelineByType, useMoveDeal, useCreateDeal, useUpdateDeal, useDeleteDeal } from '@/hooks/useCrmPipeline';
import { CrmDealCard } from './CrmDealCard';
import { CrmDealForm } from './CrmDealForm';
import type { PipelineType, CrmDealWithContact, CrmPipelineStage, CreateCrmDeal, UpdateCrmDeal } from '@/types/crm';

interface CrmPipelineBoardProps {
  projectId: string;
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
        'flex flex-col gap-2 min-h-[200px] p-2 rounded-lg transition-colors',
        isOver && 'bg-primary/5 ring-2 ring-primary/20'
      )}
    >
      {children}
    </div>
  );
}

export function CrmPipelineBoard({
  projectId,
  pipelineType,
  title,
  onSettingsClick,
}: CrmPipelineBoardProps) {
  const [activeDeal, setActiveDeal] = useState<CrmDealWithContact | null>(null);
  const [formOpen, setFormOpen] = useState(false);
  const [editingDeal, setEditingDeal] = useState<CrmDealWithContact | undefined>();
  const [initialStageId, setInitialStageId] = useState<string | undefined>();

  // Fetch pipeline by type
  const { data: pipeline, isLoading: isPipelineLoading } = useCrmPipelineByType(
    projectId,
    pipelineType
  );

  // Fetch Kanban data once we have the pipeline
  const {
    data: kanbanData,
    isLoading: isKanbanLoading,
    isRefetching,
  } = useCrmKanban(pipeline?.id);

  // Mutations
  const moveDeal = useMoveDeal();
  const createDeal = useCreateDeal();
  const updateDeal = useUpdateDeal();
  const deleteDeal = useDeleteDeal();

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
      <div className="flex items-center justify-between p-4 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-semibold">{title || kanbanData.pipeline_name}</h1>
          {isRefetching && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
          <div className="flex items-center gap-4 text-sm text-muted-foreground">
            <span>{totalDeals} deals</span>
            <span className="flex items-center gap-1">
              <DollarSign className="h-3.5 w-3.5" />
              {formatCurrency(totalAmount)}
            </span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button onClick={() => handleAddDeal()}>
            <Plus className="h-4 w-4 mr-2" />
            Add Deal
          </Button>
          {onSettingsClick && (
            <Button variant="outline" size="icon" onClick={onSettingsClick}>
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
              <Card
                key={stageData.stage.id}
                className="w-72 shrink-0 flex flex-col bg-muted/30"
              >
                <CardHeader className="p-3 pb-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <div
                        className="w-3 h-3 rounded-full"
                        style={{ backgroundColor: stageData.stage.color }}
                      />
                      <CardTitle className="text-sm font-medium">
                        {stageData.stage.name}
                      </CardTitle>
                      <Badge variant="secondary" className="text-xs">
                        {stageData.deals.length}
                      </Badge>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={() => handleAddDeal(stageData.stage.id)}
                    >
                      <Plus className="h-3 w-3" />
                    </Button>
                  </div>
                  {stageData.total_amount > 0 && (
                    <div className="text-xs text-muted-foreground">
                      {formatCurrency(stageData.total_amount)}
                    </div>
                  )}
                </CardHeader>
                <CardContent className="p-2 pt-0 flex-1 overflow-y-auto">
                  <DroppableColumn stage={stageData.stage} isOver={overId === stageData.stage.id}>
                    {stageData.deals.map((deal) => (
                      <div key={deal.id} className="group">
                        <CrmDealCard
                          deal={deal}
                          onEdit={handleEditDeal}
                          onDelete={handleDeleteDeal}
                          isDragging={activeDeal?.id === deal.id}
                        />
                      </div>
                    ))}
                    {stageData.deals.length === 0 && (
                      <div className="text-center py-8 text-sm text-muted-foreground">
                        Drop deals here
                      </div>
                    )}
                  </DroppableColumn>
                </CardContent>
              </Card>
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
          projectId={projectId}
          pipelineId={pipeline.id}
          stages={stages}
          deal={editingDeal}
          initialStageId={initialStageId}
          onSubmit={handleFormSubmit}
        />
      )}
    </div>
  );
}
