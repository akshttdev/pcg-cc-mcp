import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
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
import { cn } from '@/lib/utils';
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
import { ArrowDown, ArrowUp, Pencil, Plus, Trash2 } from 'lucide-react';

type StageFormValues = {
  name: string;
  description?: string;
  color: string;
  probability: number;
  is_closed: boolean;
  is_won: boolean;
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
  onSubmit: (values: StageFormValues) => Promise<void>;
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
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription>Configure the stage name, probability, and status.</DialogDescription>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4">
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
          <DialogFooter>
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
  onSubmit: (values: PipelineFormValues) => Promise<void>;
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

interface CrmPipelineSettingsProps {
  projectId: string;
}

export function CrmPipelineSettings({ projectId }: CrmPipelineSettingsProps) {
  const queryClient = useQueryClient();
  const { data: pipelines = [], isLoading: pipelinesLoading } = useCrmPipelines(projectId);
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

  const stageDialogValues: StageFormValues = editingStage
    ? {
        name: editingStage.name,
        description: editingStage.description ?? undefined,
        color: editingStage.color,
        probability: editingStage.probability,
        is_closed: !!editingStage.is_closed,
        is_won: !!editingStage.is_won,
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

  const stageMutation = useMutation({
    mutationFn: async (values: StageFormValues) => {
      if (!selectedPipelineId) throw new Error('No pipeline selected');

      if (editingStage) {
        await crmPipelinesApi.updateStage(selectedPipelineId, editingStage.id, {
          name: values.name,
          description: values.description,
          color: values.color,
          probability: values.probability,
          is_closed: values.is_closed,
          is_won: values.is_won,
        });
        toast.success('Stage updated.');
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
        toast.success('Stage created.');
      }
    },
    onSuccess: () => {
      if (!selectedPipelineId) return;
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.pipeline(selectedPipelineId) });
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.kanban(selectedPipelineId) });
      setEditingStage(null);
    },
  });

  const pipelineMutation = useMutation({
    mutationFn: async (values: PipelineFormValues) => {
      if (editingPipeline) {
        await crmPipelinesApi.updatePipeline(editingPipeline.id, {
          name: values.name,
          description: values.description,
          color: values.color,
        } as UpdateCrmPipeline);
        toast.success('Pipeline updated.');
      } else {
        await crmPipelinesApi.createPipeline({
          project_id: projectId,
          name: values.name,
          description: values.description,
          color: values.color,
          pipeline_type: values.pipeline_type,
        } as CreateCrmPipeline);
        toast.success('Pipeline created.');
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.pipelines(projectId) });
      setEditingPipeline(null);
    },
  });

  const reorderMutation = useMutation({
    mutationFn: async (stageIds: string[]) => {
      if (!selectedPipelineId) return;
      await crmPipelinesApi.reorderStages(selectedPipelineId, stageIds);
    },
    onSuccess: () => {
      if (!selectedPipelineId) return;
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.pipeline(selectedPipelineId) });
    },
  });

  const deleteStageMutation = useMutation({
    mutationFn: async (stageId: string) => {
      if (!selectedPipelineId) return;
      await crmPipelinesApi.deleteStage(selectedPipelineId, stageId);
    },
    onSuccess: () => {
      if (!selectedPipelineId) return;
      toast.success('Stage deleted.');
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.pipeline(selectedPipelineId) });
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.kanban(selectedPipelineId) });
    },
  });

  const deletePipelineMutation = useMutation({
    mutationFn: async (pipelineId: string) => {
      await crmPipelinesApi.deletePipeline(pipelineId);
    },
    onSuccess: () => {
      toast.success('Pipeline deleted.');
      queryClient.invalidateQueries({ queryKey: crmQueryKeys.pipelines(projectId) });
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
    <div className="grid gap-6 lg:grid-cols-[280px_1fr]">
      <Card>
        <CardHeader className="flex flex-row items-center justify-between gap-2">
          <div>
            <CardTitle className="text-base">Pipelines</CardTitle>
            <CardDescription>Scope CRM boards per initiative.</CardDescription>
          </div>
          <Button
            size="sm"
            onClick={() => {
              setEditingPipeline(null);
              setIsPipelineDialogOpen(true);
            }}
          >
            <Plus className="h-3.5 w-3.5 mr-1" />
            Add
          </Button>
        </CardHeader>
        <CardContent>
          {pipelinesLoading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((item) => (
                <Skeleton key={item} className="h-12 w-full" />
              ))}
            </div>
          ) : pipelines.length === 0 ? (
            <div className="text-sm text-muted-foreground">
              No pipelines yet. Create the first pipeline to get started.
            </div>
          ) : (
            <div className="space-y-2">
              {pipelines.map((pipeline) => (
                <div
                  key={pipeline.id}
                  role="button"
                  tabIndex={0}
                  className={cn(
                    'w-full rounded-md border px-3 py-2 text-left text-sm transition hover:bg-accent cursor-pointer',
                    selectedPipelineId === pipeline.id && 'border-primary bg-primary/5'
                  )}
                  onClick={() => setSelectedPipelineId(pipeline.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      setSelectedPipelineId(pipeline.id);
                    }
                  }}
                >
                  <div className="flex items-center justify-between">
                    <span className="font-medium">{pipeline.name}</span>
                    <Badge variant="outline" className="text-[11px]">
                      {pipeline.pipeline_type}
                    </Badge>
                  </div>
                  <p className="text-xs text-muted-foreground truncate">
                    {pipeline.description || 'No description'}
                  </p>
                  <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
                    <span
                      className="inline-flex h-2 w-2 rounded-full"
                      style={{ backgroundColor: pipeline.color ?? '#3B82F6' }}
                    />
                    {pipeline.pipeline_type === 'custom' ? 'Custom pipeline' : 'System pipeline'}
                  </div>
                  <div className="mt-2 flex gap-2">
                    <Button
                      type="button"
                      size="sm"
                      variant="secondary"
                      className="h-7 text-xs"
                      onClick={(event) => {
                        event.stopPropagation();
                        setEditingPipeline(pipeline);
                        setIsPipelineDialogOpen(true);
                      }}
                    >
                      <Pencil className="mr-1 h-3 w-3" />
                      Edit
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      className="h-7 text-xs text-destructive"
                      onClick={(event) => {
                        event.stopPropagation();
                        handleDeletePipeline(pipeline);
                      }}
                      disabled={pipelines.length <= 1}
                    >
                      <Trash2 className="mr-1 h-3 w-3" />
                      Delete
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      <Card className="min-h-[420px]">
        <CardHeader className="flex flex-row items-center justify-between gap-4">
          <div>
            <CardTitle className="text-base">
              {pipelineData ? pipelineData.name : 'Select a pipeline'}
            </CardTitle>
            <CardDescription>
              {pipelineData ? selectedPipelineCountText : 'Choose a pipeline to manage stages.'}
            </CardDescription>
          </div>
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
        </CardHeader>
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
            <ScrollArea className="h-[520px]">
              <div className="space-y-3 p-4">
                {pipelineData.stages.map((stage, index) => (
                  <div
                    key={stage.id}
                    className="rounded-lg border bg-card px-4 py-3"
                  >
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <div className="flex items-center gap-2">
                          <span
                            className="inline-flex h-3 w-3 rounded-full"
                            style={{ backgroundColor: stage.color }}
                          />
                          <p className="font-medium text-sm">{stage.name}</p>
                          <Badge variant="secondary" className="text-[11px]">
                            {stage.probability}%
                          </Badge>
                        </div>
                        <div className="mt-1 flex gap-2 text-xs text-muted-foreground">
                          {stage.is_closed ? <Badge variant="outline">Closed</Badge> : <Badge variant="outline">Open</Badge>}
                          {stage.is_won ? <Badge variant="outline">Won</Badge> : <Badge variant="outline">Standard</Badge>}
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-8 w-8"
                          onClick={() => handleMoveStage(stage.id, 'up')}
                          disabled={index === 0 || reorderMutation.isPending}
                        >
                          <ArrowUp className="h-4 w-4" />
                        </Button>
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-8 w-8"
                          onClick={() => handleMoveStage(stage.id, 'down')}
                          disabled={index === pipelineData.stages.length - 1 || reorderMutation.isPending}
                        >
                          <ArrowDown className="h-4 w-4" />
                        </Button>
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-8 w-8"
                          onClick={() => {
                            setEditingStage(stage);
                            setIsStageDialogOpen(true);
                          }}
                        >
                          <Pencil className="h-4 w-4" />
                        </Button>
                        <Button
                          size="icon"
                          variant="ghost"
                          className="h-8 w-8 text-destructive"
                          onClick={() => handleDeleteStage(stage)}
                        >
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
        onSubmit={(values) => pipelineMutation.mutateAsync(values)}
        title={editingPipeline ? 'Edit Pipeline' : 'Create Pipeline'}
        disableType={!!editingPipeline}
      />
    </div>
  );
}
