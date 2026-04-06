import { useQuery } from '@tanstack/react-query';
import { Loader2 } from 'lucide-react';
import { useCallback, useEffect, useMemo, useState } from 'react';

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
import { crmApi } from '@/lib/api';
import { crmKeys } from '@/lib/query-keys';
import type {
  CreateCrmDeal,
  CrmDealWithContact,
  CrmPipelineStage,
  UpdateCrmDeal,
} from '@/types/crm';

interface CrmDealFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  organizationId: string;
  pipelineId: string;
  stages: CrmPipelineStage[];
  deal?: CrmDealWithContact;
  initialStageId?: string;
  onSubmit: (data: CreateCrmDeal | UpdateCrmDeal) => Promise<void>;
}

export function CrmDealForm({
  open,
  onOpenChange,
  organizationId,
  pipelineId,
  stages,
  deal,
  initialStageId,
  onSubmit,
}: CrmDealFormProps) {
  const isEditing = !!deal;
  const [isSubmitting, setIsSubmitting] = useState(false);

  const getInitialFormData = useCallback(
    () => ({
      name: deal?.name ?? '',
      description: deal?.description ?? '',
      amount:
        deal?.amount != null && Number.isFinite(deal.amount)
          ? deal.amount.toString()
          : '',
      currency: deal?.currency ?? 'USD',
      stageId: deal?.crm_stage_id ?? initialStageId ?? stages[0]?.id ?? '',
      contactId: deal?.crm_contact_id ?? '',
      expectedCloseDate: deal?.expected_close_date
        ? new Date(deal.expected_close_date).toISOString().split('T')[0]
        : '',
      tags: deal?.tags ? (typeof deal.tags === 'string' ? deal.tags : '') : '',
      lostReason: deal?.lost_reason ?? '',
      winReason: deal?.win_reason ?? '',
    }),
    [deal, initialStageId, stages]
  );

  const [formData, setFormData] = useState(getInitialFormData);

  // Reset form when deal/dialog changes
  useEffect(() => {
    if (open) {
      setFormData(getInitialFormData());
      setContactSearch('');
    }
  }, [open, deal?.id, getInitialFormData]);

  // Fetch contacts for the dropdown
  const { data: contacts = [] } = useQuery({
    queryKey: crmKeys.contacts(organizationId),
    queryFn: () => crmApi.listContacts(organizationId, { limit: 100 }),
    enabled: open && !!organizationId,
  });

  const [contactSearch, setContactSearch] = useState('');
  const filteredContacts = useMemo(() => {
    if (!contactSearch) return contacts;
    const q = contactSearch.toLowerCase();
    return contacts.filter(
      (c) =>
        c.full_name?.toLowerCase().includes(q) ||
        c.email?.toLowerCase().includes(q) ||
        c.company_name?.toLowerCase().includes(q)
    );
  }, [contacts, contactSearch]);

  const handleFieldChange = useCallback(
    (field: string) =>
      (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
        setFormData((prev) => ({ ...prev, [field]: e.target.value }));
      },
    []
  );

  const handleSelectChange = useCallback(
    (field: string) => (value: string) => {
      setFormData((prev) => ({ ...prev, [field]: value }));
    },
    []
  );

  const handleContactChange = useCallback((value: string) => {
    setFormData((prev) => ({
      ...prev,
      contactId: value === '__none__' ? '' : value,
    }));
  }, []);

  const handleCancel = useCallback(() => {
    onOpenChange(false);
  }, [onOpenChange]);

  const handleSubmit = useCallback(
    async (e: React.FormEvent) => {
      e.preventDefault();
      setIsSubmitting(true);

      const parseAmount = (value: string): number | undefined => {
        if (!value) return undefined;
        const num = parseFloat(value);
        if (!Number.isFinite(num)) return undefined;
        return num;
      };

      try {
        const parsedTags = formData.tags
          ? formData.tags
              .split(',')
              .map((t) => t.trim())
              .filter(Boolean)
          : undefined;

        if (isEditing) {
          const updateData: UpdateCrmDeal = {
            name: formData.name || undefined,
            description: formData.description || undefined,
            amount: parseAmount(formData.amount),
            currency: formData.currency || undefined,
            crm_stage_id: formData.stageId || undefined,
            crm_contact_id: formData.contactId || undefined,
            expected_close_date: formData.expectedCloseDate || undefined,
            tags: parsedTags,
            lost_reason: formData.lostReason || undefined,
            win_reason: formData.winReason || undefined,
          };
          await onSubmit(updateData);
        } else {
          const createData: CreateCrmDeal = {
            organization_id: organizationId,
            crm_pipeline_id: pipelineId,
            crm_stage_id: formData.stageId || undefined,
            crm_contact_id: formData.contactId || undefined,
            name: formData.name,
            description: formData.description || undefined,
            amount: parseAmount(formData.amount),
            currency: formData.currency || undefined,
            expected_close_date: formData.expectedCloseDate || undefined,
            tags: parsedTags,
          };
          await onSubmit(createData);
        }
        onOpenChange(false);
      } catch (error) {
        console.error('Failed to save deal:', error);
      } finally {
        setIsSubmitting(false);
      }
    },
    [formData, isEditing, organizationId, pipelineId, onSubmit, onOpenChange]
  );

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[550px]">
        <DialogHeader>
          <DialogTitle>{isEditing ? 'Edit Deal' : 'Create Deal'}</DialogTitle>
          <DialogDescription>
            {isEditing
              ? 'Update the deal information below.'
              : 'Add a new deal to track in your pipeline.'}
          </DialogDescription>
        </DialogHeader>

        <form
          onSubmit={handleSubmit}
          className="space-y-4 max-h-[60vh] overflow-y-auto pr-1"
        >
          {/* Deal Name */}
          <div className="space-y-2">
            <Label htmlFor="name">Deal Name *</Label>
            <Input
              id="name"
              value={formData.name}
              onChange={handleFieldChange('name')}
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
                onChange={handleFieldChange('amount')}
                placeholder="0"
                min="0"
                step="0.01"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="currency">Currency</Label>
              <Select
                value={formData.currency}
                onValueChange={handleSelectChange('currency')}
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
              onValueChange={handleSelectChange('stageId')}
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
              value={formData.contactId || '__none__'}
              onValueChange={handleContactChange}
            >
              <SelectTrigger id="contact">
                <SelectValue placeholder="Select contact (optional)" />
              </SelectTrigger>
              <SelectContent>
                <div className="px-2 pb-2">
                  <Input
                    placeholder="Search contacts..."
                    value={contactSearch}
                    onChange={(e) => setContactSearch(e.target.value)}
                    className="h-8"
                    onKeyDown={(e) => e.stopPropagation()}
                  />
                </div>
                <SelectItem value="__none__">No contact</SelectItem>
                {filteredContacts.map((contact) => (
                  <SelectItem key={contact.id} value={contact.id}>
                    {contact.full_name || contact.email || 'Unknown'}
                    {contact.company_name ? ` (${contact.company_name})` : ''}
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
              onChange={handleFieldChange('expectedCloseDate')}
            />
          </div>

          {/* Description */}
          <div className="space-y-2">
            <Label htmlFor="description">Description</Label>
            <Textarea
              id="description"
              value={formData.description}
              onChange={handleFieldChange('description')}
              placeholder="Add notes about this deal..."
              rows={3}
            />
          </div>

          {/* Tags */}
          <div className="space-y-2">
            <Label htmlFor="tags">Tags</Label>
            <Input
              id="tags"
              value={formData.tags}
              onChange={handleFieldChange('tags')}
              placeholder="Comma-separated tags, e.g. enterprise, urgent"
            />
          </div>

          {/* Win/Lost Reason (edit mode only) */}
          {isEditing && (
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="winReason">Win Reason</Label>
                <Input
                  id="winReason"
                  value={formData.winReason}
                  onChange={handleFieldChange('winReason')}
                  placeholder="Why was this deal won?"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="lostReason">Lost Reason</Label>
                <Input
                  id="lostReason"
                  value={formData.lostReason}
                  onChange={handleFieldChange('lostReason')}
                  placeholder="Why was this deal lost?"
                />
              </div>
            </div>
          )}

          <DialogFooter>
            <Button type="button" variant="outline" onClick={handleCancel}>
              Cancel
            </Button>
            <Button type="submit" disabled={isSubmitting || !formData.name}>
              {isSubmitting && (
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
              )}
              {isEditing ? 'Save Changes' : 'Create Deal'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
