import { useEffect, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Checkbox } from '@/components/ui/checkbox';
import { Separator } from '@/components/ui/separator';
import { toast } from 'sonner';
import NiceModal from '@ebay/nice-modal-react';
import { crmPipelinesApi } from '@/lib/api';
import { useCrmPipeline, useCrmPipelines, crmQueryKeys } from '@/hooks/useCrmPipeline';
import type {
  CrmPipeline,
  CrmPipelineStage,
  PipelineType,
  CreateCrmPipeline,
  UpdateCrmPipeline,
} from '@/types/crm';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import { ArrowDown, ArrowUp, Bot, Pencil, Plus, Settings2, ShieldCheck, Trash2, Zap } from 'lucide-react';
import { StageConfigEditor, StageActiveRules } from './StageConfigEditor';
import type { StageConfig, StageAction, StageValidation } from '@/types/crm';

type StageFormValues = {
  name: string;
  description?: string;
  color: string;
  probability: number;
  is_closed: boolean;
  is_won: boolean;
  stage_config?: Partial<import('@/types/crm').StageConfig>;
};

type PipelineFormValues = {
  name: string;
  description?: string;
  color?: string;
  pipeline_type: PipelineType;
};

interface StageDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialValues: StageFormValues;
  onSubmit: (values: StageFormValues) => Promise<unknown>;
  title: string;
}

