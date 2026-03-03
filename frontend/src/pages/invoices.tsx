import { useState } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
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
import {
  DollarSign,
  Plus,
  CheckCircle,
  Clock,
  AlertCircle,
  Coins,
  TrendingUp,
  TrendingDown,
  Trash2,
} from 'lucide-react';
import { invoicesApi, type InvoiceRecord, type CreateInvoiceInput } from '@/lib/api';
import { formatDistanceToNow } from 'date-fns';

// ── Helpers ──────────────────────────────────────────────────────────────────

const VIBE_PER_USD = 100; // 1 USD = 100 VIBE

function fmtUsd(n: number) {
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(n);
}

function fmtVibe(n: number) {
  return `${n.toLocaleString()} VIBE`;
}

const STATUS_COLORS: Record<string, string> = {
  draft:    'bg-gray-100 text-gray-600',
  pending:  'bg-yellow-100 text-yellow-700',
  sent:     'bg-blue-100 text-blue-700',
  partial:  'bg-orange-100 text-orange-700',
  paid:     'bg-green-100 text-green-700',
  overdue:  'bg-red-100 text-red-700',
  cancelled:'bg-gray-100 text-gray-400',
};

const STATUSES = ['draft', 'pending', 'sent', 'partial', 'paid', 'overdue', 'cancelled'];

function StatusBadge({ status }: { status: string }) {
  return (
    <Badge className={`${STATUS_COLORS[status] ?? 'bg-gray-100 text-gray-600'} border-0 capitalize`}>
      {status}
    </Badge>
  );
}

// ── Invoice Card ──────────────────────────────────────────────────────────────

