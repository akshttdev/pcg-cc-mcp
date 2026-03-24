import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
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
import { Checkbox } from '@/components/ui/checkbox';
import { FormField } from '@/components/ui/form-field';
import { toast } from 'sonner';
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from '@/components/ui/tabs';
import { StageConfigEditor, StageActiveRules } from './StageConfigEditor';
import type { StageConfig } from '@/types/crm';

/**
 * Parse stage_config JSON with reconciliation logic.
 * Derives missing top-level fields (required_fields, assigned_agent, approval_gate)
 * from on_enter_actions / on_exit_validations so the UI checkboxes stay in sync.
 */
export function parseStageConfig(json?: string): Partial<StageConfig> {
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
}

export type StageFormValues = {
  name: string;
  description?: string;
  color: string;
  probability: number;
  is_closed: boolean;
  is_won: boolean;
  stage_config?: Partial<import('@/types/crm').StageConfig>;
};

export interface StageDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialValues: StageFormValues;
  onSubmit: (values: StageFormValues) => Promise<unknown>;
  title: string;
}

export function StageDialog({ open, onOpenChange, initialValues, onSubmit, title }: StageDialogProps) {
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
              <FormField label="Stage Name" htmlFor="stage-name" required>
                <Input
                  id="stage-name"
                  value={formValues.name}
                  onChange={(e) => setFormValues((prev) => ({ ...prev, name: e.target.value }))}
                  required
                />
              </FormField>
              <FormField label="Description" htmlFor="stage-description">
                <Textarea
                  id="stage-description"
                  value={formValues.description ?? ''}
                  onChange={(e) => setFormValues((prev) => ({ ...prev, description: e.target.value }))}
                  placeholder="Optional notes for this stage"
                />
              </FormField>
              <div className="grid grid-cols-2 gap-4">
                <FormField label="Stage Color" htmlFor="stage-color">
                  <Input
                    id="stage-color"
                    type="color"
                    value={formValues.color}
                    onChange={(e) => setFormValues((prev) => ({ ...prev, color: e.target.value }))}
                    className="h-10"
                  />
                </FormField>
                <FormField label="Probability (%)" htmlFor="stage-probability">
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
                </FormField>
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
