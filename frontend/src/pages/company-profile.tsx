import { useEffect, useRef, useState, lazy, Suspense } from 'react';
const ProposalCreateModal = lazy(() =>
  import('@/components/dialogs/ProposalCreateModal').then((m) => ({ default: m.ProposalCreateModal }))
);
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeft,
  Building2,
  Globe,
  MapPin,
  Briefcase,
  ExternalLink,
  Sparkles,
  RefreshCw,
  Users,
  FileText,
  Download,
  Plus,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import {
  companiesApi,
  type CompanyRecord,
  type ProposalRecord,
  type PersonRecord,
} from '@/lib/api';

// ── Tab types ─────────────────────────────────────────────────────────────────

type Tab = 'overview' | 'proposals' | 'contacts' | 'intelligence';

const TABS: { key: Tab; label: string }[] = [
  { key: 'overview',      label: 'Overview' },
  { key: 'proposals',     label: 'Proposals' },
  { key: 'contacts',      label: 'Contacts' },
  { key: 'intelligence',  label: 'Intelligence' },
];

// ── Helpers ───────────────────────────────────────────────────────────────────

function IntelBadge({ status }: { status: string }) {
  const cfg: Record<string, { label: string; cls: string }> = {
    idle:    { label: 'No Intel',  cls: 'bg-gray-100 text-gray-500' },
    queued:  { label: 'Queued',    cls: 'bg-yellow-100 text-yellow-700' },
    running: { label: 'Running…',  cls: 'bg-blue-100 text-blue-700 animate-pulse' },
    done:    { label: 'Intel ✓',   cls: 'bg-green-100 text-green-700' },
    failed:  { label: 'Failed',    cls: 'bg-red-100 text-red-700' },
  };
  const { label, cls } = cfg[status] ?? cfg['idle'];
  return <Badge className={`text-xs border-0 px-2 py-0.5 ${cls}`}>{label}</Badge>;
}

function ProposalStatusBadge({ status }: { status: string }) {
  const cls =
    status === 'contract_signed' ? 'bg-green-100 text-green-700' :
    status === 'declined'        ? 'bg-red-100 text-red-700' :
    status === 'verbal'          ? 'bg-orange-100 text-orange-700' :
    status === 'sent'            ? 'bg-purple-100 text-purple-700' :
    'bg-gray-100 text-gray-600';
  return (
    <Badge className={`text-xs border-0 px-1.5 py-0 ${cls}`}>
      {status.replace(/_/g, ' ')}
    </Badge>
  );
}

// ── Tab: Overview ─────────────────────────────────────────────────────────────

