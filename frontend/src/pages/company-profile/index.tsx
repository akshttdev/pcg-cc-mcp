import { useEffect, useRef, useState, lazy, Suspense } from 'react';
const ProposalCreateModal = lazy(() =>
  import('@/components/dialogs/ProposalCreateModal').then((m) => ({ default: m.ProposalCreateModal }))
);
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  ArrowLeft,
  BookOpen,
  Brain,
  Building2,
  Download,
  Loader2,
  Plus,
  RefreshCw,
} from 'lucide-react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import { entityKeys, businessKeys } from '@/lib/query-keys';
import { companiesApi } from '@/lib/api';
import { StarRating, IntelBadge } from './components/helpers';
import { OverviewTab } from './tabs/OverviewTab';
import { ProposalsTab } from './tabs/ProposalsTab';
import { ContactsTab } from './tabs/ContactsTab';
import { IntelligenceTab } from './tabs/IntelligenceTab';
import { WikiTab } from './tabs/WikiTab';
import { EditTab } from './tabs/EditTab';

// ── Tab types ─────────────────────────────────────────────────────────────────

export type Tab = 'overview' | 'proposals' | 'contacts' | 'intelligence' | 'wiki' | 'edit';

const TABS: { key: Tab; label: string }[] = [
  { key: 'overview',     label: 'Overview' },
  { key: 'proposals',    label: 'Proposals' },
  { key: 'contacts',     label: 'Contacts' },
  { key: 'intelligence', label: 'Intelligence' },
  { key: 'wiki',         label: 'Wiki' },
  { key: 'edit',         label: 'Edit' },
];

// ── Page ──────────────────────────────────────────────────────────────────────

