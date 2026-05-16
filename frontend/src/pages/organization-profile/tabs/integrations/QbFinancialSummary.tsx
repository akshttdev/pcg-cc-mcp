import { useQuery } from '@tanstack/react-query';
import { AlertCircle, DollarSign, FileWarning, TrendingUp } from 'lucide-react';

import { quickbooksApi } from '@/lib/api';

interface QbFinancialSummaryProps {
  orgId: string;
}

const fmtUsd = (v: number) =>
  v.toLocaleString('en-US', {
    style: 'currency',
    currency: 'USD',
    maximumFractionDigits: 0,
  });

interface MetricProps {
  label: string;
  value: string;
  hint?: string;
  icon: React.ElementType;
  accent: string;
}

function Metric({ label, value, hint, icon: Icon, accent }: MetricProps) {
  return (
    <div className="border rounded-lg p-3 space-y-1.5">
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        <Icon className={`h-3.5 w-3.5 ${accent}`} />
        <span>{label}</span>
      </div>
      <p className="text-lg font-semibold tabular-nums">{value}</p>
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
    </div>
  );
}

export function QbFinancialSummary({ orgId }: QbFinancialSummaryProps) {
  const { data, isLoading, error } = useQuery({
    queryKey: ['qb-financial-summary', orgId],
    queryFn: () => quickbooksApi.getFinancialSummary(orgId),
    staleTime: 30_000,
  });

  if (isLoading) {
    return (
      <div className="border rounded-xl p-4 bg-muted/20">
        <p className="text-xs text-muted-foreground">
          Loading financial summary…
        </p>
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="border rounded-xl p-4 bg-destructive/5">
        <p className="text-xs text-destructive">
          Could not load financial summary. The endpoint reads local invoices
          and works without QuickBooks credentials — check that the backend is
          running.
        </p>
      </div>
    );
  }

  return (
    <div className="border rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h4 className="text-sm font-medium">Financial summary</h4>
        <span className="text-[10px] uppercase tracking-wider text-muted-foreground">
          Local invoices · live
        </span>
      </div>
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-2">
        <Metric
          label="AR Outstanding"
          value={fmtUsd(data.ar_outstanding_usd)}
          hint={`${data.open_invoice_count} open`}
          icon={DollarSign}
          accent="text-emerald-600"
        />
        <Metric
          label="AR Overdue"
          value={fmtUsd(data.ar_overdue_usd)}
          hint={`${data.overdue_invoice_count} overdue`}
          icon={AlertCircle}
          accent="text-red-600"
        />
        <Metric
          label="AP Outstanding"
          value={fmtUsd(data.ap_outstanding_usd)}
          hint="Bills due"
          icon={FileWarning}
          accent="text-amber-600"
        />
        <Metric
          label="Revenue (30d)"
          value={fmtUsd(data.revenue_30d_usd)}
          hint={`${fmtUsd(data.revenue_90d_usd)} / 90d`}
          icon={TrendingUp}
          accent="text-blue-600"
        />
      </div>
    </div>
  );
}
