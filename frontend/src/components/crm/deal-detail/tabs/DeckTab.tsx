import { useState } from 'react';
import { useQueryClient, useMutation } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Loader2, Presentation, Wand2, Receipt, Trophy } from 'lucide-react';
import { toast } from 'sonner';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';

interface DeckTabProps {
  deal: CrmDealWithContact;
  onMarkWon?: () => void;
}

export function DeckTab({ deal, onMarkWon }: DeckTabProps) {
  const qc = useQueryClient();
  const [invoiceSending, setInvoiceSending] = useState(false);
  const [markingWon, setMarkingWon] = useState(false);

  const generateDeck = useMutation({
    mutationFn: () => crmDealsApi.generateDeck(deal.id),
    onSuccess: () => {
      toast.success('Lux generated your deck script');
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
    },
    onError: (e: Error) => toast.error(e.message ?? 'Deck generation failed'),
  });

  const sendInvoice = useMutation({
    mutationFn: () => crmDealsApi.sendInvoice(deal.id, { due_days: 14 }),
    onSuccess: (res) => {
      toast.success(`Invoice ${res.invoice_number} sent ($${res.amount_usd.toFixed(0)})`);
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
      setInvoiceSending(false);
    },
    onError: () => { toast.error('Failed to send invoice'); setInvoiceSending(false); },
  });

  const markWon = useMutation({
    mutationFn: () => crmDealsApi.markWon(deal.id),
    onSuccess: (res) => {
      toast.success(`🏆 Deal Won! Project "${res.project_name}" created with ${res.tasks_created} tasks.`);
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
      setMarkingWon(false);
      onMarkWon?.();
    },
    onError: () => { toast.error('Failed to mark deal as Won'); setMarkingWon(false); },
  });

  const deckScript = (() => {
    if (!deal.custom_fields) return null;
    try {
      const cf = JSON.parse(deal.custom_fields as unknown as string);
      return cf.script as string | undefined;
    } catch { return null; }
  })();

  return (
    <div className="p-5 space-y-5">
      {/* Deck Section */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Presentation className="h-4 w-4 text-pink-400" />
          <h3 className="font-semibold text-sm">Sales Deck</h3>
        </div>

        {!deal.deck_url ? (
          <div className="rounded-xl border border-dashed border-pink-500/30 bg-pink-500/5 p-6 text-center space-y-3">
            <Presentation className="h-8 w-8 mx-auto text-pink-400/60" />
            <div>
              <p className="text-sm font-medium">No deck generated yet</p>
              <p className="text-xs text-muted-foreground mt-1">
                Lux will create a branded slide-by-slide deck script from your approved proposal.
                {!deal.proposal_text && (
                  <span className="block mt-1 text-amber-400">Generate and approve the proposal first.</span>
                )}
              </p>
            </div>
            <Button
              size="sm"
              className="gap-1.5 bg-pink-600 hover:bg-pink-700 text-white"
              onClick={() => generateDeck.mutate()}
              disabled={generateDeck.isPending || !deal.proposal_text}
            >
              {generateDeck.isPending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Wand2 className="h-3.5 w-3.5" />}
              {generateDeck.isPending ? 'Lux is designing…' : 'Generate Deck'}
            </Button>
          </div>
        ) : (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <div className="h-2 w-2 rounded-full bg-pink-500" />
              <span className="text-xs text-muted-foreground">Deck script ready</span>
              <Button variant="ghost" size="sm" className="h-6 gap-1 text-xs ml-auto" onClick={() => generateDeck.mutate()} disabled={generateDeck.isPending}>
                {generateDeck.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Wand2 className="h-3 w-3" />}
                Regenerate
              </Button>
            </div>
            {deckScript && (
              <div className="rounded-lg border bg-muted/30 p-4 text-xs leading-relaxed font-mono whitespace-pre-wrap max-h-[260px] overflow-y-auto">
                {deckScript}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Divider */}
      <div className="border-t" />

      {/* Invoice Section — Present stage action */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Receipt className="h-4 w-4 text-blue-400" />
          <h3 className="font-semibold text-sm">Invoice</h3>
        </div>

        {deal.invoice_id ? (
          <div className="flex items-center gap-2 text-xs text-green-500">
            <Receipt className="h-3.5 w-3.5" />
            Invoice sent — awaiting payment
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              Send the invoice after presenting the deck to the client. Amount: {deal.amount ? `$${deal.amount.toFixed(0)}` : 'not set'}.
            </p>
            {!invoiceSending ? (
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5 text-xs border-blue-500/40 text-blue-400 hover:bg-blue-950/30"
                onClick={() => setInvoiceSending(true)}
                disabled={!deal.amount}
              >
                <Receipt className="h-3.5 w-3.5" />
                Send Invoice
              </Button>
            ) : (
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  className="gap-1.5 text-xs bg-blue-600 hover:bg-blue-700 text-white"
                  onClick={() => sendInvoice.mutate()}
                  disabled={sendInvoice.isPending}
                >
                  {sendInvoice.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Receipt className="h-3 w-3" />}
                  {sendInvoice.isPending ? 'Sending…' : 'Confirm Send'}
                </Button>
                <Button variant="ghost" size="sm" className="text-xs" onClick={() => setInvoiceSending(false)}>Cancel</Button>
              </div>
            )}
          </div>
        )}
      </div>

      {/* Divider */}
      <div className="border-t" />

      {/* Won Section */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <Trophy className="h-4 w-4 text-yellow-400" />
          <h3 className="font-semibold text-sm">Close Deal</h3>
        </div>

        {deal.won_at ? (
          <div className="flex items-center gap-2 text-xs text-yellow-400">
            <Trophy className="h-3.5 w-3.5" />
            Won {new Date(deal.won_at).toLocaleDateString()}
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              Mark as Won when payment is confirmed. This auto-creates a client record, project, and tasks from the proposal deliverables.
            </p>
            {!markingWon ? (
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5 text-xs border-yellow-500/40 text-yellow-400 hover:bg-yellow-950/30"
                onClick={() => setMarkingWon(true)}
              >
                <Trophy className="h-3.5 w-3.5" />
                Mark Won
              </Button>
            ) : (
              <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/5 p-3 space-y-2">
                <p className="text-xs font-medium text-yellow-400">Confirm deal is won?</p>
                <p className="text-[11px] text-muted-foreground">This will create a client, project, and all tasks from the proposal.</p>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    className="gap-1.5 text-xs bg-yellow-500 hover:bg-yellow-600 text-black font-semibold"
                    onClick={() => markWon.mutate()}
                    disabled={markWon.isPending}
                  >
                    {markWon.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Trophy className="h-3 w-3" />}
                    {markWon.isPending ? 'Closing…' : 'Confirm Won'}
                  </Button>
                  <Button variant="ghost" size="sm" className="text-xs" onClick={() => setMarkingWon(false)}>Cancel</Button>
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
