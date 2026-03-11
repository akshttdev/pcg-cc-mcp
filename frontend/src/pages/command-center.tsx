import { useQuery } from '@tanstack/react-query';
import { useNavigate } from 'react-router-dom';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  LayoutDashboard,
  AlertTriangle,
  Clock,
  MessageSquare,
  Phone,
  Coins,
  ArrowRight,
  CheckCircle2,
  FileText,
} from 'lucide-react';
import { commandCenterApi } from '@/lib/api';

function fmtVibe(v: number) {
  if (v >= 1_000_000) return `${(v / 1_000_000).toFixed(1)}M ꝩ`;
  if (v >= 1_000) return `${(v / 1_000).toFixed(0)}k ꝩ`;
  return `${v} ꝩ`;
}

function formatDate(s: string) {
  if (!s) return '';
  const d = new Date(s);
  if (isNaN(d.getTime())) return s;
  return d.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
}

// ── Section card ──────────────────────────────────────────────────────────────

function Section({
  title,
  icon: Icon,
  count,
  accent,
  children,
}: {
  title: string;
  icon: React.ElementType;
  count: number;
  accent: string;
  children: React.ReactNode;
}) {
  return (
    <div className="border rounded-xl overflow-hidden">
      <div className={`flex items-center gap-3 px-4 py-3 ${accent} border-b`}>
        <Icon className="h-4 w-4" />
        <span className="font-medium text-sm">{title}</span>
        <Badge className="ml-auto text-xs">{count}</Badge>
      </div>
      <div className="divide-y">
        {count === 0 ? (
          <div className="flex items-center justify-center py-6 text-muted-foreground text-sm">
            <CheckCircle2 className="h-4 w-4 mr-2 text-green-500" />
            All clear
          </div>
        ) : (
          children
        )}
      </div>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function CommandCenterPage() {
  const navigate = useNavigate();

  const { data, isLoading } = useQuery({
    queryKey: ['command-center'],
    queryFn: () => commandCenterApi.get(),
    refetchInterval: 60_000, // refresh every minute
  });

  if (isLoading) {
    return (
      <div className="p-6 space-y-4 max-w-5xl mx-auto">
        <Skeleton className="h-10 w-64" />
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {[...Array(6)].map((_, i) => (
            <Skeleton key={i} className="h-40 w-full rounded-xl" />
          ))}
        </div>
      </div>
    );
  }

  if (!data) {
    return (
      <div className="p-6 max-w-5xl mx-auto text-center text-muted-foreground">
        <p>Unable to load Command Center data. The backend may not be running or the endpoint is unavailable.</p>
      </div>
    );
  }

  const {
    overdue_tasks,
    deliverables_due_this_week,
    waiting_on_client_projects,
    follow_up_required,
    proposals_awaiting_approval,
    closed_unpaid_projects,
  } = data;

  const totalAlerts =
    overdue_tasks.length +
    deliverables_due_this_week.length +
    waiting_on_client_projects.length +
    follow_up_required.length +
    proposals_awaiting_approval.length +
    closed_unpaid_projects.length;

  return (
    <div className="p-6 space-y-6 max-w-5xl mx-auto">
      {/* Header */}
      <div className="flex items-center gap-3">
        <LayoutDashboard className="h-6 w-6 text-muted-foreground" />
        <div>
          <h1 className="text-2xl font-semibold">Command Center</h1>
          <p className="text-sm text-muted-foreground">
            {totalAlerts === 0
              ? 'Everything is on track'
              : `${totalAlerts} item${totalAlerts !== 1 ? 's' : ''} need attention`}
          </p>
        </div>
        {totalAlerts > 0 && (
          <Badge className="ml-auto bg-red-100 text-red-700 border-0">
            {totalAlerts} alerts
          </Badge>
        )}
      </div>

      {/* Needs Attention */}
      <div>
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
          Needs Attention
        </h2>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">

          {/* Overdue Tasks */}
          <Section
            title="Overdue Tasks"
            icon={AlertTriangle}
            count={overdue_tasks.length}
            accent="bg-red-50 text-red-700"
          >
            {overdue_tasks.slice(0, 5).map((t) => (
              <div key={t.id} className="flex items-center gap-3 px-4 py-2.5 hover:bg-muted/40">
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{t.title}</p>
                  <p className="text-xs text-muted-foreground truncate">{t.project_name}</p>
                </div>
                <span className="text-xs text-red-600 shrink-0">{formatDate(t.due_date)}</span>
              </div>
            ))}
            {overdue_tasks.length > 5 && (
              <div className="px-4 py-2 text-xs text-muted-foreground">
                +{overdue_tasks.length - 5} more
              </div>
            )}
          </Section>

          {/* Deliverables Due */}
          <Section
            title="Deliverables Due This Week"
            icon={Clock}
            count={deliverables_due_this_week.length}
            accent="bg-orange-50 text-orange-700"
          >
            {deliverables_due_this_week.slice(0, 5).map((d) => (
              <div key={d.id} className="flex items-center gap-3 px-4 py-2.5 hover:bg-muted/40">
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{d.title}</p>
                  <p className="text-xs text-muted-foreground truncate">{d.project_name} · {d.status.replace(/_/g, ' ')}</p>
                </div>
                <span className="text-xs text-orange-600 shrink-0">{formatDate(d.due_date)}</span>
              </div>
            ))}
          </Section>

        </div>
      </div>

      {/* Pipeline */}
      <div>
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
          Pipeline
        </h2>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">

          {/* Waiting on Client */}
          <Section
            title="Waiting on Client"
            icon={MessageSquare}
            count={waiting_on_client_projects.length}
            accent="bg-blue-50 text-blue-700"
          >
            {waiting_on_client_projects.slice(0, 5).map((p) => (
              <div key={p.id} className="flex items-center gap-3 px-4 py-2.5 hover:bg-muted/40 cursor-pointer"
                onClick={() => navigate(`/projects/${p.id}`)}>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{p.name}</p>
                  {p.client_name && (
                    <p className="text-xs text-muted-foreground">{p.client_name}</p>
                  )}
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
              </div>
            ))}
          </Section>

          {/* Follow-up Required */}
          <Section
            title="Follow-up Required"
            icon={Phone}
            count={follow_up_required.length}
            accent="bg-purple-50 text-purple-700"
          >
            {follow_up_required.slice(0, 5).map((lead) => (
              <div key={lead.id} className="flex items-center gap-3 px-4 py-2.5 hover:bg-muted/40 cursor-pointer"
                onClick={() => navigate(`/people/${lead.id}`)}>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{lead.full_name}</p>
                  {lead.email && (
                    <p className="text-xs text-muted-foreground truncate">{lead.email}</p>
                  )}
                </div>
                <Badge className="text-xs bg-purple-50 text-purple-700 border-0 shrink-0">
                  {lead.follow_up_attempts} attempts
                </Badge>
              </div>
            ))}
          </Section>

        </div>
      </div>

      {/* Finance */}
      <div>
        <h2 className="text-sm font-semibold text-muted-foreground uppercase tracking-wide mb-3">
          Finance
        </h2>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">

          {/* Proposals Awaiting Approval */}
          <Section
            title="Proposals Awaiting Approval"
            icon={FileText}
            count={proposals_awaiting_approval.length}
            accent="bg-yellow-50 text-yellow-700"
          >
            {proposals_awaiting_approval.slice(0, 5).map((p) => (
              <div key={p.id} className="flex items-center gap-3 px-4 py-2.5 hover:bg-muted/40 cursor-pointer"
                onClick={() => navigate('/proposals')}>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{p.title}</p>
                  {p.lead_name && (
                    <p className="text-xs text-muted-foreground">{p.lead_name}</p>
                  )}
                </div>
                {p.quote_amount_vibe > 0 && (
                  <span className="flex items-center gap-1 text-xs text-yellow-700 shrink-0">
                    <Coins className="h-3 w-3" />
                    {fmtVibe(p.quote_amount_vibe)}
                  </span>
                )}
              </div>
            ))}
          </Section>

          {/* Closed Unpaid Projects */}
          <Section
            title="Complete — Invoice Needed"
            icon={Coins}
            count={closed_unpaid_projects.length}
            accent="bg-green-50 text-green-700"
          >
            {closed_unpaid_projects.slice(0, 5).map((p) => (
              <div key={p.id} className="flex items-center gap-3 px-4 py-2.5 hover:bg-muted/40 cursor-pointer"
                onClick={() => navigate(`/projects/${p.id}`)}>
                <div className="flex-1 min-w-0">
                  <p className="text-sm font-medium truncate">{p.name}</p>
                  {p.client_name && (
                    <p className="text-xs text-muted-foreground">{p.client_name}</p>
                  )}
                </div>
                <ArrowRight className="h-4 w-4 text-muted-foreground shrink-0" />
              </div>
            ))}
          </Section>

        </div>
      </div>
    </div>
  );
}