function StageDialog({ open, onOpenChange, initialValues, onSubmit, title }: StageDialogProps) {
  const [formValues, setFormValues] = useState(initialValues);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (open) {
      setFormValues(initialValues);
    }
  }, [initialValues, open]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmitting(true);
    try {
      await onSubmit(formValues);
      onOpenChange(false);
    } catch (error) {
      console.error('Failed to save pipeline stage', error);
      toast.error('Failed to save stage.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[520px] max-h-[85vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>Configure stage properties and automation behavior.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit}>
          <Tabs defaultValue="stage" className="w-full">
            <TabsList className="w-full">
              <TabsTrigger value="stage" className="flex-1 text-xs">Stage</TabsTrigger>
              <TabsTrigger value="automation" className="flex-1 text-xs">Automation</TabsTrigger>
              <TabsTrigger value="rules" className="flex-1 text-xs">Active Rules</TabsTrigger>
            </TabsList>

            <TabsContent value="stage" className="space-y-4 mt-4">
              <div className="space-y-2">
                <Label htmlFor="stage-name">Stage Name</Label>
                <Input
                  id="stage-name"
                  value={formValues.name}
                  onChange={(e) => setFormValues((prev) => ({ ...prev, name: e.target.value }))}
                  required
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="stage-description">Description</Label>
                <Textarea
                  id="stage-description"
                  value={formValues.description ?? ''}
                  onChange={(e) => setFormValues((prev) => ({ ...prev, description: e.target.value }))}
                  placeholder="Optional notes for this stage"
                />
              </div>
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="stage-color">Stage Color</Label>
                  <Input
                    id="stage-color"
                    type="color"
                    value={formValues.color}
                    onChange={(e) => setFormValues((prev) => ({ ...prev, color: e.target.value }))}
                    className="h-10"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="stage-probability">Probability (%)</Label>
                  <Input
                    id="stage-probability"
                    type="number"
                    min={0}
                    max={100}
                    value={formValues.probability}
                    onChange={(e) =>
                      setFormValues((prev) => ({ ...prev, probability: Number(e.target.value ?? 0) }))
                    }
                  />
                </div>
              </div>
              <div className="flex items-center gap-3">
                <Checkbox
                  id="stage-closed"
                  checked={formValues.is_closed}
                  onCheckedChange={(checked) =>
                    setFormValues((prev) => ({ ...prev, is_closed: checked === true }))
                  }
                />
                <Label htmlFor="stage-closed" className="text-sm">Mark as Closed Stage</Label>
              </div>
              <div className="flex items-center gap-3">
                <Checkbox
                  id="stage-won"
                  checked={formValues.is_won}
                  onCheckedChange={(checked) =>
                    setFormValues((prev) => ({ ...prev, is_won: checked === true }))
                  }
                />
                <Label htmlFor="stage-won" className="text-sm">Counts as Closed Won</Label>
              </div>
            </TabsContent>

            <TabsContent value="automation" className="mt-4">
              <StageConfigEditor
                config={formValues.stage_config ?? {}}
                onChange={(config) => setFormValues((prev) => ({ ...prev, stage_config: config }))}
              />
            </TabsContent>

            <TabsContent value="rules" className="mt-4">
              <StageActiveRules config={formValues.stage_config ?? {}} />
            </TabsContent>
          </Tabs>

          <DialogFooter className="mt-4">
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Saving…' : 'Save Stage'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

interface PipelineDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialValues: PipelineFormValues;
  onSubmit: (values: PipelineFormValues) => Promise<unknown>;
  title: string;
  disableType?: boolean;
}

function PipelineDialog({ open, onOpenChange, initialValues, onSubmit, title, disableType }: PipelineDialogProps) {
  const [formValues, setFormValues] = useState(initialValues);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    if (open) {
      setFormValues(initialValues);
    }
  }, [initialValues, open]);

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setIsSubmitting(true);
    try {
      await onSubmit(formValues);
      onOpenChange(false);
    } catch (error) {
      console.error('Failed to save pipeline', error);
      toast.error('Failed to save pipeline.');
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>Manage CRM pipelines for this project.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="pipeline-name">Pipeline Name</Label>
            <Input
              id="pipeline-name"
              value={formValues.name}
              onChange={(e) => setFormValues((prev) => ({ ...prev, name: e.target.value }))}
              required
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="pipeline-description">Description</Label>
            <Textarea
              id="pipeline-description"
              value={formValues.description ?? ''}
              onChange={(e) => setFormValues((prev) => ({ ...prev, description: e.target.value }))}
              placeholder="What is this pipeline used for?"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="pipeline-color">Accent Color</Label>
            <Input
              id="pipeline-color"
              type="color"
              value={formValues.color ?? '#3B82F6'}
              onChange={(e) => setFormValues((prev) => ({ ...prev, color: e.target.value }))}
              className="h-10"
            />
          </div>
          <div className="space-y-2">
            <Label>Pipeline Type</Label>
            <Select
              value={formValues.pipeline_type}
              onValueChange={(value: PipelineType) =>
                setFormValues((prev) => ({ ...prev, pipeline_type: value }))
              }
              disabled={disableType}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="conferences">Conferences</SelectItem>
                <SelectItem value="clients">Clients</SelectItem>
                <SelectItem value="custom">Custom</SelectItem>
              </SelectContent>
            </Select>
          </div>
          <DialogFooter>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Saving…' : 'Save Pipeline'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

// ── Helpers for Automations tab ──────────────────────────────────────────────

function parseStageConfigSafe(json?: string): StageConfig | null {
  if (!json) return null;
  try { return JSON.parse(json) as StageConfig; } catch { return null; }
}

function formatStageAction(action: StageAction): string {
  switch (action.type) {
    case 'trigger_agent': return `Trigger ${action.agent} (${action.flow_type})`;
    case 'create_review_task': return `Review task: "${action.description}"`;
    case 'create_delivery_deal': return 'Auto-create delivery deal';
    default: return JSON.stringify(action);
  }
}

function formatStageValidation(v: StageValidation): string {
  switch (v.type) {
    case 'require_field': return `Require ${v.field}`;
    case 'require_intel': return `${v.entity} intel = "${v.status}"`;
    case 'require_pending_tasks': return `Max ${v.count} pending tasks`;
    default: return JSON.stringify(v);
  }
}

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

  const parseStageConfig = (json?: string): Partial<StageConfig> => {
    if (!json) return {};
    try {
      const config = JSON.parse(json) as Partial<StageConfig>;
      // Reconcile: if required_fields is empty but on_exit_validations has RequireField
      // entries, derive the checkboxes from the validations so the UI matches reality.
      if ((!config.required_fields || config.required_fields.length === 0) && config.on_exit_validations?.length) {
        const derivedFields = config.on_exit_validations
          .filter((v): v is { type: 'require_field'; field: string; message: string } => v.type === 'require_field')
          .map((v) => v.field);
        if (derivedFields.length > 0) {
          config.required_fields = derivedFields;
        }
      }
      // Reconcile: if no assigned_agent but on_enter_actions has TriggerAgent, derive it
      if (!config.assigned_agent && config.on_enter_actions?.length) {
        const agentAction = config.on_enter_actions.find(
          (a): a is { type: 'trigger_agent'; agent: string; flow_type: string } => a.type === 'trigger_agent'
        );
        if (agentAction) {
          config.assigned_agent = agentAction.agent;
          config.auto_trigger = config.auto_trigger ?? true;
        }
      }
      // Reconcile: if no approval_gate but on_enter_actions has CreateReviewTask, derive it
      if (!config.approval_gate && config.on_enter_actions?.length) {
        const hasReviewTask = config.on_enter_actions.some((a) => a.type === 'create_review_task');
        if (hasReviewTask) {
          config.approval_gate = true;
        }
      }
      return config;
    } catch { return {}; }
  };

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
        pipeline_type: editingPipeline.pipeline_type as PipelineType,
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
                    <Badge variant="outline" className="text-[10px] ml-1">{pipeline.pipeline_type}</Badge>
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

              {/* ── Stages Tab ── */}
              <TabsContent value="stages" className="mt-0">
                <ScrollArea className="h-[480px]">
                  <div className="space-y-3 p-4">
                    {pipelineData.stages.map((stage, index) => (
                      <div key={stage.id} className="rounded-lg border bg-card px-4 py-3" data-testid={`stage-row-${stage.name.toLowerCase().replace(/\s+/g, '-')}`}>
                        <div className="flex items-center justify-between gap-4">
                          <div>
                            <div className="flex items-center gap-2">
                              <span className="inline-flex h-3 w-3 rounded-full" style={{ backgroundColor: stage.color }} />
                              <p className="font-medium text-sm">{stage.name}</p>
                              <Badge variant="secondary" className="text-[11px]">{stage.probability}%</Badge>
                            </div>
                            <div className="mt-1 flex gap-2 text-xs text-muted-foreground">
                              {stage.is_closed ? <Badge variant="outline">Closed</Badge> : <Badge variant="outline">Open</Badge>}
                              {stage.is_won ? <Badge variant="outline">Won</Badge> : <Badge variant="outline">Standard</Badge>}
                            </div>
                          </div>
                          <div className="flex items-center gap-2">
                            <Button size="icon" variant="ghost" className="h-8 w-8" onClick={() => handleMoveStage(stage.id, 'up')} disabled={index === 0 || reorderMutation.isPending}>
                              <ArrowUp className="h-4 w-4" />
                            </Button>
                            <Button size="icon" variant="ghost" className="h-8 w-8" onClick={() => handleMoveStage(stage.id, 'down')} disabled={index === pipelineData.stages.length - 1 || reorderMutation.isPending}>
                              <ArrowDown className="h-4 w-4" />
                            </Button>
                            <Button size="icon" variant="ghost" className="h-8 w-8" data-testid={`stage-edit-${stage.name.toLowerCase().replace(/\s+/g, '-')}`} onClick={() => { setEditingStage(stage); setIsStageDialogOpen(true); }}>
                              <Pencil className="h-4 w-4" />
                            </Button>
                            <Button size="icon" variant="ghost" className="h-8 w-8 text-destructive" onClick={() => handleDeleteStage(stage)}>
                              <Trash2 className="h-4 w-4" />
                            </Button>
                          </div>
                        </div>
                        {stage.description && (
                          <p className="mt-2 text-xs text-muted-foreground">{stage.description}</p>
                        )}
                        {index < pipelineData.stages.length - 1 && <Separator className="mt-3" />}
                      </div>
                    ))}
                    {pipelineData.stages.length === 0 && (
                      <div className="text-sm text-muted-foreground">
                        No stages configured. Add the first stage to define your pipeline.
                      </div>
                    )}
                  </div>
                </ScrollArea>
              </TabsContent>

              {/* ── Automations Tab ── */}
              <TabsContent value="automations" className="mt-0">
                <ScrollArea className="h-[480px]">
                  <div className="space-y-4 p-4">
                    {pipelineData.stages.map((stage) => {
                      const config = parseStageConfigSafe(stage.stage_config);
                      if (!config) return (
                        <div key={stage.id} className="rounded-lg border bg-card px-4 py-2.5">
                          <div className="flex items-center gap-2">
                            <span className="inline-flex h-2.5 w-2.5 rounded-full" style={{ backgroundColor: stage.color }} />
                            <p className="font-medium text-sm">{stage.name}</p>
                            <Badge variant="outline" className="text-[10px] text-muted-foreground">No config</Badge>
                          </div>
                        </div>
                      );
                      const owner = config.stage_owner;
                      const actions = config.on_enter_actions ?? [];
                      const validations = config.on_exit_validations ?? [];
                      const hasAgent = !!config.assigned_agent;
                      return (
                        <div key={stage.id} className="rounded-lg border bg-card px-4 py-3 space-y-2">
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2">
                              <span className="inline-flex h-2.5 w-2.5 rounded-full" style={{ backgroundColor: stage.color }} />
                              <p className="font-medium text-sm">{stage.name}</p>
                              <Badge variant="secondary" className="text-[10px]">{stage.probability}%</Badge>
                            </div>
                            {owner && (
                              <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                                <Badge variant="outline" className="text-[10px]">{owner.type}</Badge>
                                {owner.label}
                              </div>
                            )}
                          </div>
                          {/* Agent + trigger */}
                          {hasAgent && (
                            <div className="flex items-center gap-2 text-xs">
                              <Bot className="h-3 w-3 text-primary" />
                              <span className="font-medium capitalize">{config.assigned_agent}</span>
                              {config.auto_trigger ? (
                                <Badge className="text-[9px] bg-blue-500/15 text-blue-600 border-blue-200 dark:text-blue-400 dark:border-blue-800">auto-trigger {config.cancel_window_secs}s</Badge>
                              ) : (
                                <Badge variant="outline" className="text-[9px]">manual</Badge>
                              )}
                            </div>
                          )}
                          {/* On enter actions */}
                          {actions.length > 0 && (
                            <div className="space-y-1">
                              {actions.map((action, i) => (
                                <div key={i} className="flex items-start gap-1.5 text-[11px] text-muted-foreground">
                                  <Zap className="h-3 w-3 text-blue-500 mt-0.5 shrink-0" />
                                  <span>{formatStageAction(action)}</span>
                                </div>
                              ))}
                            </div>
                          )}
                          {/* Exit validations */}
                          {validations.length > 0 && (
                            <div className="space-y-1">
                              {validations.map((v, i) => (
                                <div key={i} className="flex items-start gap-1.5 text-[11px] text-muted-foreground">
                                  <ShieldCheck className="h-3 w-3 text-amber-500 mt-0.5 shrink-0" />
                                  <span>{formatStageValidation(v)}</span>
                                </div>
                              ))}
                            </div>
                          )}
                          {/* Required fields */}
                          {(config.required_fields ?? []).length > 0 && (
                            <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                              <ShieldCheck className="h-3 w-3 text-amber-500 shrink-0" />
                              <span>Required: {config.required_fields!.join(', ')}</span>
                            </div>
                          )}
                          {/* No automations */}
                          {!hasAgent && actions.length === 0 && validations.length === 0 && (config.required_fields ?? []).length === 0 && (
                            <p className="text-[11px] text-muted-foreground/50 italic">No automations configured</p>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </ScrollArea>
              </TabsContent>

              {/* ── Pipeline Settings Tab ── */}
              <TabsContent value="settings" className="mt-0">
                <div className="p-4 space-y-4">
                  <div className="space-y-3">
                    <div className="flex items-center gap-2">
                      <Settings2 className="h-4 w-4 text-muted-foreground" />
                      <p className="text-sm font-medium">Pipeline Details</p>
                    </div>
                    <div className="grid gap-3 text-sm">
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Name</span>
                        <span className="font-medium">{pipelineData.name}</span>
                      </div>
                      <Separator />
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Type</span>
                        <Badge variant="outline">{pipelineData.pipeline_type}</Badge>
                      </div>
                      <Separator />
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Description</span>
                        <span className="text-right max-w-[250px] truncate">{pipelineData.description || '—'}</span>
                      </div>
                      <Separator />
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Color</span>
                        <div className="flex items-center gap-2">
                          <span className="inline-flex h-4 w-4 rounded-full border" style={{ backgroundColor: selectedPipeline?.color ?? '#3B82F6' }} />
                          <span className="text-xs font-mono">{selectedPipeline?.color ?? '#3B82F6'}</span>
                        </div>
                      </div>
                      <Separator />
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Stages</span>
                        <span>{pipelineData.stages.length}</span>
                      </div>
                      <Separator />
                      <div className="flex items-center justify-between">
                        <span className="text-muted-foreground">Status</span>
                        <Badge variant={selectedPipeline?.is_active ? 'default' : 'secondary'}>{selectedPipeline?.is_active ? 'Active' : 'Inactive'}</Badge>
                      </div>
                    </div>
                  </div>
                  <div className="pt-2 flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        setEditingPipeline(selectedPipeline ?? null);
                        setIsPipelineDialogOpen(true);
                      }}
                    >
                      <Pencil className="h-3.5 w-3.5 mr-1.5" />
                      Edit Pipeline
                    </Button>
                    <Button
                      variant="ghost"
                      size="sm"
                      className="text-destructive"
                      onClick={() => selectedPipeline && handleDeletePipeline(selectedPipeline)}
                      disabled={pipelines.length <= 1}
                    >
                      <Trash2 className="h-3.5 w-3.5 mr-1.5" />
                      Delete
                    </Button>
                  </div>
                </div>
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