function InvoiceCard({ invoice, onStatusChange, onDelete }: {
  invoice: InvoiceRecord;
  onStatusChange: (id: string, status: string) => void;
  onDelete: (id: string) => void;
}) {
  const vibeEquiv = invoice.amount_vibe > 0 ? invoice.amount_vibe : Math.ceil(invoice.amount_usd * VIBE_PER_USD);
  return (
    <Card className="hover:shadow-md transition-shadow">
      <CardContent className="pt-4">
        <div className="flex items-start justify-between gap-2">
          <div className="flex-1 min-w-0">
            <p className="font-medium text-sm truncate">{invoice.title || invoice.invoice_number}</p>
            <p className="text-xs text-muted-foreground mt-0.5">
              #{invoice.invoice_number} · {formatDistanceToNow(new Date(invoice.created_at), { addSuffix: true })}
            </p>
          </div>
          <StatusBadge status={invoice.status} />
        </div>

        {/* USD + VIBE amounts */}
        <div className="mt-3 flex items-center gap-4">
          <div className="flex items-center gap-1">
            <DollarSign className="h-3.5 w-3.5 text-green-600" />
            <span className="text-sm font-semibold">{fmtUsd(invoice.amount_usd)}</span>
          </div>
          <div className="flex items-center gap-1 text-purple-600">
            <Coins className="h-3.5 w-3.5" />
            <span className="text-xs font-medium">{fmtVibe(vibeEquiv)}</span>
          </div>
        </div>

        {invoice.due_date && (
          <p className="text-xs text-muted-foreground mt-1">
            Due {new Date(invoice.due_date).toLocaleDateString()}
          </p>
        )}

        {/* Status actions */}
        <div className="mt-3 flex items-center gap-2 flex-wrap">
          {invoice.status !== 'paid' && (
            <Button
              size="sm"
              variant="outline"
              className="h-6 text-xs px-2 text-green-700 border-green-300 hover:bg-green-50"
              onClick={() => onStatusChange(invoice.id, 'paid')}
            >
              <CheckCircle className="h-3 w-3 mr-1" />
              Mark Paid
            </Button>
          )}
          {invoice.status === 'draft' && (
            <Button
              size="sm"
              variant="outline"
              className="h-6 text-xs px-2"
              onClick={() => onStatusChange(invoice.id, 'sent')}
            >
              <Clock className="h-3 w-3 mr-1" />
              Send
            </Button>
          )}
          <Button
            size="sm"
            variant="ghost"
            className="h-6 text-xs px-2 text-destructive hover:text-destructive ml-auto"
            onClick={() => onDelete(invoice.id)}
          >
            <Trash2 className="h-3 w-3" />
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}

// ── Create Dialog ─────────────────────────────────────────────────────────────

function CreateInvoiceDialog({ open, onClose, onCreated }: {
  open: boolean;
  onClose: () => void;
  onCreated: () => void;
}) {
  const [form, setForm] = useState<Partial<CreateInvoiceInput>>({ invoice_type: 'ar', currency: 'USD' });
  const [saving, setSaving] = useState(false);

  const handleSubmit = async () => {
    if (!form.amount_usd) return;
    setSaving(true);
    try {
      await invoicesApi.create({
        ...form,
        amount_usd: Number(form.amount_usd),
      });
      onCreated();
      onClose();
      setForm({ invoice_type: 'ar', currency: 'USD' });
    } finally {
      setSaving(false);
    }
  };

  const vibePreview = form.amount_usd ? Math.ceil(Number(form.amount_usd) * VIBE_PER_USD) : 0;

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>New Invoice</DialogTitle>
        </DialogHeader>
        <div className="space-y-4 py-2">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label className="text-xs">Type</Label>
              <Select
                value={form.invoice_type}
                onValueChange={(v) => setForm(f => ({ ...f, invoice_type: v as 'ar' | 'ap' }))}
              >
                <SelectTrigger className="h-8 text-sm mt-1">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="ar">AR — Accounts Receivable</SelectItem>
                  <SelectItem value="ap">AP — Accounts Payable</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div>
              <Label className="text-xs">Status</Label>
              <Select defaultValue="draft">

                <SelectTrigger className="h-8 text-sm mt-1">
                  <SelectValue placeholder="draft" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="draft">Draft</SelectItem>
                  <SelectItem value="pending">Pending</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>

          <div>
            <Label className="text-xs">Title</Label>
            <Input
              className="h-8 text-sm mt-1"
              placeholder="e.g. Social Campaign — March 2026"
              value={form.title ?? ''}
              onChange={e => setForm(f => ({ ...f, title: e.target.value }))}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label className="text-xs">Amount (USD)</Label>
              <Input
                className="h-8 text-sm mt-1"
                type="number"
                min="0"
                step="0.01"
                placeholder="0.00"
                value={form.amount_usd ?? ''}
                onChange={e => setForm(f => ({ ...f, amount_usd: parseFloat(e.target.value) || 0 }))}
              />
            </div>
            <div>
              <Label className="text-xs">VIBE Equivalent</Label>
              <div className="h-8 mt-1 flex items-center px-3 border rounded-md bg-muted text-sm text-purple-700 font-medium">
                <Coins className="h-3.5 w-3.5 mr-1.5" />
                {vibePreview.toLocaleString()}
              </div>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label className="text-xs">Issue Date</Label>
              <Input
                className="h-8 text-sm mt-1"
                type="date"
                value={form.issue_date ?? ''}
                onChange={e => setForm(f => ({ ...f, issue_date: e.target.value }))}
              />
            </div>
            <div>
              <Label className="text-xs">Due Date</Label>
              <Input
                className="h-8 text-sm mt-1"
                type="date"
                value={form.due_date ?? ''}
                onChange={e => setForm(f => ({ ...f, due_date: e.target.value }))}
              />
            </div>
          </div>
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={onClose}>Cancel</Button>
          <Button onClick={handleSubmit} disabled={saving || !form.amount_usd}>
            {saving ? 'Creating…' : 'Create Invoice'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

// ── Summary Stats ─────────────────────────────────────────────────────────────

function InvoiceSummary({ invoices, type }: { invoices: InvoiceRecord[]; type: 'ar' | 'ap' }) {
  const total = invoices.filter(i => i.invoice_type === type).reduce((s, i) => s + i.amount_usd, 0);
  const paid = invoices.filter(i => i.invoice_type === type && i.status === 'paid').reduce((s, i) => s + i.amount_usd, 0);
  const outstanding = total - paid;

  return (
    <div className="grid grid-cols-3 gap-3 mb-4">
      <Card>
        <CardContent className="pt-3 pb-3">
          <p className="text-xs text-muted-foreground">{type === 'ar' ? 'Total Billed' : 'Total Owed'}</p>
          <p className="text-lg font-bold">{fmtUsd(total)}</p>
          <p className="text-xs text-purple-600">{fmtVibe(Math.ceil(total * VIBE_PER_USD))}</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="pt-3 pb-3">
          <p className="text-xs text-muted-foreground">Collected</p>
          <p className="text-lg font-bold text-green-600">{fmtUsd(paid)}</p>
          <p className="text-xs text-purple-600">{fmtVibe(Math.ceil(paid * VIBE_PER_USD))}</p>
        </CardContent>
      </Card>
      <Card>
        <CardContent className="pt-3 pb-3">
          <p className="text-xs text-muted-foreground">Outstanding</p>
          <p className={`text-lg font-bold ${outstanding > 0 ? 'text-amber-600' : 'text-muted-foreground'}`}>
            {fmtUsd(outstanding)}
          </p>
          <p className="text-xs text-purple-600">{fmtVibe(Math.ceil(outstanding * VIBE_PER_USD))}</p>
        </CardContent>
      </Card>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function InvoicesPage() {
  const [tab, setTab] = useState<'ar' | 'ap'>('ar');
  const [createOpen, setCreateOpen] = useState(false);
  const [statusFilter, setStatusFilter] = useState<string>('');
  const queryClient = useQueryClient();

  const { data: invoices = [], isLoading } = useQuery<InvoiceRecord[]>({
    queryKey: ['invoices', statusFilter],
    queryFn: () => invoicesApi.list(statusFilter ? { status: statusFilter } : undefined),
  });

  const moveStatus = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      invoicesApi.moveStatus(id, status),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['invoices'] }),
  });

  const deleteInvoice = useMutation({
    mutationFn: (id: string) => invoicesApi.delete(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['invoices'] }),
  });

  const filtered = invoices.filter(i => i.invoice_type === tab);

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-bold flex items-center gap-2">
            <DollarSign className="h-5 w-5 text-green-600" />
            Invoices
          </h1>
          <p className="text-sm text-muted-foreground mt-0.5">
            Priced in USD · VIBE equivalent shown (1 USD = {VIBE_PER_USD} VIBE)
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Select value={statusFilter || 'all'} onValueChange={v => setStatusFilter(v === 'all' ? '' : v)}>
            <SelectTrigger className="h-8 w-32 text-xs">
              <SelectValue placeholder="All statuses" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All statuses</SelectItem>
              {STATUSES.map(s => <SelectItem key={s} value={s} className="capitalize">{s}</SelectItem>)}
            </SelectContent>
          </Select>
          <Button size="sm" onClick={() => setCreateOpen(true)}>
            <Plus className="h-3.5 w-3.5 mr-1.5" />
            New Invoice
          </Button>
        </div>
      </div>

      {/* AR / AP Tabs */}
      <Tabs value={tab} onValueChange={v => setTab(v as 'ar' | 'ap')}>
        <TabsList className="h-8">
          <TabsTrigger value="ar" className="text-xs flex items-center gap-1.5">
            <TrendingUp className="h-3.5 w-3.5 text-green-600" />
            Accounts Receivable
            <Badge variant="secondary" className="h-4 text-xs px-1.5 ml-1">
              {invoices.filter(i => i.invoice_type === 'ar').length}
            </Badge>
          </TabsTrigger>
          <TabsTrigger value="ap" className="text-xs flex items-center gap-1.5">
            <TrendingDown className="h-3.5 w-3.5 text-red-500" />
            Accounts Payable
            <Badge variant="secondary" className="h-4 text-xs px-1.5 ml-1">
              {invoices.filter(i => i.invoice_type === 'ap').length}
            </Badge>
          </TabsTrigger>
        </TabsList>

        <TabsContent value="ar" className="mt-4">
          <InvoiceSummary invoices={invoices} type="ar" />
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : filtered.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <AlertCircle className="h-8 w-8 mx-auto mb-3 opacity-30" />
              <p className="text-sm">No AR invoices found</p>
            </div>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {filtered.map(inv => (
                <InvoiceCard
                  key={inv.id}
                  invoice={inv}
                  onStatusChange={(id, status) => moveStatus.mutate({ id, status })}
                  onDelete={(id) => deleteInvoice.mutate(id)}
                />
              ))}
            </div>
          )}
        </TabsContent>

        <TabsContent value="ap" className="mt-4">
          <InvoiceSummary invoices={invoices} type="ap" />
          {isLoading ? (
            <p className="text-sm text-muted-foreground">Loading…</p>
          ) : filtered.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <AlertCircle className="h-8 w-8 mx-auto mb-3 opacity-30" />
              <p className="text-sm">No AP invoices found</p>
            </div>
          ) : (
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
              {filtered.map(inv => (
                <InvoiceCard
                  key={inv.id}
                  invoice={inv}
                  onStatusChange={(id, status) => moveStatus.mutate({ id, status })}
                  onDelete={(id) => deleteInvoice.mutate(id)}
                />
              ))}
            </div>
          )}
        </TabsContent>
      </Tabs>

      <CreateInvoiceDialog
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        onCreated={() => queryClient.invalidateQueries({ queryKey: ['invoices'] })}
      />
    </div>
  );
}
