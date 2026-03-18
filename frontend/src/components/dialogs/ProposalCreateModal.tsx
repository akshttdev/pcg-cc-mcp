import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
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
import { toast } from 'sonner';
import { proposalsApi, type DealType } from '@/lib/api';
import { businessKeys } from '@/lib/query-keys';

interface Props {
  open: boolean;
  onClose: () => void;
  /** Pre-fill company_id when opened from a company profile */
  defaultCompanyId?: string;
  /** Pre-fill lead_id when opened from a person profile */
  defaultLeadId?: string;
  /** Pre-fill organization_id when opened from an org profile */
  defaultOrgId?: string;
}

const DEAL_TYPES: { value: DealType; label: string }[] = [
  { value: 'one-off',  label: 'One-off project' },
  { value: 'retainer', label: 'Retainer / monthly' },
  { value: 'hybrid',   label: 'Hybrid' },
];

export function ProposalCreateModal({
  open,
  onClose,
  defaultCompanyId,
  defaultLeadId,
  defaultOrgId,
}: Props) {
  const queryClient = useQueryClient();
  const [saving, setSaving] = useState(false);

  const [title,       setTitle]       = useState('');
  const [description, setDescription] = useState('');
  const [dealType,    setDealType]    = useState<DealType>('one-off');
  const [quoteVibe,   setQuoteVibe]   = useState('');

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!title.trim()) { toast.error('Title is required'); return; }

    setSaving(true);
    try {
      await proposalsApi.create({
        title: title.trim(),
        description: description.trim() || undefined,
        deal_type: dealType,
        quote_amount_vibe: quoteVibe ? parseInt(quoteVibe, 10) : undefined,
        company_id: defaultCompanyId,
        lead_id: defaultLeadId,
        organization_id: defaultOrgId,
      });
      toast.success('Proposal created');
      queryClient.invalidateQueries({ queryKey: businessKeys.proposals() });
      if (defaultCompanyId) {
        queryClient.invalidateQueries({ queryKey: businessKeys.companyProposals(defaultCompanyId!) });
      }
      handleClose();
    } catch {
      toast.error('Failed to create proposal');
    } finally {
      setSaving(false);
    }
  }

  function handleClose() {
    setTitle('');
    setDescription('');
    setDealType('one-off');
    setQuoteVibe('');
    onClose();
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && handleClose()}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>New Proposal</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit} className="space-y-4 pt-2">
          <div className="space-y-1">
            <Label htmlFor="proposal-title">Title *</Label>
            <Input
              id="proposal-title"
              placeholder="e.g. Brand Strategy Retainer Q2"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
            />
          </div>

          <div className="space-y-1">
            <Label htmlFor="proposal-desc">Description</Label>
            <Textarea
              id="proposal-desc"
              rows={3}
              placeholder="Scope of work, deliverables, timeline…"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1">
              <Label>Deal type</Label>
              <Select value={dealType} onValueChange={(v) => setDealType(v as DealType)}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {DEAL_TYPES.map((d) => (
                    <SelectItem key={d.value} value={d.value}>{d.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-1">
              <Label htmlFor="proposal-vibe">Quote (VIBE)</Label>
              <Input
                id="proposal-vibe"
                type="number"
                min={0}
                placeholder="0"
                value={quoteVibe}
                onChange={(e) => setQuoteVibe(e.target.value)}
              />
            </div>
          </div>

          <div className="flex justify-end gap-2 pt-2">
            <Button type="button" variant="ghost" onClick={handleClose} disabled={saving}>
              Cancel
            </Button>
            <Button type="submit" disabled={saving}>
              {saving ? 'Creating…' : 'Create Proposal'}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
