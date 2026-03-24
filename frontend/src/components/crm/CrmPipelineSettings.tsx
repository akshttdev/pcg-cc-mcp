import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import NiceModal from '@ebay/nice-modal-react';
import { crmPipelinesApi } from '@/lib/api';
import { useCrmPipeline, useCrmPipelines, crmQueryKeys } from '@/hooks/useCrmPipeline';
import type {
  CrmPipeline,
  CrmPipelineStage,
  CreateCrmPipeline,
  UpdateCrmPipeline,
} from '@/types/crm';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import { Plus } from 'lucide-react';
import { StageDialog, parseStageConfig } from './PipelineSettingsStageDialog';
import type { StageFormValues } from './PipelineSettingsStageDialog';
import { PipelineDialog } from './PipelineSettingsPipelineDialog';
import type { PipelineFormValues } from './PipelineSettingsPipelineDialog';
import { PipelineSettingsStagesTab } from './PipelineSettingsStagesTab';
import { PipelineSettingsAutomationsTab } from './PipelineSettingsAutomationsTab';
import { PipelineSettingsPipelineTab } from './PipelineSettingsPipelineTab';

// ── Component ───────────────────────────────────────────────────────────────

interface CrmPipelineSettingsProps {
  organizationId: string;
}

export function CrmPipelineSettings({ organizationId }: CrmPipelineSettingsProps) {
  const queryClient = useQueryClient();
  const { data: pipelines = [], isLoading: pipelinesLoading } = useCrmPipelines(organizationId);
  const [selectedPipelineId, setSelectedPipelineId] = useState<string | null>(null);
  const [isStageDialogOpen, setIsStageDialogOpen] = useState(false);
  const [isPipelineDialogOpen, setIsPipelineDialogOpen] = useState(false);
  const [editingStage, setEditingStage] = useState<CrmPipelineStage | null>(null);
  const [editingPipeline, setEditingPipeline] = useState<CrmPipeline | null>(null);

  useEffect(() => {
    if (!selectedPipelineId && pipelines.length > 0) {
      setSelectedPipelineId(pipelines[0].id);
    } else if (
      selectedPipelineId &&
      pipelines.length > 0 &&
      !pipelines.some((pipeline) => pipeline.id === selectedPipelineId)
    ) {
      setSelectedPipelineId(pipelines[0].id);
    }
  }, [pipelines, selectedPipelineId]);

  const { data: pipelineData, isLoading: pipelineLoading } = useCrmPipeline(
    selectedPipelineId ?? undefined
  );

  const selectedPipeline = pipelines.find((p) => p.id === selectedPipelineId) ?? null;

  const stageDialogValues: StageFormValues = editingStage
    ? {
        name: editingStage.name,
        description: editingStage.description ?? undefined,
        color: editingStage.color,
        probability: editingStage.probability,
        is_closed: !!editingStage.is_closed,
        is_won: !!editingStage.is_won,
        stage_config: parseStageConfig(editingStage.stage_config),
      }
    : {
        name: 'New Stage',
        description: '',
        color: '#6B7280',
        probability: 10,
        is_closed: false,
        is_won: false,
      };

  const pipelineDialogValues: PipelineFormValues = editingPipeline
    ? {
        name: editingPipeline.name,
        description: editingPipeline.description ?? undefined,
        color: editingPipeline.color ?? '#3B82F6',
        pipeline_type: editingPipeline.pipeline_type as import('@/types/crm').PipelineType,
      }
    : {
        name: 'Custom Pipeline',
        description: '',
        color: '#3B82F6',
        pipeline_type: 'custom',
      };

  const stageMutation = useMutationWithToast({
    mutationFn: async (values: StageFormValues): Promise<string> => {
      if (!selectedPipelineId) throw new Error('No pipeline selected');

      if (editingStage) {
        await crmPipelinesApi.updateStage(selectedPipelineId, editingStage.id, {
          name: values.name,
          description: values.description,
          color: values.color,
          probability: values.probability,
          is_closed: values.is_closed,
          is_won: values.is_won,
          stage_config: values.stage_config ? JSON.stringify(values.stage_config) : undefined,
        });
        return 'Stage updated.';
      } else {
        const position = pipelineData?.stages.length ?? 0;
        await crmPipelinesApi.createStage(selectedPipelineId, {
          name: values.name,
          description: values.description,
          color: values.color,
          position,
          probability: values.probability,
          is_closed: values.is_closed,
          is_won: values.is_won,
        });
        return 'Stage created.';
      }
    },
    successMessage: (msg) => msg,
    errorMessage: 'Failed to save stage',
    invalidateKeys: selectedPipelineId
      ? [crmQueryKeys.pipeline(selectedPipelineId), crmQueryKeys.kanban(selectedPipelineId)]
      : [],
    onSuccess: () => {
      setEditingStage(null);
    },
  });

  const pipelineMutation = useMutationWithToast({
    mutationFn: async (values: PipelineFormValues): Promise<string> => {
      if (editingPipeline) {
        await crmPipelinesApi.updatePipeline(editingPipeline.id, {
          name: values.name,
          description: values.description,
          color: values.color,
        } as UpdateCrmPipeline);
        return 'Pipeline updated.';
      } else {
        await crmPipelinesApi.createPipeline({
          organization_id: organizationId,
          name: values.name,
          description: values.description,
          color: values.color,
          pipeline_type: values.pipeline_type,
        } as CreateCrmPipeline);
        return 'Pipeline created.';
      }
    },
    successMessage: (msg) => msg,
    errorMessage: 'Failed to save pipeline',
    invalidateKeys: [crmQueryKeys.pipelines(organizationId)],
    onSuccess: () => {
      setEditingPipeline(null);
    },
  });

  const reorderMutation = useMutationWithToast({
    mutationFn: async (stageIds: string[]) => {
      if (!selectedPipelineId) return;
      await crmPipelinesApi.reorderStages(selectedPipelineId, stageIds);
    },
    errorMessage: 'Failed to reorder stages',
    invalidateKeys: selectedPipelineId
      ? [crmQueryKeys.pipeline(selectedPipelineId)]
      : [],
  });

  const deleteStageMutation = useMutationWithToast({
    mutationFn: async (stageId: string) => {
      if (!selectedPipelineId) return;
      await crmPipelinesApi.deleteStage(selectedPipelineId, stageId);
    },
    successMessage: 'Stage deleted.',
    errorMessage: 'Failed to delete stage',
    invalidateKeys: selectedPipelineId
      ? [crmQueryKeys.pipeline(selectedPipelineId), crmQueryKeys.kanban(selectedPipelineId)]
      : [],
  });

  const deletePipelineMutation = useMutationWithToast({
    mutationFn: async (pipelineId: string) => {
      await crmPipelinesApi.deletePipeline(pipelineId);
    },
    successMessage: 'Pipeline deleted.',
    errorMessage: 'Failed to delete pipeline',
    invalidateKeys: [crmQueryKeys.pipelines(organizationId)],
    onSuccess: () => {
      if (selectedPipelineId) {
        queryClient.removeQueries({ queryKey: crmQueryKeys.pipeline(selectedPipelineId) });
      }
      setSelectedPipelineId(null);
    },
  });

  const handleMoveStage = (stageId: string, direction: 'up' | 'down') => {
    if (!pipelineData) return;
    const stages = [...pipelineData.stages];
    const currentIndex = stages.findIndex((stage) => stage.id === stageId);
    if (currentIndex === -1) return;

    const targetIndex = direction === 'up' ? currentIndex - 1 : currentIndex + 1;
    if (targetIndex < 0 || targetIndex >= stages.length) return;

    const updated = [...stages];
    const [removed] = updated.splice(currentIndex, 1);
    updated.splice(targetIndex, 0, removed);
    reorderMutation.mutate(updated.map((stage) => stage.id));
  };

  const handleDeleteStage = async (stage: CrmPipelineStage) => {
    const result = await NiceModal.show('confirm', {
      title: 'Delete Stage',
      message: `Delete "${stage.name}"? Deals in this stage will need to be reassigned.`,
      confirmText: 'Delete Stage',
      variant: 'destructive',
    });

    if (result === 'confirmed') {
      deleteStageMutation.mutate(stage.id);
    }
  };

  const handleDeletePipeline = async (pipeline: CrmPipeline) => {
    const result = await NiceModal.show('confirm', {
      title: 'Delete Pipeline',
      message: `Delete pipeline "${pipeline.name}"? This action cannot be undone.`,
      confirmText: 'Delete Pipeline',
      variant: 'destructive',
    });

    if (result === 'confirmed') {
      deletePipelineMutation.mutate(pipeline.id);
    }
  };

  const selectedPipelineCountText = useMemo(() => {
    const count = pipelineData?.stages.length ?? 0;
    return `${count} stage${count === 1 ? '' : 's'}`;
  }, [pipelineData]);

  return (
    <div className="space-y-4">
      {/* ── Pipeline Selector (compact dropdown) ── */}
      <div className="flex items-center gap-3">
        {pipelinesLoading ? (
          <Skeleton className="h-9 w-48" />
        ) : (
          <Select
            value={selectedPipelineId ?? undefined}
            onValueChange={(id) => setSelectedPipelineId(id)}
          >
            <SelectTrigger className="w-[240px]">
              <SelectValue placeholder="Select a pipeline" />
            </SelectTrigger>
            <SelectContent>
              {pipelines.map((pipeline) => (
                <SelectItem key={pipeline.id} value={pipeline.id}>
                  <div className="flex items-center gap-2">
                    <span
                      className="inline-flex h-2 w-2 rounded-full shrink-0"
                      style={{ backgroundColor: pipeline.color ?? '#3B82F6' }}
                    />
                    {pipeline.name}
                    <Badge variant="outline" className="text-xs ml-1">{pipeline.pipeline_type}</Badge>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        )}
        {pipelineData && (
          <span className="text-xs text-muted-foreground">{selectedPipelineCountText}</span>
        )}
        <div className="ml-auto flex items-center gap-2">
          {pipelineData && (
            <Button
              size="sm"
              onClick={() => {
                setEditingStage(null);
                setIsStageDialogOpen(true);
              }}
            >
              <Plus className="h-3.5 w-3.5 mr-1" />
              Add Stage
            </Button>
          )}
          <Button
            size="sm"
            variant="outline"
            onClick={() => {
              setEditingPipeline(null);
              setIsPipelineDialogOpen(true);
            }}
          >
            <Plus className="h-3.5 w-3.5 mr-1" />
            New Pipeline
          </Button>
        </div>
      </div>

      {/* ── Main Content ── */}
      <Card className="min-h-[420px]">
        <CardContent className="p-0">
          {pipelineLoading ? (
            <div className="space-y-3 p-4">
              {[1, 2, 3].map((item) => (
                <Skeleton key={item} className="h-16 w-full" />
              ))}
            </div>
          ) : !pipelineData ? (
            <div className="p-6 text-sm text-muted-foreground">
              Select a pipeline to edit its stages.
            </div>
          ) : (
            <Tabs defaultValue="stages" className="w-full">
              <TabsList className="mx-4 mt-1">
                <TabsTrigger value="stages" className="text-xs">Stages</TabsTrigger>
                <TabsTrigger value="automations" className="text-xs">Automations</TabsTrigger>
                <TabsTrigger value="settings" className="text-xs">Pipeline</TabsTrigger>
              </TabsList>

              <TabsContent value="stages" className="mt-0">
                <PipelineSettingsStagesTab
                  stages={pipelineData.stages}
                  onMoveStage={handleMoveStage}
                  onEditStage={(stage) => { setEditingStage(stage); setIsStageDialogOpen(true); }}
                  onDeleteStage={handleDeleteStage}
                  isReordering={reorderMutation.isPending}
                />
              </TabsContent>

              <TabsContent value="automations" className="mt-0">
                <PipelineSettingsAutomationsTab stages={pipelineData.stages} />
              </TabsContent>

              <TabsContent value="settings" className="mt-0">
                <PipelineSettingsPipelineTab
                  pipelineData={pipelineData}
                  selectedPipeline={selectedPipeline}
                  onEditPipeline={(pipeline) => {
                    setEditingPipeline(pipeline);
                    setIsPipelineDialogOpen(true);
                  }}
                  onDeletePipeline={handleDeletePipeline}
                  canDelete={pipelines.length > 1}
                />
              </TabsContent>
            </Tabs>
          )}
        </CardContent>
      </Card>

      <StageDialog
        open={isStageDialogOpen}
        onOpenChange={(open) => {
          setIsStageDialogOpen(open);
          if (!open) {
            setEditingStage(null);
          }
        }}
        initialValues={stageDialogValues}
        onSubmit={(values) => stageMutation.mutateAsync(values)}
        title={editingStage ? 'Edit Stage' : 'Create Stage'}
      />

      <PipelineDialog
        open={isPipelineDialogOpen}
        onOpenChange={(open) => {
          setIsPipelineDialogOpen(open);
          if (!open) {
            setEditingPipeline(null);
          }
        }}
        initialValues={pipelineDialogValues}
        onSubmit={async (values) => { await pipelineMutation.mutateAsync(values); }}
        title={editingPipeline ? 'Edit Pipeline' : 'Create Pipeline'}
        disableType={!!editingPipeline}
      />
    </div>
  );
}
