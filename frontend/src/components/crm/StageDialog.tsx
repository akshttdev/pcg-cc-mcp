import { useEffect, useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
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

import { StageConfigEditor } from './StageConfigEditor';

export type StageFormValues = {
  name: string;
  description?: string;
  color: string;
  probability: number;
  is_closed: boolean;
  is_won: boolean;
  stage_config?: Partial<import('@/types/crm').StageConfig>;
};

interface StageDialogProps {
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
          <StageConfigEditor
            config={formValues.stage_config ?? {}}
            onChange={(config) => setFormValues((prev) => ({ ...prev, stage_config: config }))}
          />
          <DialogFooter>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting ? 'Saving...' : 'Save Stage'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
