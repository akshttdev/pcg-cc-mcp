import { useMutation,useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, Edit3, FileText, Loader2, Save, Wand2, X } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';

import { Button } from '@/components/ui/button';
import { EmptyState } from '@/components/ui/empty-state';
import { StatusBadge } from '@/components/ui/status-badge';
import { Textarea } from '@/components/ui/textarea';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys } from '@/lib/query-keys';
import { getStatusInfo } from '@/lib/status-utils';
import type { CrmDealWithContact } from '@/types/crm';

interface ProposalTabProps {
  deal: CrmDealWithContact;
}

export function ProposalTab({ deal }: ProposalTabProps) {
  const qc = useQueryClient();
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState(deal.proposal_text ?? '');

  const status = deal.proposal_status ?? 'draft';
  const statusInfo = getStatusInfo(status, 'proposal');

  const generate = useMutation({
    mutationFn: () => crmDealsApi.generateProposal(deal.id),
    onSuccess: (updatedDeal) => {
      if (updatedDeal.proposal_text) {
        toast.success('Proposal generated successfully');
      } else {
        toast.error('Proposal generation returned empty — the agent may need more context (intel, transcripts, or operator notes).');
      }
      // Update all kanban caches (legacy + org) so the panel re-renders with proposal text
      const patchKanban = (old: { stages: Array<{ deals: CrmDealWithContact[] }> } | undefined) => {
        if (!old) return old;
        return {
          ...old,
          stages: old.stages.map((s) => ({
            ...s,
            deals: s.deals.map((d) =>
              d.id === deal.id
                ? { ...d, proposal_text: updatedDeal.proposal_text, proposal_status: updatedDeal.proposal_status }
                : d
            ),
          })),
        };
      };
      qc.setQueriesData({ queryKey: crmKeys.kanbanAll() }, patchKanban);
      qc.setQueriesData({ queryKey: crmKeys.orgKanbanAll() }, patchKanban);
      qc.setQueriesData({ queryKey: crmKeys.kanbanLegacy() }, patchKanban);
      qc.invalidateQueries({ queryKey: crmKeys.dealLegacy(deal.id) });
    },
    onError: () => toast.error('Proposal generation failed — check that the LLM backend is available and try again.'),
  });

  const approve = useMutation({
    mutationFn: () => crmDealsApi.approveProposal(deal.id),
    onSuccess: () => {
      toast.success('Proposal approved — ready for Polish');
      const patchApprove = (old: { stages: Array<{ deals: CrmDealWithContact[] }> } | undefined) => {
        if (!old) return old;
        return {
          ...old,
          stages: old.stages.map((s) => ({
            ...s,
            deals: s.deals.map((d) =>
              d.id === deal.id ? { ...d, proposal_status: 'approved' } : d
            ),
          })),
        };
      };
      qc.setQueriesData({ queryKey: crmKeys.kanbanAll() }, patchApprove);
      qc.setQueriesData({ queryKey: crmKeys.orgKanbanAll() }, patchApprove);
      qc.setQueriesData({ queryKey: crmKeys.kanbanLegacy() }, patchApprove);
    },
    onError: () => toast.error('Failed to approve proposal'),
  });

  const save = useMutation({
    mutationFn: () => crmDealsApi.updateDeal(deal.id, { proposal_text: editText }),
    onSuccess: () => {
      toast.success('Proposal saved');
      setEditing(false);
      const patchSave = (old: { stages: Array<{ deals: CrmDealWithContact[] }> } | undefined) => {
        if (!old) return old;
        return {
          ...old,
          stages: old.stages.map((s) => ({
            ...s,
            deals: s.deals.map((d) =>
              d.id === deal.id ? { ...d, proposal_text: editText } : d
            ),
          })),
        };
      };
      qc.setQueriesData({ queryKey: crmKeys.kanbanAll() }, patchSave);
      qc.setQueriesData({ queryKey: crmKeys.orgKanbanAll() }, patchSave);
      qc.setQueriesData({ queryKey: crmKeys.kanbanLegacy() }, patchSave);
    },
    onError: () => toast.error('Failed to save proposal'),
  });

  return (
    <div className="p-5 space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <FileText className="h-4 w-4 text-amber-400" />
          <h3 className="font-semibold text-sm">Proposal</h3>
          <StatusBadge
            status={statusInfo.variant}
            label={statusInfo.label}
            icon={statusInfo.icon}
            size="sm"
          />
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
        <EmptyState
          variant="branded"
          borderColor="amber-500"
          bgTint="amber-500"
          icon={FileText}
          title="No proposal yet"
          description="The proposal agent will synthesize a proposal from the business report, discovery transcript, and intel wikis."
          action={{ label: generate.isPending ? 'Generating...' : 'Generate Proposal', onClick: () => generate.mutate() }}
        />
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
