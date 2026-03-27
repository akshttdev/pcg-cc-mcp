import React from 'react';
import { useParams, useNavigate, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { companiesApi } from '@/lib/api';
import { entityKeys } from '@/lib/query-keys';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import {
  ArrowLeft, Loader2, RefreshCw, BookOpen,
  Building2, Globe, Users, Target, Zap,
  MapPin, AlertCircle, CheckCircle, Star,
  ChevronRight, ChevronLeft, TrendingUp, Briefcase, Brain,
  ExternalLink, Printer, Fingerprint,
} from 'lucide-react';

// ── Nav sections ──────────────────────────────────────────────────────────────

const NAV_SECTIONS = [
  { id: 'exec',        label: 'Executive Summary',    icon: BookOpen },
  { id: 'overview',    label: 'Company Overview',     icon: Building2 },
  { id: 'market',      label: 'Market Analysis',      icon: TrendingUp },
  { id: 'positioning', label: 'Brand Positioning',    icon: Target },
  { id: 'clients',     label: 'Target Clients',       icon: Users },
  { id: 'digital',     label: 'Digital Presence',     icon: Globe },
  { id: 'competitors', label: 'Competitive Landscape',icon: MapPin },
  { id: 'pain',        label: 'Pain Points',          icon: AlertCircle },
  { id: 'opps',        label: 'Opportunities',        icon: Zap },
  { id: 'services',    label: 'Recommended Services', icon: Star },
  { id: 'people',      label: 'Key People',           icon: Fingerprint },
  { id: 'sources',     label: 'Sources',              icon: Globe },
];

// ── Helpers (same as report-helpers) ─────────────────────────────────────────

function parseJson<T>(str: string | undefined | null, fallback: T): T {
  if (!str) return fallback;
  try { return JSON.parse(str) as T; } catch { return fallback; }
}

function fmtDate(dt: string | null | undefined) {
  if (!dt) return '—';
  return new Date(dt).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

function severityColor(s: string) {
  if (s === 'high') return 'text-red-400 bg-red-950/40 border-red-900';
  if (s === 'medium') return 'text-amber-400 bg-amber-950/40 border-amber-900';
  return 'text-blue-400 bg-blue-950/40 border-blue-900';
}

function priorityDot(p: string) {
  if (p === 'high') return 'bg-red-500';
  if (p === 'medium') return 'bg-amber-500';
  return 'bg-blue-500';
}

function threatBadge(t: string) {
  if (t === 'high') return 'border-red-800 text-red-400';
  if (t === 'medium') return 'border-amber-800 text-amber-400';
  return 'border-slate-700 text-slate-400';
}

function IntelCard({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-slate-800 bg-slate-900/60 p-6 ${className}`}>
      {children}
    </div>
  );
}

function SectionHeader({ icon: Icon, title }: { icon: React.ElementType; title: string }) {
  return (
    <div className="flex items-center gap-3 mb-5">
      <div className="w-8 h-8 rounded-lg bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center">
        <Icon className="w-4 h-4 text-indigo-400" />
      </div>
      <h2 className="text-sm font-semibold uppercase tracking-widest text-slate-400">{title}</h2>
    </div>
  );
}

function TextBlock({ content }: { content: string | null | undefined }) {
  if (!content) return <p className="text-slate-500 italic text-sm">No data yet</p>;
  return <p className="text-slate-300 text-sm leading-relaxed">{content}</p>;
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function CompanyIntelPage() {
  const { companyId } = useParams<{ companyId: string }>();
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: company, isLoading } = useQuery({
    queryKey: entityKeys.company(companyId!),
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
  });

  const { data: contactsRaw } = useQuery({
    queryKey: entityKeys.companyContacts(companyId!),
    queryFn: () => companiesApi.listPersons(companyId!),
    enabled: !!companyId,
  });
  const contacts = Array.isArray(contactsRaw) ? contactsRaw : [];

  const { data: intel } = useQuery({
    queryKey: entityKeys.companyIntel(companyId!),
    queryFn: () => companiesApi.getIntelligenceStatus(companyId!),
    enabled: !!companyId,
  });

  const [navOpen, setNavOpen] = React.useState(true);

  const researchMut = useMutation({
    mutationFn: () => companiesApi.research(companyId!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: entityKeys.companyIntel(companyId!) });
      toast.success('Research queued — Scout is gathering intelligence');
    },
    onError: () => toast.error('Failed to queue research'),
  });

  const phase2Mut = useMutation({
    mutationFn: (params: { clientId?: string; dealId?: string; reviewTaskId?: string }) =>
      fetch('/api/business-reports/phase2', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          company_id: companyId,
          client_id: params.clientId ?? null,
          deal_id: params.dealId ?? null,
          review_task_id: params.reviewTaskId ?? null,
        }),
      }).then(r => r.json()),
    onSuccess: () => toast.success('Phase II deep research started — report will appear in Business Reports'),
    onError: () => toast.error('Failed to trigger Phase II research'),
  });

  if (isLoading) return (
    <div className="min-h-screen bg-slate-950 flex items-center justify-center">
      <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
    </div>
  );

  if (!company) return (
    <div className="min-h-screen bg-slate-950 flex items-center justify-center text-slate-500">
      Company not found
    </div>
  );

  // Parse intelligence_raw — supports both rich v2 schema and legacy text blobs
  const raw = parseJson<Record<string, unknown>>(company.intelligence_raw, {});
  const isRunning = intel?.status === 'running' || intel?.status === 'queued';

  // Structured fields (v2 schema from upgraded Scout prompt)
  const executiveSummary  = raw.executive_summary  as string | undefined;
  const companyOverview   = (raw.company_overview  ?? raw.description) as string | undefined;
  const marketAnalysis    = raw.market_analysis    as string | undefined;
  const brandPositioning  = (raw.brand_positioning ?? raw.market_position) as string | undefined;
  const targetClients     = (raw.target_clients    ?? raw.target_audience)   as string | undefined;
  const digitalPresence   = raw.digital_presence   as string | undefined;

  const competitors       = parseJson<Array<{ name: string; website?: string; strengths: string; weaknesses: string; threat_level: string }>>(
    typeof raw.competitors === 'string' ? raw.competitors : JSON.stringify(raw.competitors ?? []), []
  );
  const painPoints        = parseJson<Array<{ point: string; severity: string }>>(
    typeof raw.pain_points === 'string' ? raw.pain_points : JSON.stringify(raw.pain_points ?? []), []
  );
  const opportunities     = parseJson<Array<{ title: string; description: string; priority: string; estimated_value?: string }>>(
    typeof raw.opportunities === 'string' ? raw.opportunities : JSON.stringify(raw.opportunities ?? []), []
  );
  const recommendedSvcs   = parseJson<Array<{ name: string; rationale: string; timeline?: string }>>(
    typeof raw.recommended_services === 'string' ? raw.recommended_services : JSON.stringify(raw.recommended_services ?? []), []
  );
  const sources           = parseJson<Array<{ title: string; url: string; excerpt?: string }>>(
    typeof raw.sources === 'string' ? raw.sources : JSON.stringify(raw.sources ?? []), []
  );

  const founderName    = (raw.founder_name   ?? null) as string | null;
  const foundedYear    = (raw.founded_year   ?? company.founded_year ?? null) as number | null;
  const sizeEstimate   = (raw.size_estimate  ?? null) as string | null;
  const confidence     = company.intelligence_confidence ?? 0;

  const hasIntel = !!(company.intelligence_summary || executiveSummary || companyOverview);

  return (
    <div className="min-h-screen bg-slate-950 text-slate-200">
      <div className="max-w-5xl mx-auto px-4 py-8 space-y-8 print:px-0 print:py-0">

        {/* Header bar */}
        <div className="flex items-start justify-between gap-4 print:hidden">
          <Button variant="ghost" size="sm" className="gap-2 text-slate-400"
            onClick={() => navigate(`/companies/${companyId}`)}>
            <ArrowLeft className="w-4 h-4" /> Company Profile
          </Button>
          <div className="flex items-center gap-2">
            {intel?.status && (
              <Badge variant="outline" className={`text-xs ${
                intel.status === 'done'    ? 'border-emerald-700 text-emerald-400' :
                intel.status === 'running' ? 'border-blue-700 text-blue-400' :
                intel.status === 'queued'  ? 'border-amber-700 text-amber-400' :
                'border-slate-700 text-slate-500'
              }`}>
                {intel.status}
              </Badge>
            )}
            <Button variant="ghost" size="sm" className="gap-2 text-slate-400" onClick={() => window.print()}>
              <Printer className="w-4 h-4" /> Print
            </Button>
            <Button size="sm" onClick={() => researchMut.mutate()} disabled={researchMut.isPending || isRunning}
              className="gap-1.5 bg-indigo-600 hover:bg-indigo-500">
              {researchMut.isPending || isRunning
                ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
                : <RefreshCw className="w-3.5 h-3.5" />}
              {isRunning ? 'Researching…' : 'Refresh Intel'}
            </Button>
            {intel?.status === 'done' && (
              <Button
                size="sm"
                variant="outline"
                className="gap-1.5 border-emerald-700/60 text-emerald-400 hover:bg-emerald-950/40"
                onClick={() => phase2Mut.mutate({})}
                disabled={phase2Mut.isPending}
              >
                {phase2Mut.isPending
                  ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  : <Zap className="w-3.5 h-3.5" />}
                Approve & Deep Research
              </Button>
            )}
            <Link to={`/companies/${companyId}/brand-guide`}>
              <Button size="sm" variant="outline" className="gap-1.5 border-violet-700/60 text-violet-400 hover:bg-violet-950/40">
                <BookOpen className="w-3.5 h-3.5" /> Brand Guide
              </Button>
            </Link>
          </div>
        </div>

        {/* Title block */}
        <div className="print:mt-8">
          <p className="text-xs font-medium uppercase tracking-[0.2em] text-indigo-400 mb-2">
            Company Intel · {fmtDate(company.updated_at)}
          </p>
          <div className="flex items-center gap-4">
            {company.logo_url ? (
              <img src={company.logo_url} alt={company.name}
                className="w-12 h-12 rounded-xl object-contain bg-white/5 border border-slate-800 p-1" />
            ) : (
              <div className="w-12 h-12 rounded-xl bg-slate-800 border border-slate-700 flex items-center justify-center">
                <Building2 className="w-6 h-6 text-slate-500" />
              </div>
            )}
            <div>
              <h1 className="text-3xl font-semibold text-white tracking-tight">{company.name}</h1>
              {(company.industry || company.city) && (
                <p className="text-slate-400 mt-0.5">
                  {[company.industry, company.city].filter(Boolean).join(' · ')}
                </p>
              )}
            </div>
          </div>

          {/* Meta chips */}
          <div className="flex flex-wrap gap-3 mt-4">
            {foundedYear && (
              <span className="text-xs px-3 py-1 rounded-full bg-slate-800 border border-slate-700 text-slate-300">
                Est. {foundedYear}
              </span>
            )}
            {sizeEstimate && (
              <span className="text-xs px-3 py-1 rounded-full bg-slate-800 border border-slate-700 text-slate-300">
                {sizeEstimate} employees
              </span>
            )}
            {founderName && (
              <span className="text-xs px-3 py-1 rounded-full bg-slate-800 border border-slate-700 text-slate-300">
                Founder: {founderName}
              </span>
            )}
            {company.website && (
              <a href={company.website} target="_blank" rel="noopener noreferrer"
                className="text-xs px-3 py-1 rounded-full bg-indigo-950/40 border border-indigo-800/40 text-indigo-400 hover:text-indigo-300 flex items-center gap-1">
                <Globe className="w-3 h-3" />{company.website.replace(/^https?:\/\//, '')}
              </a>
            )}
          </div>

          {confidence > 0 && (
            <div className="mt-4 flex items-center gap-3">
              <div className="flex-1 h-1.5 bg-slate-800 rounded-full overflow-hidden max-w-xs">
                <div className="h-full bg-indigo-500 rounded-full" style={{ width: `${confidence * 100}%` }} />
              </div>
              <span className="text-xs text-slate-500">{Math.round(confidence * 100)}% confidence</span>
            </div>
          )}

          <div className="mt-4 h-px bg-gradient-to-r from-indigo-500/40 to-transparent" />
        </div>

        {/* Empty state */}
        {!hasIntel ? (
          <IntelCard>
            <div className="text-center py-8">
              <Brain className="w-10 h-10 mx-auto mb-3 opacity-20 text-indigo-400" />
              <p className="text-slate-500 text-sm mb-1">No company intelligence gathered yet</p>
              <p className="text-slate-600 text-xs mb-4">Run Scout research to build this company's knowledge profile.</p>
              <Button size="sm" onClick={() => researchMut.mutate()} disabled={researchMut.isPending}
                className="gap-1.5 bg-indigo-600 hover:bg-indigo-500">
                {researchMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Zap className="w-4 h-4" />}
                Run Company Research
              </Button>
            </div>
          </IntelCard>
        ) : (
          <div className="flex gap-6">

            {/* Collapsible sidebar nav */}
            <aside className={`hidden lg:block shrink-0 print:hidden transition-all duration-200 ${navOpen ? 'w-48' : 'w-10'}`}>
              <div className="sticky top-6">
                <div className={`flex items-center mb-3 ${navOpen ? 'justify-between px-2' : 'justify-center'}`}>
                  {navOpen && <p className="text-xs font-semibold uppercase tracking-widest text-slate-600">Contents</p>}
                  <button
                    onClick={() => setNavOpen(v => !v)}
                    className="p-1 rounded text-slate-600 hover:text-slate-400 hover:bg-slate-800/50 transition-colors"
                    title={navOpen ? 'Collapse nav' : 'Expand nav'}
                  >
                    {navOpen ? <ChevronLeft className="w-3.5 h-3.5" /> : <ChevronRight className="w-3.5 h-3.5" />}
                  </button>
                </div>
                <div className="space-y-0.5">
                  {NAV_SECTIONS.map(s => (
                    <a key={s.id} href={`#${s.id}`} title={!navOpen ? s.label : undefined}
                      className={`flex items-center gap-2.5 rounded-lg text-sm text-slate-500 hover:text-slate-300 hover:bg-slate-800/50 transition-colors group ${navOpen ? 'px-2 py-2' : 'px-2.5 py-2 justify-center'}`}>
                      <s.icon className="w-3.5 h-3.5 shrink-0 group-hover:text-indigo-400 transition-colors" />
                      {navOpen && s.label}
                    </a>
                  ))}
                </div>
              </div>
            </aside>

            {/* Main content */}
            <div className="flex-1 min-w-0 space-y-6">

              {/* Executive Summary */}
              <IntelCard className="scroll-mt-6" id="exec">
                <SectionHeader icon={BookOpen} title="Executive Summary" />
                <TextBlock content={executiveSummary ?? company.intelligence_summary} />
              </IntelCard>

              {/* Company Overview */}
              <IntelCard className="scroll-mt-6" id="overview">
                <SectionHeader icon={Building2} title="Company Overview" />
                <TextBlock content={companyOverview} />
              </IntelCard>

              {/* Market Analysis */}
              <IntelCard className="scroll-mt-6" id="market">
                <SectionHeader icon={TrendingUp} title="Market Analysis" />
                <TextBlock content={marketAnalysis} />
              </IntelCard>

              {/* Brand Positioning + Target Clients side by side */}
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <IntelCard className="scroll-mt-6" id="positioning">
                  <SectionHeader icon={Target} title="Brand Positioning" />
                  <TextBlock content={brandPositioning} />
                </IntelCard>
                <IntelCard className="scroll-mt-6" id="clients">
                  <SectionHeader icon={Users} title="Target Clients" />
                  <TextBlock content={targetClients} />
                </IntelCard>
              </div>

              {/* Digital Presence */}
              <IntelCard className="scroll-mt-6" id="digital">
                <SectionHeader icon={Globe} title="Digital Presence" />
                <TextBlock content={digitalPresence} />
              </IntelCard>

              {/* Competitive Landscape */}
              {competitors.length > 0 && (
                <IntelCard className="scroll-mt-6" id="competitors">
                  <SectionHeader icon={MapPin} title="Competitive Landscape" />
                  <div className="space-y-3">
                    {competitors.map((c, i) => (
                      <div key={i} className="flex items-start gap-4 p-4 rounded-xl bg-slate-800/40 border border-slate-700/40">
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2 mb-1">
                            <p className="font-semibold text-slate-200">{c.name}</p>
                            <span className={`text-xs px-2 py-0.5 rounded-full border font-medium ${threatBadge(c.threat_level)}`}>
                              {c.threat_level} threat
                            </span>
                            {c.website && (
                              <a href={c.website} target="_blank" rel="noopener noreferrer" className="text-slate-500 hover:text-slate-400">
                                <Globe className="w-3 h-3" />
                              </a>
                            )}
                          </div>
                          <div className="grid grid-cols-2 gap-3 mt-2">
                            <div>
                              <p className="text-xs uppercase tracking-wider text-emerald-500 mb-1">Strengths</p>
                              <p className="text-xs text-slate-400">{c.strengths}</p>
                            </div>
                            <div>
                              <p className="text-xs uppercase tracking-wider text-red-500 mb-1">Weaknesses</p>
                              <p className="text-xs text-slate-400">{c.weaknesses}</p>
                            </div>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                </IntelCard>
              )}

              {/* Pain Points */}
              {painPoints.length > 0 && (
                <IntelCard className="scroll-mt-6" id="pain">
                  <SectionHeader icon={AlertCircle} title="Pain Points & Challenges" />
                  <div className="space-y-2">
                    {painPoints.map((p, i) => (
                      <div key={i} className={`flex items-start gap-3 px-4 py-3 rounded-lg border text-sm ${severityColor(p.severity)}`}>
                        <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
                        <p>{p.point}</p>
                        <span className="ml-auto text-xs uppercase tracking-wider font-medium opacity-70">{p.severity}</span>
                      </div>
                    ))}
                  </div>
                </IntelCard>
              )}

              {/* Opportunities for PCG */}
              {opportunities.length > 0 && (
                <IntelCard className="scroll-mt-6" id="opps">
                  <SectionHeader icon={Zap} title="Opportunities for PCG" />
                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    {opportunities.map((o, i) => (
                      <div key={i} className="p-4 rounded-xl border border-slate-700/60 bg-slate-800/30">
                        <div className="flex items-center gap-2 mb-2">
                          <div className={`w-2 h-2 rounded-full ${priorityDot(o.priority)}`} />
                          <p className="font-semibold text-slate-200 text-sm">{o.title}</p>
                        </div>
                        <p className="text-xs text-slate-400 leading-relaxed mb-3">{o.description}</p>
                        {o.estimated_value && (
                          <span className="text-xs px-2 py-1 rounded-full border border-emerald-800 text-emerald-400 bg-emerald-950/30 font-medium">
                            {o.estimated_value}
                          </span>
                        )}
                      </div>
                    ))}
                  </div>
                </IntelCard>
              )}

              {/* Recommended Services */}
              {recommendedSvcs.length > 0 && (
                <IntelCard className="scroll-mt-6" id="services">
                  <SectionHeader icon={Star} title="Recommended Services" />
                  <div className="space-y-3">
                    {recommendedSvcs.map((s, i) => (
                      <div key={i} className="flex items-start gap-4">
                        <div className="w-6 h-6 rounded-full bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0 mt-0.5">
                          <span className="text-xs font-semibold text-indigo-400">{i + 1}</span>
                        </div>
                        <div>
                          <p className="font-semibold text-slate-200 text-sm">{s.name}</p>
                          <p className="text-xs text-slate-400 mt-0.5">{s.rationale}</p>
                          {s.timeline && <p className="text-xs text-slate-500 mt-1">Timeline: {s.timeline}</p>}
                        </div>
                      </div>
                    ))}
                  </div>
                </IntelCard>
              )}

              {/* Key People */}
              {contacts.length > 0 && (
                <IntelCard className="scroll-mt-6" id="people">
                  <SectionHeader icon={Fingerprint} title="Key People" />
                  <div className="overflow-x-auto">
                    <table className="w-full text-sm">
                      <thead>
                        <tr className="border-b border-slate-700">
                          <th className="text-left text-xs text-slate-500 uppercase tracking-wider pb-2 pr-4">Name</th>
                          <th className="text-left text-xs text-slate-500 uppercase tracking-wider pb-2 pr-4">Role</th>
                          <th className="text-left text-xs text-slate-500 uppercase tracking-wider pb-2">Intel</th>
                        </tr>
                      </thead>
                      <tbody className="divide-y divide-slate-800">
                        {contacts.map(contact => (
                          <tr key={contact.id}>
                            <td className="py-2.5 pr-4">
                              <Link to={`/people/${contact.id}/intel`}
                                className="text-indigo-400 hover:text-indigo-300 transition-colors font-medium flex items-center gap-1">
                                {contact.full_name}
                                <ExternalLink className="w-3 h-3 opacity-60" />
                              </Link>
                            </td>
                            <td className="py-2.5 pr-4 text-slate-400 capitalize">
                              {contact.job_title ?? contact.person_type}
                            </td>
                            <td className="py-2.5">
                              <Badge variant="outline" className={`text-xs ${
                                contact.intelligence_status === 'done'
                                  ? 'border-emerald-700 text-emerald-400'
                                  : 'border-slate-700 text-slate-500'
                              }`}>
                                {contact.intelligence_status ?? 'idle'}
                              </Badge>
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                </IntelCard>
              )}

              {/* Sources */}
              {sources.length > 0 && (
                <IntelCard className="scroll-mt-6" id="sources">
                  <SectionHeader icon={Globe} title="Sources & References" />
                  <div className="space-y-2">
                    {sources.map((src, i) => (
                      <div key={i} className="flex items-start gap-3 py-2 border-b border-slate-800/60 last:border-0">
                        <span className="text-xs text-slate-600 w-5 pt-0.5 shrink-0">{i + 1}.</span>
                        <div className="min-w-0 flex-1">
                          <a href={src.url} target="_blank" rel="noopener noreferrer"
                            className="text-sm text-indigo-400 hover:text-indigo-300 flex items-center gap-1 group">
                            {src.title || src.url}
                            <ExternalLink className="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                          </a>
                          {src.excerpt && <p className="text-xs text-slate-500 mt-0.5 italic">{src.excerpt}</p>}
                        </div>
                      </div>
                    ))}
                  </div>
                </IntelCard>
              )}

            </div>
          </div>
        )}

        {/* Re-run research CTA at bottom */}
        <div className="flex justify-center pt-4 print:hidden">
          <Button variant="outline" size="sm" onClick={() => researchMut.mutate()}
            disabled={researchMut.isPending || isRunning}
            className="gap-2 text-slate-400 border-slate-700">
            {researchMut.isPending || isRunning
              ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
              : <RefreshCw className="w-3.5 h-3.5" />}
            {isRunning ? 'Scout is researching…' : 'Re-run Scout Research'}
          </Button>
        </div>

      </div>
    </div>
  );
}