export function CompanyProfilePage() {
  const { companyId } = useParams<{ companyId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [searchParams, setSearchParams] = useSearchParams();
  const activeTab = (searchParams.get('tab') as Tab) ?? 'overview';
  const [isPolling, setIsPolling] = useState(false);
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const [showCreateProposal, setShowCreateProposal] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  const { data: company, isLoading, refetch: refetchCompany } = useQuery({
    queryKey: entityKeys.company(companyId!),
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
  });

  const { data: proposals = [] } = useQuery({
    queryKey: businessKeys.companyProposals(companyId!),
    queryFn: () => companiesApi.listProposals(companyId!),
    enabled: !!companyId,
  });

  const { data: contacts = [] } = useQuery({
    queryKey: entityKeys.companyContacts(companyId!),
    queryFn: () => companiesApi.listPersons(companyId!),
    enabled: !!companyId,
  });

  const { data: intel, refetch: refetchIntel } = useQuery({
    queryKey: entityKeys.companyIntel(companyId!),
    queryFn: () => companiesApi.getIntelligenceStatus(companyId!),
    enabled: !!companyId,
  });

  const { data: contactMethods = [], refetch: refetchMethods } = useQuery({
    queryKey: entityKeys.companyContactMethods(companyId!),
    queryFn: () => companiesApi.listContactMethods(companyId!),
    enabled: !!companyId,
  });

  async function handleRemoveMethod(methodId: string) {
    if (!companyId) return;
    await companiesApi.removeContactMethod(companyId, methodId);
    refetchMethods();
  }

  function setTab(tab: Tab) { setSearchParams({ tab }); }

  // Auto-poll during research
  useEffect(() => {
    if (intel?.status === 'running' || intel?.status === 'queued') {
      if (!pollRef.current) {
        setIsPolling(true);
        pollRef.current = setInterval(async () => {
          const updated = await refetchIntel();
          if (updated.data?.status !== 'running' && updated.data?.status !== 'queued') {
            clearInterval(pollRef.current!);
            pollRef.current = null;
            setIsPolling(false);
            queryClient.invalidateQueries({ queryKey: entityKeys.company(companyId!) });
          }
        }, 3000);
      }
    } else {
      if (pollRef.current) { clearInterval(pollRef.current); pollRef.current = null; setIsPolling(false); }
    }
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, [intel?.status, companyId, refetchIntel, queryClient]);

  async function handleRunResearch() {
    if (!companyId) return;
    try {
      await companiesApi.research(companyId);
      toast.success('Research queued');
      refetchIntel();
    } catch { toast.error('Failed to queue research'); }
  }

  async function handleExportAnalysis() {
    if (!companyId || !company) return;
    setIsExporting(true);
    try { await companiesApi.exportAnalysis(companyId, company.name); }
    catch { toast.error('Export failed'); }
    finally { setIsExporting(false); }
  }

  async function handlePersonResearch(personId: string) {
    try {
      const { intelligenceApi } = await import('@/lib/api');
      await intelligenceApi.triggerResearch(personId);
      toast.success('Contact research queued');
    } catch { toast.error('Failed to queue contact research'); }
  }

  if (isLoading) {
    return (
      <div className="p-6 space-y-4">
        <Skeleton className="h-48 w-full rounded-xl" />
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (!company) {
    return <div className="flex items-center justify-center h-full text-muted-foreground">Company not found.</div>;
  }

  const totalRevenue = proposals
    .filter(p => p.status === 'contract_signed')
    .reduce((sum, p) => sum + (p.quote_amount_vibe ?? 0) / 100, 0);

  return (
    <div className="flex flex-col h-full">
      {/* -- Cover + Header -- */}
      <div className="shrink-0">
        {/* Cover image — only rendered when a cover URL exists */}
        {company.cover_image_url && (
          <div
            className="h-32 w-full relative"
            style={{
              backgroundImage: `url(${company.cover_image_url})`,
              backgroundSize: 'cover',
              backgroundPosition: 'center',
            }}
          >
            <div className="absolute inset-0 bg-black/20" />
          </div>
        )}

        {/* Header row */}
        <div className="px-6 pt-3 pb-0">
          <div className="flex items-center gap-2 mb-3">
            <button
              className="flex items-center gap-1 px-2 py-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted text-xs transition-colors"
              onClick={() => navigate(-1)}
            >
              <ArrowLeft className="h-3.5 w-3.5" />
            </button>
          </div>

          <div className={`flex items-start gap-4 ${company.cover_image_url ? '-mt-14' : ''}`}>
            {/* Logo */}
            <div className="h-14 w-14 rounded-xl border-2 border-background bg-card shadow-sm flex items-center justify-center overflow-hidden shrink-0">
              {company.logo_url ? (
                <img src={company.logo_url} alt={company.name} className="h-full w-full object-contain p-1" />
              ) : (
                <Building2 className="h-7 w-7 text-muted-foreground" />
              )}
            </div>
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between gap-3">
                <div className="flex items-center gap-2 flex-wrap min-w-0">
                  <h1 className="text-xl font-bold truncate">{company.name}</h1>
                  <IntelBadge status={company.intelligence_status} />
                  {totalRevenue > 0 && (
                    <span className="px-2 py-0.5 rounded-md bg-green-500/90 text-white text-xs font-semibold">
                      ${totalRevenue.toLocaleString()}
                    </span>
                  )}
                </div>
                {/* Action buttons — matches org profile pattern */}
                <div className="flex items-center gap-1.5 shrink-0">
                  <Button
                    variant="outline"
                    size="sm"
                    className="text-xs gap-1.5"
                    onClick={handleRunResearch}
                    disabled={isPolling}
                  >
                    {isPolling
                      ? <><Loader2 className="h-3.5 w-3.5 animate-spin" />Researching…</>
                      : <><RefreshCw className="h-3.5 w-3.5" />Research</>
                    }
                  </Button>
                  <Link to={`/companies/${companyId}/brand-guide`}>
                    <Button variant="outline" size="sm" className="text-xs gap-1.5">
                      <BookOpen className="h-3.5 w-3.5" />
                      Brand Guide
                    </Button>
                  </Link>
                  <Button
                    variant="outline"
                    size="sm"
                    className="text-xs gap-1.5 border-indigo-700/60 text-indigo-400 hover:bg-indigo-950/40"
                    onClick={() => setTab('intelligence')}
                  >
                    <Brain className="h-3.5 w-3.5" />
                    Intel
                  </Button>
                  <Button size="sm" variant="outline" className="text-xs" onClick={handleExportAnalysis} disabled={isExporting}>
                    {isExporting ? <RefreshCw className="h-3 w-3 animate-spin" /> : <Download className="h-3 w-3" />}
                  </Button>
                  <Button size="sm" className="text-xs" onClick={() => setShowCreateProposal(true)}>
                    <Plus className="h-3 w-3 mr-0.5" />
                    Proposal
                  </Button>
                </div>
              </div>
              <div className="flex items-center gap-2 text-sm text-muted-foreground flex-wrap mt-1">
                {company.industry && <span>{company.industry}</span>}
                {company.city && <><span>·</span><span>{company.city}</span></>}
                {company.headquarters && !company.city && <><span>·</span><span>{company.headquarters}</span></>}
                {company.gmb_rating != null && (
                  <><span>·</span><StarRating rating={company.gmb_rating} count={company.gmb_review_count} /></>
                )}
                {company.organization_id && (
                  <button
                    onClick={() => navigate(`/organizations/${company.organization_id}`)}
                    className="text-xs px-2 py-0.5 rounded-full bg-indigo-100/10 text-indigo-400 hover:bg-indigo-100/20 transition-colors font-medium border border-indigo-500/30"
                  >
                    View Org →
                  </button>
                )}
              </div>
            </div>
          </div>

          {/* Tabs */}
          <div className="flex gap-0 mt-4 border-b">
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
      </div>

      {/* -- Tab content -- */}
      <div className="flex-1 overflow-auto px-6 py-5">
        {activeTab === 'overview' && (
          <OverviewTab
            company={company}
            proposals={proposals}
            contacts={contacts}
            contactMethods={contactMethods}
            onNavigate={setTab}
            onRemoveMethod={handleRemoveMethod}
          />
        )}
        {activeTab === 'proposals' && (
          <ProposalsTab proposals={proposals} onNewProposal={() => setShowCreateProposal(true)} />
        )}
        {activeTab === 'contacts' && (
          <ContactsTab contacts={contacts} onResearch={handlePersonResearch} />
        )}
        {activeTab === 'intelligence' && (
          <IntelligenceTab
            company={company}
            intel={intel ?? null}
            onRun={handleRunResearch}
            isPolling={isPolling}
          />
        )}
        {activeTab === 'wiki' && (
          <WikiTab
            company={company}
            onSaved={() => refetchCompany()}
          />
        )}
        {activeTab === 'edit' && (
          <EditTab
            company={company}
            onSaved={() => {
              refetchCompany();
              setTab('overview');
            }}
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