function OverviewTab({
  company,
  proposalCount,
  contactCount,
  onNavigate,
}: {
  company: CompanyRecord;
  proposalCount: number;
  contactCount: number;
  onNavigate: (tab: Tab) => void;
}) {
  return (
    <div className="space-y-4">
      {/* Stat pills */}
      <div className="flex flex-wrap gap-3">
        <button
          onClick={() => onNavigate('proposals')}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-muted hover:bg-muted/80 transition-colors text-sm"
        >
          <FileText className="h-4 w-4 text-muted-foreground" />
          <span className="font-medium">{proposalCount}</span>
          <span className="text-muted-foreground">Proposal{proposalCount !== 1 ? 's' : ''}</span>
        </button>
        <button
          onClick={() => onNavigate('contacts')}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-muted hover:bg-muted/80 transition-colors text-sm"
        >
          <Users className="h-4 w-4 text-muted-foreground" />
          <span className="font-medium">{contactCount}</span>
          <span className="text-muted-foreground">Contact{contactCount !== 1 ? 's' : ''}</span>
        </button>
        {company.organization_id && (
          <div className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-indigo-50 text-sm text-indigo-700">
            <Building2 className="h-4 w-4" />
            <span>Platform Organisation</span>
          </div>
        )}
      </div>

      {/* Description */}
      {company.description && (
        <Card>
          <CardContent className="pt-4">
            <p className="text-sm text-muted-foreground">{company.description}</p>
          </CardContent>
        </Card>
      )}

      {/* Details grid */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium">Company Details</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2">
          {company.industry && (
            <div className="flex items-center gap-2 text-sm">
              <Briefcase className="h-4 w-4 text-muted-foreground shrink-0" />
              <span>{company.industry}</span>
            </div>
          )}
          {company.headquarters && (
            <div className="flex items-center gap-2 text-sm">
              <MapPin className="h-4 w-4 text-muted-foreground shrink-0" />
              <span>{company.headquarters}</span>
            </div>
          )}
          {company.website && (
            <div className="flex items-center gap-2 text-sm">
              <Globe className="h-4 w-4 text-muted-foreground shrink-0" />
              <a
                href={company.website}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-500 hover:underline"
              >
                {company.website}
              </a>
            </div>
          )}
          {company.slug && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span className="font-mono text-xs bg-muted px-1.5 py-0.5 rounded">
                {company.slug}
              </span>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

// ── Tab: Proposals ────────────────────────────────────────────────────────────

function ProposalsTab({ proposals }: { proposals: ProposalRecord[] }) {
  if (proposals.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-32 text-muted-foreground gap-2">
        <FileText className="h-8 w-8 opacity-20" />
        <p className="text-sm">No proposals linked to this company</p>
      </div>
    );
  }
  return (
    <div className="space-y-2">
      {proposals.map((p) => (
        <Card key={p.id} className="hover:shadow-sm transition-shadow">
          <CardContent className="pt-3 pb-3">
            <div className="flex items-start justify-between gap-3">
              <div className="flex-1 min-w-0">
                <p className="font-medium text-sm">{p.title}</p>
                {p.description && (
                  <p className="text-xs text-muted-foreground mt-0.5 line-clamp-1">
                    {p.description}
                  </p>
                )}
              </div>
              <ProposalStatusBadge status={p.status} />
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// ── Tab: Contacts ─────────────────────────────────────────────────────────────

function ContactsTab({
  contacts,
  onResearch,
}: {
  contacts: PersonRecord[];
  onResearch: (personId: string) => void;
}) {
  if (contacts.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-32 text-muted-foreground gap-2">
        <Users className="h-8 w-8 opacity-20" />
        <p className="text-sm">No contacts linked to this company</p>
      </div>
    );
  }
  return (
    <div className="grid gap-2">
      {contacts.map((c) => (
        <Card key={c.id} className="hover:shadow-sm transition-shadow">
          <CardContent className="pt-3 pb-3">
            <div className="flex items-center justify-between gap-3">
              <div className="flex-1 min-w-0">
                <p className="font-medium text-sm">{c.full_name}</p>
                <p className="text-xs text-muted-foreground">
                  {c.job_title ?? c.person_type}
                  {c.email ? ` · ${c.email}` : ''}
                </p>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                {c.intelligence_status === 'done' && (
                  <Badge className="text-xs border-0 bg-green-100 text-green-700 px-1.5 py-0">
                    Intel ✓
                  </Badge>
                )}
                <Button
                  size="sm"
                  variant="ghost"
                  className="h-7 text-xs"
                  onClick={() => onResearch(c.id)}
                >
                  <Sparkles className="h-3 w-3 mr-1" />
                  Research
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}

// ── Tab: Intelligence ─────────────────────────────────────────────────────────

function IntelligenceTab({
  intel,
  onRun,
  isPolling,
}: {
  intel: { status: string; summary?: string; confidence: number; agent?: string; last_run_at?: string } | null;
  onRun: () => void;
  isPolling: boolean;
}) {
  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <IntelBadge status={intel?.status ?? 'idle'} />
          {intel?.last_run_at && (
            <span className="text-xs text-muted-foreground">
              Last run: {new Date(intel.last_run_at).toLocaleDateString()}
            </span>
          )}
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={onRun}
          disabled={isPolling || intel?.status === 'running' || intel?.status === 'queued'}
        >
          {isPolling ? (
            <RefreshCw className="h-3.5 w-3.5 mr-1 animate-spin" />
          ) : (
            <Sparkles className="h-3.5 w-3.5 mr-1" />
          )}
          Run Research
        </Button>
      </div>

      {/* Confidence bar */}
      {(intel?.confidence ?? 0) > 0 && (
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Confidence</span>
            <span>{Math.round((intel?.confidence ?? 0) * 100)}%</span>
          </div>
          <div className="h-2 bg-muted rounded-full overflow-hidden">
            <div
              className="h-full bg-green-500 rounded-full transition-all"
              style={{ width: `${(intel?.confidence ?? 0) * 100}%` }}
            />
          </div>
        </div>
      )}

      {/* Summary */}
      {intel?.summary ? (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-1.5">
              <Sparkles className="h-4 w-4 text-yellow-500" />
              AI Summary
              {intel.agent && (
                <span className="text-xs font-normal text-muted-foreground">by {intel.agent}</span>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground leading-relaxed">{intel.summary}</p>
          </CardContent>
        </Card>
      ) : (
        <div className="flex flex-col items-center justify-center h-24 text-muted-foreground gap-2 border rounded-lg">
          <Sparkles className="h-6 w-6 opacity-20" />
          <p className="text-sm">No intelligence gathered yet</p>
        </div>
      )}
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────

export function CompanyProfilePage() {
  const { companyId } = useParams<{ companyId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTab = (searchParams.get('tab') as Tab) ?? 'overview';
  const [isPolling, setIsPolling] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const { data: company, isLoading } = useQuery({
    queryKey: ['company', companyId],
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
  });

  const { data: proposals = [] } = useQuery({
    queryKey: ['company-proposals', companyId],
    queryFn: () => companiesApi.listProposals(companyId!),
    enabled: !!companyId,
  });

  const { data: contacts = [] } = useQuery({
    queryKey: ['company-contacts', companyId],
    queryFn: () => companiesApi.listPersons(companyId!),
    enabled: !!companyId,
  });

  const { data: intel, refetch: refetchIntel } = useQuery({
    queryKey: ['company-intel', companyId],
    queryFn: () => companiesApi.getIntelligenceStatus(companyId!),
    enabled: !!companyId,
  });

  function setTab(tab: Tab) {
    setSearchParams({ tab });
  }

  // Auto-poll when research is running/queued
  useEffect(() => {
    if (intel?.status === 'running' || intel?.status === 'queued') {
      if (!pollRef.current) {
        setIsPolling(true);
        pollRef.current = setInterval(async () => {
          const updated = await refetchIntel();
          if (
            updated.data?.status !== 'running' &&
            updated.data?.status !== 'queued'
          ) {
            clearInterval(pollRef.current!);
            pollRef.current = null;
            setIsPolling(false);
            queryClient.invalidateQueries({ queryKey: ['company', companyId] });
          }
        }, 3000);
      }
    } else {
      if (pollRef.current) {
        clearInterval(pollRef.current);
        pollRef.current = null;
        setIsPolling(false);
      }
    }
    return () => {
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, [intel?.status, companyId, refetchIntel, queryClient]);

  const [showCreateProposal, setShowCreateProposal] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  async function handleRunResearch() {
    if (!companyId) return;
    try {
      await companiesApi.research(companyId);
      toast.success('Research queued');
      refetchIntel();
    } catch {
      toast.error('Failed to queue research');
    }
  }

  async function handleExportAnalysis() {
    if (!companyId || !company) return;
    setIsExporting(true);
    try {
      await companiesApi.exportAnalysis(companyId, company.name);
    } catch {
      toast.error('Export failed');
    } finally {
      setIsExporting(false);
    }
  }

  async function handlePersonResearch(personId: string) {
    try {
      const { intelligenceApi } = await import('@/lib/api');
      await intelligenceApi.triggerResearch(personId);
      toast.success('Contact research queued');
    } catch {
      toast.error('Failed to queue contact research');
    }
  }

  if (isLoading) {
    return (
      <div className="p-6 space-y-4">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-24 w-full" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (!company) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        Company not found.
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="px-6 py-4 border-b shrink-0">
        <Button
          variant="ghost"
          size="sm"
          className="mb-3 -ml-2 text-muted-foreground"
          onClick={() => navigate('/companies')}
        >
          <ArrowLeft className="h-4 w-4 mr-1" />
          Companies
        </Button>
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-center gap-3">
            <div className="h-10 w-10 rounded-lg bg-muted flex items-center justify-center shrink-0">
              <Building2 className="h-5 w-5 text-muted-foreground" />
            </div>
            <div>
              <h1 className="text-xl font-semibold">{company.name}</h1>
              <div className="flex items-center gap-2 mt-0.5">
                {company.industry && (
                  <span className="text-sm text-muted-foreground">{company.industry}</span>
                )}
                {company.headquarters && (
                  <>
                    <span className="text-muted-foreground">·</span>
                    <span className="text-sm text-muted-foreground">{company.headquarters}</span>
                  </>
                )}
                <IntelBadge status={company.intelligence_status} />
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {company.website && (
              <Button size="sm" variant="ghost" asChild>
                <a href={company.website} target="_blank" rel="noopener noreferrer">
                  <Globe className="h-4 w-4 mr-1" />
                  Website
                  <ExternalLink className="h-3 w-3 ml-1" />
                </a>
              </Button>
            )}
            <Button
              size="sm"
              variant="outline"
              onClick={handleExportAnalysis}
              disabled={isExporting}
              title="Download business analysis document"
            >
              {isExporting ? (
                <RefreshCw className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <Download className="h-4 w-4 mr-1" />
              )}
              Export
            </Button>
            <Button
              size="sm"
              onClick={() => setShowCreateProposal(true)}
            >
              <Plus className="h-4 w-4 mr-1" />
              Proposal
            </Button>
            {company.organization_id ? (
              <Button
                size="sm"
                variant="outline"
                onClick={() => navigate(`/organizations/${company.organization_id}`)}
              >
                <Building2 className="h-4 w-4 mr-1" />
                View Org
              </Button>
            ) : (
              <Button
                size="sm"
                variant="outline"
                disabled
                title="Go to Contacts tab to provision an org from a lead contact"
              >
                Provision Org
              </Button>
            )}
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-1 mt-4 border-b -mb-4 -mx-0">
          {TABS.map((t) => (
            <button
              key={t.key}
              onClick={() => setTab(t.key)}
              className={cn(
                'px-4 py-2 text-sm font-medium border-b-2 transition-colors',
                activeTab === t.key
                  ? 'border-primary text-primary'
                  : 'border-transparent text-muted-foreground hover:text-foreground',
              )}
            >
              {t.label}
              {t.key === 'proposals' && proposals.length > 0 && (
                <span className="ml-1.5 text-xs text-muted-foreground">({proposals.length})</span>
              )}
              {t.key === 'contacts' && contacts.length > 0 && (
                <span className="ml-1.5 text-xs text-muted-foreground">({contacts.length})</span>
              )}
            </button>
          ))}
        </div>
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-auto px-6 py-5">
        {activeTab === 'overview' && (
          <OverviewTab
            company={company}
            proposalCount={proposals.length}
            contactCount={contacts.length}
            onNavigate={setTab}
          />
        )}
        {activeTab === 'proposals' && <ProposalsTab proposals={proposals} />}
        {activeTab === 'contacts' && (
          <ContactsTab contacts={contacts} onResearch={handlePersonResearch} />
        )}
        {activeTab === 'intelligence' && (
          <IntelligenceTab
            intel={intel ?? null}
            onRun={handleRunResearch}
            isPolling={isPolling}
          />
        )}
      </div>

      <Suspense fallback={null}>
        <ProposalCreateModal
          open={showCreateProposal}
          onClose={() => setShowCreateProposal(false)}
          defaultCompanyId={companyId}
        />
      </Suspense>
    </div>
  );
}
