import { useState } from 'react';
import { useQueryClient, useMutation } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import { Loader2, FileText, Wand2, CheckCircle2, Edit3, Save, X } from 'lucide-react';
import { toast } from 'sonner';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';
import { cn } from '@/lib/utils';

interface ProposalTabProps {
  deal: CrmDealWithContact;
}

const STATUS_LABELS: Record<string, { label: string; color: string }> = {
  draft:    { label: 'Draft',    color: 'bg-amber-500/15 text-amber-500 border-amber-500/30' },
  approved: { label: 'Approved', color: 'bg-green-500/15 text-green-500 border-green-500/30' },
  sent:     { label: 'Sent',     color: 'bg-blue-500/15 text-blue-500 border-blue-500/30' },
};

export function ProposalTab({ deal }: ProposalTabProps) {
  const qc = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(deal.proposal_text ?? '');

  const status = deal.proposal_status ?? 'draft';
  const statusInfo = STATUS_LABELS[status] ?? STATUS_LABELS.draft;

  const generate = useMutation({
    mutationFn: () => crmDealsApi.generateProposal(deal.id),
    onSuccess: () => {
      toast.success('Cash generated your proposal');
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.deal(deal.id) });
    },
    onError: () => toast.error('Proposal generation failed'),
  });

  const approve = useMutation({
    mutationFn: () => crmDealsApi.approveProposal(deal.id),
    onSuccess: () => {
      toast.success('Proposal approved — ready for Polish');
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
    },
    onError: () => toast.error('Failed to approve proposal'),
  });

  const save = useMutation({
    mutationFn: () => crmDealsApi.updateDeal(deal.id, { proposal_text: editText }),
    onSuccess: () => {
      toast.success('Proposal saved');
      setEditing(false);
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
    },
    onError: () => toast.error('Failed to save proposal'),
  });

  return (
    <div className="p-5 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-amber-400" />
          <h3 className="font-semibold text-sm">Proposal</h3>
          <Badge variant="outline" className={cn('text-[10px] px-1.5 py-0.5 border', statusInfo.color)}>
            {statusInfo.label}
          </Badge>
        </div>
        <div className="flex items-center gap-1.5">
          {deal.proposal_text && !editing && (
            <Button variant="ghost" size="sm" className="h-7 gap-1 text-xs" onClick={() => { setEditText(deal.proposal_text ?? ''); setEditing(true); }}>
              <Edit3 className="h-3 w-3" /> Edit
            </Button>
          )}
          {editing && (
            <>
              <Button variant="ghost" size="sm" className="h-7 gap-1 text-xs" onClick={() => setEditing(false)}>
                <X className="h-3 w-3" /> Cancel
              </Button>
              <Button size="sm" className="h-7 gap-1 text-xs" onClick={() => save.mutate()} disabled={save.isPending}>
                {save.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Save className="h-3 w-3" />} Save
              </Button>
            </>
          )}
        </div>
      </div>

      {/* No proposal yet */}
      {!deal.proposal_text && !editing && (
        <div className="rounded-xl border border-dashed border-amber-500/30 bg-amber-500/5 p-6 text-center space-y-3">
          <FileText className="h-8 w-8 mx-auto text-amber-400/60" />
          <div>
            <p className="text-sm font-medium">No proposal yet</p>
            <p className="text-xs text-muted-foreground mt-1">
              Cash will synthesize a proposal from the business report, discovery transcript, and intel wikis.
            </p>
          </div>
          <Button
            size="sm"
            className="gap-1.5 bg-amber-500 hover:bg-amber-600 text-white"
            onClick={() => generate.mutate()}
            disabled={generate.isPending}
          >
            {generate.isPending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Wand2 className="h-3.5 w-3.5" />}
            {generate.isPending ? 'Cash is writing…' : 'Generate Proposal'}
          </Button>
        </div>
      )}

      {/* Proposal text — view mode */}
      {deal.proposal_text && !editing && (
        <div className="space-y-3">
          <div className="rounded-lg border bg-muted/30 p-4 text-xs leading-relaxed font-mono whitespace-pre-wrap max-h-[420px] overflow-y-auto">
            {deal.proposal_text}
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5 text-xs"
              onClick={() => generate.mutate()}
              disabled={generate.isPending}
            >
              {generate.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Wand2 className="h-3 w-3" />}
              Regenerate
            </Button>
            {status !== 'approved' && (
              <Button
                size="sm"
                className="gap-1.5 text-xs bg-green-600 hover:bg-green-700 text-white"
                onClick={() => approve.mutate()}
                disabled={approve.isPending}
              >
                {approve.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <CheckCircle2 className="h-3 w-3" />}
                Approve Proposal
              </Button>
            )}
            {status === 'approved' && (
              <p className="text-xs text-green-500 flex items-center gap-1">
                <CheckCircle2 className="h-3 w-3" /> Approved — ready for deck generation
              </p>
            )}
          </div>
        </div>
      )}

      {/* Proposal text — edit mode */}
      {editing && (
        <Textarea
          value={editText}
          onChange={(e) => setEditText(e.target.value)}
          className="min-h-[380px] font-mono text-xs"
          placeholder="Write or paste your proposal here…"
        />
      )}
    </div>
  );
}
