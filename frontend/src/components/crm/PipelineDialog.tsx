import { useEffect, useState } from 'react';
import { toast } from 'sonner';

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
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';
import type { PipelineType } from '@/types/crm';

export type PipelineFormValues = {
  name: string;
  description?: string;
  color?: string;
  pipeline_type: PipelineType;
};

interface PipelineDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  initialValues: PipelineFormValues;
  onSubmit: (values: PipelineFormValues) => Promise<unknown>;
  title: string;
  disableType?: boolean;
}

export function PipelineDialog({ open, onOpenChange, initialValues, onSubmit, title, disableType }: PipelineDialogProps) {
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
              {isSubmitting ? 'Saving...' : 'Save Pipeline'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
