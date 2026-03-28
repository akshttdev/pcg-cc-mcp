import { useMutation,useQueryClient } from '@tanstack/react-query';
import { Calendar, CheckCircle2, Copy, ExternalLink, Link2, Loader2, Presentation, Receipt, Share2, Trophy, Wand2 } from 'lucide-react';
import { useState } from 'react';
import { deck as tid } from 'shared/testids';
import { toast } from 'sonner';

import MarkdownRenderer from '@/components/ui/markdown-renderer';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { EmptyState } from '@/components/ui/empty-state';
import { SectionHeader } from '@/components/ui/section-header';
import { crmDealsApi } from '@/lib/api/crm';
import { crmKeys } from '@/lib/query-keys';
import type { CrmDealWithContact } from '@/types/crm';

/** Stages where invoice sending is gated (before deck is presented) */
const PRE_INVOICE_STAGES = ['lead', 'intel', 'business_analysis', 'discovery', 'build_proposal', 'proposal'] as const;

interface DeckTabProps {
  deal: CrmDealWithContact;
  onMarkWon?: () => void;
}

export function DeckTab({ deal, onMarkWon }: DeckTabProps) {
  const qc = useQueryClient();
  const [invoiceSending, setInvoiceSending] = useState(false);
  const [markingWon, setMarkingWon] = useState(false);
  const [reviewLinkCopied, setReviewLinkCopied] = useState(false);

  const generateDeck = useMutation({
    mutationFn: () => crmDealsApi.generateDeck(deal.id),
    onSuccess: () => {
      toast.success('Deck script generated successfully');
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
    },
    onError: (e: Error) => toast.error(e.message ?? 'Deck generation failed — check that the LLM backend is available and try again.'),
  });

  const sendInvoice = useMutation({
    mutationFn: () => crmDealsApi.sendInvoice(deal.id, { due_days: 14 }),
    onSuccess: (res) => {
      toast.success(`Invoice ${res.invoice_number} sent (${new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', minimumFractionDigits: 0 }).format(res.amount_usd)})`);
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
      setInvoiceSending(false);
    },
    onError: () => { toast.error('Failed to send invoice — verify the deal amount is set and try again.'); setInvoiceSending(false); },
  });

  const markWon = useMutation({
    mutationFn: () => crmDealsApi.markWon(deal.id),
    onSuccess: (res) => {
      toast.success(`Deal won! Project "${res.project_name}" created with ${res.tasks_created} tasks.`);
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
      setMarkingWon(false);
      onMarkWon?.();
    },
    onError: () => { toast.error('Failed to mark deal as won — check that the proposal is approved and try again.'); setMarkingWon(false); },
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
        <SectionHeader icon={Presentation} title="Sales Deck" />

        {!deal.deck_url ? (
          <EmptyState
            variant="branded"
            borderColor="pink-500"
            bgTint="pink-500"
            icon={Presentation}
            title="No deck generated yet"
            description={`The deck agent will create a branded slide-by-slide deck script from your approved proposal.${!deal.proposal_text ? ' Generate and approve the proposal first.' : ''}`}
            action={{ label: generateDeck.isPending ? 'Generating...' : 'Generate Deck', onClick: () => generateDeck.mutate(), 'data-testid': tid.generate }}
          />
        ) : (
          <div className="space-y-3">
            <div className="flex items-center gap-2">
              <div className="h-2 w-2 rounded-full bg-pink-500" />
              <span className="text-xs text-muted-foreground">Deck ready</span>
              <a
                href={deal.deck_url}
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-1 ml-auto text-xs text-pink-400 hover:text-pink-300 hover:underline"
              >
                <ExternalLink className="h-3 w-3" /> Open deck
              </a>
              <Button variant="ghost" size="sm" className="h-6 gap-1 text-xs" onClick={() => generateDeck.mutate()} disabled={generateDeck.isPending}>
                {generateDeck.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Wand2 className="h-3 w-3" />}
                Regenerate
              </Button>
            </div>

            {/* PDF embed if the deck_url is a PDF */}
            {deal.deck_url.toLowerCase().includes('.pdf') && (() => {
              const isDropbox = deal.deck_url!.includes('dropbox.com');
              const embedUrl = isDropbox
                ? deal.deck_url!.replace('www.dropbox.com', 'dl.dropboxusercontent.com').replace('?dl=0', '').replace('&dl=0', '')
                : deal.deck_url!;
              return (
                <div className="rounded-lg overflow-hidden border border-border/40" style={{ height: '420px' }}>
                  <iframe src={embedUrl} title="Sales Deck" className="w-full h-full" style={{ border: 'none' }} />
                </div>
              );
            })()}

            {deckScript && (
              <div className="rounded-lg border bg-muted/30 p-4 text-xs leading-relaxed max-h-[320px] overflow-y-auto space-y-2">
                <MarkdownRenderer content={deckScript} enableCopyButton />
              </div>
            )}

            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                className="gap-1.5 text-xs border-indigo-500/40 text-indigo-400 hover:bg-indigo-950/30"
                onClick={() => {
                  navigator.clipboard.writeText(deal.deck_url!);
                  setReviewLinkCopied(true);
                  toast.success('Deck link copied');
                  setTimeout(() => setReviewLinkCopied(false), 3000);
                }}
              >
                {reviewLinkCopied ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Share2 className="h-3.5 w-3.5" />}
                {reviewLinkCopied ? 'Copied!' : 'Copy Link'}
              </Button>
            </div>
          </div>
        )}
      </div>

      {/* Divider */}
      <div className="border-t" />

      {/* Presentation Section — track deck presentation to client */}
      <PresentationSection deal={deal} />

      {/* Divider */}
      <div className="border-t" />

      {/* Invoice Section — Present & Invoice stage action */}
      <div className="space-y-3">
        <SectionHeader icon={Receipt} title="Invoice" />

        {deal.invoice_id ? (
          <div className="flex items-center gap-2 text-xs text-green-500">
            <Receipt className="h-3.5 w-3.5" />
            Invoice sent — awaiting payment
          </div>
        ) : (
          <div className="space-y-2">
            <p className="text-xs text-muted-foreground">
              Send the invoice after presenting the deck to the client. Amount: {deal.amount ? new Intl.NumberFormat('en-US', { style: 'currency', currency: deal.currency || 'USD', minimumFractionDigits: 0 }).format(deal.amount) : 'not set'}.
            </p>
            {!invoiceSending ? (() => {
                const dealStage = (deal.stage ?? '').toLowerCase().replace(/\s+/g, '_');
                const cf = (() => { try { return typeof deal.custom_fields === 'string' ? JSON.parse(deal.custom_fields) : (deal.custom_fields ?? {}); } catch { return {}; } })();
                const presentationDone = cf.presentation_status === 'presented';
                const isTooEarly = PRE_INVOICE_STAGES.some((s) => s === dealStage) || !presentationDone;
                return (
                  <div>
                    <Button
                      variant="outline"
                      size="sm"
                      className="gap-1.5 text-xs border-blue-500/40 text-blue-400 hover:bg-blue-950/30"
                      onClick={() => setInvoiceSending(true)}
                      disabled={!deal.amount || isTooEarly}
                      data-testid={tid.sendInvoice}
                      title={isTooEarly ? 'Available after presenting to client' : undefined}
                    >
                      <Receipt className="h-3.5 w-3.5" />
                      Send Invoice
                    </Button>
                    {isTooEarly && (
                      <p className="text-xs text-muted-foreground mt-1">Available after presenting to client</p>
                    )}
                  </div>
                );
              })() : (
              <div className="flex items-center gap-2">
                <Button
                  size="sm"
                  className="gap-1.5 text-xs bg-blue-600 hover:bg-blue-700 text-white"
                  onClick={() => sendInvoice.mutate()}
                  disabled={sendInvoice.isPending}
                  data-testid={tid.confirmSendInvoice}
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
        <SectionHeader icon={Trophy} title="Close Deal" />

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
                data-testid={tid.markWon}
              >
                <Trophy className="h-3.5 w-3.5" />
                Mark Won
              </Button>
            ) : (
              <div className="rounded-lg border border-yellow-500/30 bg-yellow-500/5 p-3 space-y-2">
                <p className="text-xs font-medium text-yellow-400">Confirm deal is won?</p>
                <p className="text-xs text-muted-foreground">This will create a client, project, and all tasks from the proposal.</p>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    className="gap-1.5 text-xs bg-yellow-500 hover:bg-yellow-600 text-black font-semibold"
                    onClick={() => markWon.mutate()}
                    disabled={markWon.isPending}
                    data-testid={tid.confirmWon}
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

      {/* Invite Link — appears after deal is won */}
      {deal.won_at && deal.crm_contact_id && (
        <>
          <div className="border-t" />
          <InviteLinkSection deal={deal} />
        </>
      )}
    </div>
  );
}

// ── InviteLinkSection ────────────────────────────────────────────────────────

function InviteLinkSection({ deal }: { deal: CrmDealWithContact }) {
  const qc = useQueryClient();
  const [inviteCopied, setInviteCopied] = useState(false);

  const existingToken = (() => {
    try {
      const cf = typeof deal.custom_fields === 'string' ? JSON.parse(deal.custom_fields) : deal.custom_fields;
      return cf?.invite_token as string | undefined;
    } catch { return undefined; }
  })();

  const generateInvite = useMutation({
    mutationFn: () => crmDealsApi.generateInvite(deal.id),
    onSuccess: (res) => {
      const fullUrl = `${window.location.origin}${res.invite_url}`;
      navigator.clipboard.writeText(fullUrl);
      setInviteCopied(true);
      toast.success('Invite link copied to clipboard', {
        description: res.contact_email ? `For ${res.contact_email}` : undefined,
      });
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
      setTimeout(() => setInviteCopied(false), 3000);
    },
    onError: (e: Error) => toast.error(e.message ?? 'Failed to generate invite link'),
  });

  return (
    <div className="space-y-3">
      <SectionHeader icon={Link2} title="Client Invitation" />

      {existingToken ? (
        <div className="space-y-2">
          <div className="flex items-center gap-2 text-xs text-emerald-500">
            <CheckCircle2 className="h-3.5 w-3.5" />
            Invite link generated
          </div>
          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              className="gap-1.5 text-xs border-emerald-500/40 text-emerald-400 hover:bg-emerald-950/30"
              onClick={() => {
                const url = `${window.location.origin}/invite/${existingToken}`;
                navigator.clipboard.writeText(url);
                setInviteCopied(true);
                toast.success('Invite link copied');
                setTimeout(() => setInviteCopied(false), 3000);
              }}
            >
              {inviteCopied ? <CheckCircle2 className="h-3.5 w-3.5" /> : <Copy className="h-3.5 w-3.5" />}
              {inviteCopied ? 'Copied!' : 'Copy Link'}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="gap-1.5 text-xs"
              onClick={() => generateInvite.mutate()}
              disabled={generateInvite.isPending}
            >
              {generateInvite.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <Link2 className="h-3 w-3" />}
              Regenerate
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-2">
          <p className="text-xs text-muted-foreground">
            Generate an invite link for the client to access their project dashboard.
          </p>
          <Button
            variant="outline"
            size="sm"
            className="gap-1.5 text-xs border-emerald-500/40 text-emerald-400 hover:bg-emerald-950/30"
            onClick={() => generateInvite.mutate()}
            disabled={generateInvite.isPending}
            data-testid={tid.generateInvite}
          >
            {generateInvite.isPending ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : <Link2 className="h-3.5 w-3.5" />}
            {generateInvite.isPending ? 'Generating...' : 'Generate Invite Link'}
          </Button>
        </div>
      )}
    </div>
  );
}

// ── Presentation Tracking Section ──────────────────────────────────────────

const PRESENTATION_STATUSES = ['pending', 'scheduled', 'presented'] as const;

function PresentationSection({ deal }: { deal: CrmDealWithContact }) {
  const qc = useQueryClient();
  const customFields = (() => {
    try {
      return typeof deal.custom_fields === 'string'
        ? JSON.parse(deal.custom_fields)
        : (deal.custom_fields ?? {});
    } catch { return {}; }
  })();

  const [status, setStatus] = useState<string>(customFields.presentation_status ?? 'pending');
  const [date, setDate] = useState<string>(customFields.presentation_date ?? '');
  const [dirty, setDirty] = useState(false);

  const saveMutation = useMutation({
    mutationFn: () =>
      crmDealsApi.updateDeal(deal.id, {
        custom_fields: {
          ...customFields,
          presentation_status: status,
          presentation_date: date || undefined,
        },
      }),
    onSuccess: () => {
      toast.success('Presentation tracking saved');
      qc.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
      qc.invalidateQueries({ queryKey: crmKeys.kanbanLegacy() });
      setDirty(false);
    },
    onError: () => toast.error('Failed to save presentation tracking'),
  });

  return (
    <div className="space-y-3">
      <SectionHeader icon={Calendar} title="Presentation" />
      <div className="space-y-3">
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted-foreground w-16">Status:</span>
          <div className="flex gap-1.5">
            {PRESENTATION_STATUSES.map((s) => (
              <Badge
                key={s}
                data-testid={s === status ? tid.presentationStatus : undefined}
                variant={s === status ? 'default' : 'outline'}
                className={`cursor-pointer capitalize text-xs ${s === status && s === 'presented' ? 'bg-green-600' : ''}`}
                onClick={() => { setStatus(s); setDirty(true); }}
              >
                {s}
              </Badge>
            ))}
          </div>
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted-foreground w-16">Date:</span>
          <input
            type="date"
            data-testid={tid.presentationDate}
            value={date}
            onChange={(e) => { setDate(e.target.value); setDirty(true); }}
            className="h-7 rounded border border-border bg-background px-2 text-xs"
          />
        </div>
        {dirty && (
          <Button
            size="sm"
            variant="outline"
            className="gap-1.5 text-xs"
            onClick={() => saveMutation.mutate()}
            disabled={saveMutation.isPending}
            data-testid={tid.presentationSave}
          >
            {saveMutation.isPending ? <Loader2 className="h-3 w-3 animate-spin" /> : <CheckCircle2 className="h-3 w-3" />}
            Save
          </Button>
        )}
      </div>
    </div>
  );
}
