import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
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
import { Loader2 } from 'lucide-react';
import { crmApi } from '@/lib/api';
import type { CrmDealWithContact, CrmPipelineStage, CreateCrmDeal, UpdateCrmDeal } from '@/types/crm';

interface CrmDealFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  pipelineId: string;
  stages: CrmPipelineStage[];
  deal?: CrmDealWithContact;
  initialStageId?: string;
  onSubmit: (data: CreateCrmDeal | UpdateCrmDeal) => Promise<void>;
}

export function CrmDealForm({
  open,
  onOpenChange,
  projectId,
  pipelineId,
  stages,
  deal,
  initialStageId,
  onSubmit,
}: CrmDealFormProps) {
  const isEditing = !!deal;
  const [isSubmitting, setIsSubmitting] = useState(false);

  const [formData, setFormData] = useState({
    name: deal?.name ?? '',
    description: deal?.description ?? '',
    amount: deal?.amount?.toString() ?? '',
    currency: deal?.currency ?? 'USD',
    stageId: deal?.crm_stage_id ?? initialStageId ?? stages[0]?.id ?? '',
    contactId: deal?.crm_contact_id ?? '',
    expectedCloseDate: deal?.expected_close_date
      ? new Date(deal.expected_close_date).toISOString().split('T')[0]
      : '',
  });

  // Fetch contacts for the dropdown
  const { data: contacts = [] } = useQuery({
    queryKey: ['crm', 'contacts', projectId],
    queryFn: () => crmApi.listContacts(projectId, { limit: 100 }),
    enabled: open,
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsSubmitting(true);

    try {
      if (isEditing) {
        const updateData: UpdateCrmDeal = {
          name: formData.name || undefined,
          description: formData.description || undefined,
          amount: formData.amount ? parseFloat(formData.amount) : undefined,
          currency: formData.currency || undefined,
          crm_stage_id: formData.stageId || undefined,
          crm_contact_id: formData.contactId || undefined,
          expected_close_date: formData.expectedCloseDate || undefined,
        };
        await onSubmit(updateData);
      } else {
        const createData: CreateCrmDeal = {
          project_id: projectId,
          crm_pipeline_id: pipelineId,
          crm_stage_id: formData.stageId || undefined,
          crm_contact_id: formData.contactId || undefined,
          name: formData.name,
          description: formData.description || undefined,
          amount: formData.amount ? parseFloat(formData.amount) : undefined,
          currency: formData.currency || undefined,
          expected_close_date: formData.expectedCloseDate || undefined,
        };
        await onSubmit(createData);
      }
      onOpenChange(false);
    } catch (error) {
      console.error('Failed to save deal:', error);
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Edit Deal' : 'Create Deal'}</DialogTitle>
          <DialogDescription>
            {isEditing
              ? 'Update the deal information below.'
              : 'Add a new deal to track in your pipeline.'}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          {/* Deal Name */}
          <div className="space-y-2">
            <Label htmlFor="name">Deal Name *</Label>
            <Input
              id="name"
              value={formData.name}
              onChange={(e) => setFormData({ ...formData, name: e.target.value })}
              placeholder="e.g., Enterprise License - Acme Corp"
              required
            />
          </div>

          {/* Amount and Currency */}
          <div className="grid grid-cols-3 gap-4">
            <div className="col-span-2 space-y-2">
              <Label htmlFor="amount">Amount</Label>
              <Input
                id="amount"
                type="number"
                value={formData.amount}
                onChange={(e) => setFormData({ ...formData, amount: e.target.value })}
                placeholder="0"
                min="0"
                step="0.01"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="currency">Currency</Label>
              <Select
                value={formData.currency}
                onValueChange={(value) => setFormData({ ...formData, currency: value })}
              >
                <SelectTrigger id="currency">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="USD">USD</SelectItem>
                  <SelectItem value="EUR">EUR</SelectItem>
                  <SelectItem value="GBP">GBP</SelectItem>
                  <SelectItem value="CAD">CAD</SelectItem>
                  <SelectItem value="AUD">AUD</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          {/* Stage */}
          <div className="space-y-2">
            <Label htmlFor="stage">Stage</Label>
            <Select
              value={formData.stageId}
              onValueChange={(value) => setFormData({ ...formData, stageId: value })}
            >
              <SelectTrigger id="stage">
                <SelectValue placeholder="Select stage" />
              </SelectTrigger>
              <SelectContent>
                {stages.map((stage) => (
                  <SelectItem key={stage.id} value={stage.id}>
                    <div className="flex items-center gap-2">
                      <div
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: stage.color }}
                      />
                      {stage.name}
                    </div>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Contact */}
          <div className="space-y-2">
            <Label htmlFor="contact">Contact</Label>
            <Select
              value={formData.contactId}
              onValueChange={(value) => setFormData({ ...formData, contactId: value })}
            >
              <SelectTrigger id="contact">
                <SelectValue placeholder="Select contact (optional)" />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="">No contact</SelectItem>
                {contacts.map((contact) => (
                  <SelectItem key={contact.id} value={contact.id}>
                    {contact.full_name || contact.email || 'Unknown'}
                    {contact.company_name && (
                      <span className="text-muted-foreground ml-1">({contact.company_name})</span>
                    )}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Expected Close Date */}
          <div className="space-y-2">
            <Label htmlFor="expectedCloseDate">Expected Close Date</Label>
            <Input
              id="expectedCloseDate"
              type="date"
              value={formData.expectedCloseDate}
              onChange={(e) => setFormData({ ...formData, expectedCloseDate: e.target.value })}
            />
          </div>

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              placeholder="Add notes about this deal..."
              rows={3}
            />
          </div>

          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || !formData.name}>
              {isSubmitting && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              {isEditing ? 'Save Changes' : 'Create Deal'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
