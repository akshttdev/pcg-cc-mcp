import { useState, useCallback } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { reportsApi, personsApi, companiesApi, type BusinessReportRecord, type PersonRecord, type CompanyRecord } from '@/lib/api';
import { InlineEdit } from '@/components/ui/inline-edit';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import {
  ClipboardList, ArrowLeft, User, Building2, Globe, ExternalLink,
  TrendingUp, Users, Target, Zap, MapPin, AlertCircle, CheckCircle,
  Star, ChevronRight, Printer, BookOpen, RefreshCw, Loader2,
  Brain, Fingerprint, ShieldAlert, RotateCcw, ThumbsUp,
} from 'lucide-react';

// ── Helpers ───────────────────────────────────────────────────────────────────

function parseJson<T>(str: string | undefined | null, fallback: T): T {
  if (!str) return fallback;
  try { return JSON.parse(str); } catch { return fallback; }
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

// ── Sub-components ─────────────────────────────────────────────────────────────

function SectionHeader({ icon: Icon, title, action }: {
  icon: React.ElementType;
  title: string;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between mb-5">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center">
          <Icon className="w-4 h-4 text-indigo-400" />
        </div>
        <h2 className="text-sm font-semibold uppercase tracking-widest text-slate-400">{title}</h2>
      </div>
      {action}
    </div>
  );
}

function IntelCard({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-slate-800 bg-slate-900/60 p-6 ${className}`}>
      {children}
    </div>
  );
}

// ── Intel Source Panels ───────────────────────────────────────────────────────

function IntelSourcePanels({ personId, companyId }: { personId?: string; companyId?: string }) {
  const [flagged, setFlagged] = useState<Set<string>>(new Set());
  const toggleFlag = (key: string) =>
    setFlagged(prev => { const n = new Set(prev); n.has(key) ? n.delete(key) : n.add(key); return n; });

  const { data: person } = useQuery<PersonRecord>({
    queryKey: ['intel-person', personId],
    queryFn: () => personsApi.get(personId!),
    enabled: !!personId,
    staleTime: 120_000,
  });

  const { data: company } = useQuery<CompanyRecord>({
    queryKey: ['intel-company', companyId],
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
    staleTime: 120_000,
  });

  if (!person && !company) return null;

  const hasPersonIntel = person && person.intelligence_status === 'done' && person.intelligence_summary;
  const hasCompanyIntel = company && company.intelligence_status === 'done' && company.intelligence_summary;

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
      {/* Person intel panel */}
      {person && (
        <div className={`rounded-xl border p-4 space-y-3 transition-all ${
          flagged.has('person') ? 'border-red-800 bg-red-950/20 opacity-60' : 'border-indigo-800/40 bg-indigo-950/20'
        }`}>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Fingerprint className="w-4 h-4 text-indigo-400" />
              <span className="text-xs font-semibold uppercase tracking-widest text-indigo-400">Person Intel</span>
              {person.intelligence_status === 'done' && (
                <Badge variant="outline" className="text-[10px] px-1.5 border-emerald-700 text-emerald-400">verified</Badge>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Link to={`/persons/${person.id}`} className="text-[10px] text-slate-500 hover:text-slate-300 flex items-center gap-0.5">
                <ExternalLink className="w-3 h-3" /> Profile
              </Link>
              <button
                onClick={() => toggleFlag('person')}
                title={flagged.has('person') ? 'Remove flag' : 'Flag as unreliable'}
                className={`p-1 rounded transition-colors ${flagged.has('person') ? 'text-red-400' : 'text-slate-600 hover:text-red-400'}`}
              >
                <ShieldAlert className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <div>
            <p className="text-sm font-medium text-white">{person.full_name}</p>
            {person.job_title && <p className="text-xs text-slate-400">{person.job_title}</p>}
          </div>

          {hasPersonIntel ? (
            <p className="text-xs text-slate-300 leading-relaxed">{person.intelligence_summary}</p>
          ) : (
            <p className="text-xs text-slate-600 italic">No intelligence data yet — run research from the person profile.</p>
          )}

          {person.intelligence_confidence != null && person.intelligence_confidence > 0 && (
            <div className="flex items-center gap-2">
              <div className="flex-1 h-1 bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-indigo-500 rounded-full"
                  style={{ width: `${Math.round((person.intelligence_confidence ?? 0) * 100)}%` }}
                />
              </div>
              <span className="text-[10px] text-slate-500">{Math.round((person.intelligence_confidence ?? 0) * 100)}% confidence</span>
            </div>
          )}
        </div>
      )}

      {/* Company intel panel */}
      {company && (
        <div className={`rounded-xl border p-4 space-y-3 transition-all ${
          flagged.has('company') ? 'border-red-800 bg-red-950/20 opacity-60' : 'border-violet-800/40 bg-violet-950/20'
        }`}>
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Brain className="w-4 h-4 text-violet-400" />
              <span className="text-xs font-semibold uppercase tracking-widest text-violet-400">Brand Intel</span>
              {company.intelligence_status === 'done' && (
                <Badge variant="outline" className="text-[10px] px-1.5 border-emerald-700 text-emerald-400">verified</Badge>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Link to={`/companies/${company.id}`} className="text-[10px] text-slate-500 hover:text-slate-300 flex items-center gap-0.5">
                <ExternalLink className="w-3 h-3" /> Profile
              </Link>
              <button
                onClick={() => toggleFlag('company')}
                title={flagged.has('company') ? 'Remove flag' : 'Flag as unreliable'}
                className={`p-1 rounded transition-colors ${flagged.has('company') ? 'text-red-400' : 'text-slate-600 hover:text-red-400'}`}
              >
                <ShieldAlert className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <div>
            <p className="text-sm font-medium text-white">{company.name}</p>
            {company.industry && <p className="text-xs text-slate-400">{company.industry}</p>}
            {company.website && (
              <a href={company.website} target="_blank" rel="noopener noreferrer"
                className="text-[10px] text-violet-400 hover:text-violet-300 flex items-center gap-0.5 mt-0.5">
                <Globe className="w-3 h-3" />{company.website.replace(/^https?:\/\//, '')}
              </a>
            )}
          </div>

          {hasCompanyIntel ? (
            <p className="text-xs text-slate-300 leading-relaxed">{company.intelligence_summary}</p>
          ) : (
            <p className="text-xs text-slate-600 italic">No brand intelligence yet — run research from the company profile.</p>
          )}

          {company.intelligence_confidence != null && company.intelligence_confidence > 0 && (
            <div className="flex items-center gap-2">
              <div className="flex-1 h-1 bg-slate-800 rounded-full overflow-hidden">
                <div
                  className="h-full bg-violet-500 rounded-full"
                  style={{ width: `${Math.round((company.intelligence_confidence ?? 0) * 100)}%` }}
                />
              </div>
              <span className="text-[10px] text-slate-500">{Math.round((company.intelligence_confidence ?? 0) * 100)}% confidence</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Review Banner ──────────────────────────────────────────────────────────────

function ReviewBanner({
  report,
  onApprove,
  onRequestRevision,
  isLoading,
}: {
  report: BusinessReportRecord;
  onApprove: () => void;
  onRequestRevision: (notes: string) => void;
  isLoading: boolean;
}) {
  const [showRevisionDialog, setShowRevisionDialog] = useState(false);
  const [revisionNotes, setRevisionNotes] = useState('');

  if (report.review_status === 'approved') {
    return (
      <div className="flex items-center gap-2 px-4 py-3 rounded-xl border border-emerald-800 bg-emerald-950/30 text-emerald-400 text-sm">
        <CheckCircle className="w-4 h-4 shrink-0" />
        <span className="font-medium">Approved — Proposal Generated</span>
        {report.reviewed_at && (
          <span className="text-xs text-emerald-600 ml-auto">
            {new Date(report.reviewed_at).toLocaleDateString('en-GB', { day: 'numeric', month: 'short' })}
          </span>
        )}
      </div>
    );
  }

  if (report.review_status === 'rejected') {
    return (
      <div className="px-4 py-3 rounded-xl border border-amber-800 bg-amber-950/30 space-y-1">
        <div className="flex items-center gap-2 text-amber-400 text-sm">
          <RotateCcw className="w-4 h-4 shrink-0" />
          <span className="font-medium">Revision Requested</span>
        </div>
        {report.review_notes && (
          <p className="text-xs text-amber-300/70 ml-6">{report.review_notes}</p>
        )}
      </div>
    );
  }

  // pending_review
  return (
    <div className="rounded-xl border border-amber-700 bg-amber-950/20 p-4 space-y-3">
      <div className="flex items-center gap-2 text-amber-400">
        <AlertCircle className="w-4 h-4 shrink-0" />
        <p className="text-sm font-medium">This analysis requires human review before generating a proposal</p>
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          className="gap-1.5 bg-emerald-700 hover:bg-emerald-600 text-white border-0"
          disabled={isLoading}
          onClick={onApprove}
        >
          <ThumbsUp className="w-3.5 h-3.5" />
          Approve & Generate Proposal
        </Button>
        <Button
          size="sm"
          variant="outline"
          className="gap-1.5 border-amber-700 text-amber-400 hover:bg-amber-950/40"
          disabled={isLoading}
          onClick={() => setShowRevisionDialog(true)}
        >
          <RotateCcw className="w-3.5 h-3.5" />
          Request Revision
        </Button>
      </div>

      {showRevisionDialog && (
        <div className="mt-3 space-y-2">
          <textarea
            value={revisionNotes}
            onChange={(e) => setRevisionNotes(e.target.value)}
            placeholder="Describe what needs to be revised or researched further..."
            className="w-full bg-slate-800 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200 placeholder-slate-500 resize-none focus:outline-none focus:border-amber-600"
            rows={3}
          />
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              className="bg-amber-700 hover:bg-amber-600 text-white border-0"
              disabled={isLoading || !revisionNotes.trim()}
              onClick={() => {
                onRequestRevision(revisionNotes.trim());
                setShowRevisionDialog(false);
              }}
            >
              Send Revision Request
            </Button>
            <Button
              size="sm"
              variant="ghost"
              className="text-slate-400"
              onClick={() => { setShowRevisionDialog(false); setRevisionNotes(''); }}
            >
              Cancel
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Report Detail (intel page) ─────────────────────────────────────────────────

export function ReportDetail() {
  const { id } = useParams<{ id: string }>();
  const queryClient = useQueryClient();

  const { data: report, isLoading } = useQuery({
    queryKey: ['report', id],
    queryFn: () => reportsApi.get(id!),
    enabled: !!id,
    refetchInterval: (q) => {
      const r = q.state.data;
      return r?.status === 'draft' ? 5000 : false;
    },
  });

  const patchMut = useMutation({
    mutationFn: (data: Partial<BusinessReportRecord>) => reportsApi.patch(id!, data),
    onSuccess: (updated) => {
      queryClient.setQueryData(['report', id], updated);
    },
  });

  const approveMut = useMutation({
    mutationFn: () => reportsApi.approve(id!),
    onSuccess: (result) => {
      queryClient.setQueryData(['report', id], result.report);
      queryClient.invalidateQueries({ queryKey: ['business-reports'] });
      toast.success('Report approved and proposal generated');
    },
    onError: () => toast.error('Failed to approve report'),
  });

  const revisionMut = useMutation({
    mutationFn: (notes: string) => reportsApi.requestRevision(id!, notes),
    onSuccess: (updated) => {
      queryClient.setQueryData(['report', id], updated);
      queryClient.invalidateQueries({ queryKey: ['business-reports'] });
      toast.success('Revision requested');
    },
    onError: () => toast.error('Failed to request revision'),
  });

  const save = useCallback((field: string, value: string) => {
    patchMut.mutate({ [field]: value } as Partial<BusinessReportRecord>);
  }, [patchMut]);

  if (isLoading) return (
    <div className="flex items-center justify-center h-64">
      <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
    </div>
  );

  if (!report) return (
    <div className="p-8 text-slate-500">Report not found.</div>
  );

  const painPoints = parseJson<Array<{ point: string; severity: string }>>(report.pain_points, []);
  const opportunities = parseJson<Array<{ title: string; description: string; priority: string; estimated_value: string }>>(report.opportunities, []);
  const recommendedServices = parseJson<Array<{ name: string; rationale: string; timeline: string }>>(report.recommended_services, []);
  const nextSteps = parseJson<Array<{ action: string; owner: string; deadline: string }>>(report.next_steps, []);
  const individuals = parseJson<Array<{ name: string; role: string; company: string; linkedin?: string; summary: string; key_insights: string }>>(report.individual_profiles, []);
  const competitors = parseJson<Array<{ name: string; website?: string; strengths: string; weaknesses: string; threat_level: string }>>(report.competitor_analysis, []);
  const sources = parseJson<Array<{ title: string; url: string; excerpt?: string }>>(report.sources, []);

  return (
    <div className="max-w-5xl mx-auto px-4 py-8 space-y-8 print:px-0 print:py-0">
      {/* Header bar */}
      <div className="flex items-start justify-between gap-4 print:hidden">
        <div className="flex items-center gap-3">
          <Link to="/business-reports">
            <Button variant="ghost" size="sm" className="gap-2 text-slate-400">
              <ArrowLeft className="w-4 h-4" /> Reports
            </Button>
          </Link>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant="outline" className={
            report.status === 'ready' ? 'border-emerald-700 text-emerald-400' :
            report.status === 'draft' ? 'border-amber-700 text-amber-400' :
            'border-slate-700 text-slate-400'
          }>
            {report.status}
          </Badge>
          <Button variant="ghost" size="sm" className="gap-2 text-slate-400" onClick={() => window.print()}>
            <Printer className="w-4 h-4" /> Print
          </Button>
        </div>
      </div>

      {/* Review Banner */}
      {report.review_status && (
        <ReviewBanner
          report={report}
          onApprove={() => approveMut.mutate()}
          onRequestRevision={(notes) => revisionMut.mutate(notes)}
          isLoading={approveMut.isPending || revisionMut.isPending}
        />
      )}

      {/* Title */}
      <div className="print:mt-8">
        <p className="text-xs font-medium uppercase tracking-[0.2em] text-indigo-400 mb-2">
          Business Analytics Report · {fmtDate(report.created_at)}
        </p>
        <InlineEdit
          value={report.title}
          onSave={(v) => save('title', v)}
          className="text-3xl font-bold text-white tracking-tight"
          placeholder="Report Title"
        />
        <div className="mt-3 h-px bg-gradient-to-r from-indigo-500/40 to-transparent" />
      </div>

      {/* ── Intelligence Source Panels ───────────────────────────────────────── */}
      <IntelSourcePanels personId={report.person_id} companyId={report.company_id} />

      {/* Executive Summary */}
      <IntelCard>
        <SectionHeader icon={BookOpen} title="Executive Summary" />
        <InlineEdit
          value={report.executive_summary ?? ''}
          onSave={(v) => save('executive_summary', v)}
          multiline
          placeholder="Click to add executive summary..."
          className="text-slate-200 text-base leading-relaxed"
          inputClassName="min-h-[120px]"
        />
      </IntelCard>

      {/* Individual Profiles */}
      {individuals.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Users} title="Individual Profiles" />
          <div className="space-y-4">
            {individuals.map((ind, i) => (
              <div key={i} className="flex gap-4 p-4 rounded-xl bg-slate-800/50 border border-slate-700/50">
                <div className="w-10 h-10 rounded-full bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0">
                  <User className="w-5 h-5 text-indigo-400" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <p className="font-semibold text-white">{ind.name}</p>
                    {ind.linkedin && (
                      <a href={ind.linkedin} target="_blank" rel="noopener noreferrer"
                        className="text-indigo-400 hover:text-indigo-300">
                        <ExternalLink className="w-3.5 h-3.5" />
                      </a>
                    )}
                  </div>
                  <p className="text-xs text-slate-400 mb-2">
                    {ind.role && <span>{ind.role}</span>}
                    {ind.role && ind.company && <span className="mx-1.5">·</span>}
                    {ind.company && <span>{ind.company}</span>}
                  </p>
                  {ind.summary && <p className="text-sm text-slate-300 mb-1">{ind.summary}</p>}
                  {ind.key_insights && (
                    <p className="text-xs text-indigo-300 bg-indigo-950/40 border border-indigo-900/60 rounded-lg px-3 py-2 mt-2">
                      {ind.key_insights}
                    </p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Company Overview + Market Analysis row */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <IntelCard>
          <SectionHeader icon={Building2} title="Company Overview" />
          <InlineEdit
            value={report.company_overview ?? ''}
            onSave={(v) => save('company_overview', v)}
            multiline
            placeholder="Company overview..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
        <IntelCard>
          <SectionHeader icon={TrendingUp} title="Market Analysis" />
          <InlineEdit
            value={report.market_analysis ?? ''}
            onSave={(v) => save('market_analysis', v)}
            multiline
            placeholder="Market position, size, trends..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
      </div>

      {/* Brand Positioning + Target Clients row */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <IntelCard>
          <SectionHeader icon={Target} title="Brand Positioning" />
          <InlineEdit
            value={report.brand_positioning ?? ''}
            onSave={(v) => save('brand_positioning', v)}
            multiline
            placeholder="How they position themselves..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
        <IntelCard>
          <SectionHeader icon={Users} title="Target Clients" />
          <InlineEdit
            value={report.target_clients ?? ''}
            onSave={(v) => save('target_clients', v)}
            multiline
            placeholder="Their ideal customer profile..."
            className="text-slate-300 text-sm leading-relaxed"
            inputClassName="min-h-[100px]"
          />
        </IntelCard>
      </div>

      {/* Digital Presence */}
      <IntelCard>
        <SectionHeader icon={Globe} title="Digital Presence" />
        <InlineEdit
          value={report.digital_presence ?? ''}
          onSave={(v) => save('digital_presence', v)}
          multiline
          placeholder="Web, social media, SEO, content presence..."
          className="text-slate-300 text-sm leading-relaxed"
          inputClassName="min-h-[80px]"
        />
      </IntelCard>

      {/* Competitive Landscape */}
      {competitors.length > 0 && (
        <IntelCard>
          <SectionHeader icon={MapPin} title="Competitive Landscape" />
          <div className="space-y-3">
            {competitors.map((c, i) => (
              <div key={i} className="flex items-start gap-4 p-4 rounded-xl bg-slate-800/40 border border-slate-700/40">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <p className="font-semibold text-slate-200">{c.name}</p>
                    <span className={`text-[10px] px-2 py-0.5 rounded-full border font-medium ${threatBadge(c.threat_level)}`}>
                      {c.threat_level} threat
                    </span>
                    {c.website && (
                      <a href={c.website} target="_blank" rel="noopener noreferrer"
                        className="text-slate-500 hover:text-slate-400">
                        <Globe className="w-3 h-3" />
                      </a>
                    )}
                  </div>
                  <div className="grid grid-cols-2 gap-3 mt-2">
                    <div>
                      <p className="text-[10px] uppercase tracking-wider text-emerald-500 mb-1">Strengths</p>
                      <p className="text-xs text-slate-400">{c.strengths}</p>
                    </div>
                    <div>
                      <p className="text-[10px] uppercase tracking-wider text-red-500 mb-1">Weaknesses</p>
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
        <IntelCard>
          <SectionHeader icon={AlertCircle} title="Pain Points & Challenges" />
          <div className="space-y-2">
            {painPoints.map((p, i) => (
              <div key={i} className={`flex items-start gap-3 px-4 py-3 rounded-lg border text-sm ${severityColor(p.severity)}`}>
                <AlertCircle className="w-4 h-4 mt-0.5 shrink-0" />
                <p>{p.point}</p>
                <span className="ml-auto text-[10px] uppercase tracking-wider font-medium opacity-70">{p.severity}</span>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Opportunities */}
      {opportunities.length > 0 && (
        <IntelCard>
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
                  <span className="text-[10px] px-2 py-1 rounded-full border border-emerald-800 text-emerald-400 bg-emerald-950/30 font-medium">
                    {o.estimated_value}
                  </span>
                )}
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Recommended Services */}
      {recommendedServices.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Star} title="Recommended Services" />
          <div className="space-y-3">
            {recommendedServices.map((s, i) => (
              <div key={i} className="flex items-start gap-4">
                <div className="w-6 h-6 rounded-full bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0 mt-0.5">
                  <span className="text-[10px] font-bold text-indigo-400">{i + 1}</span>
                </div>
                <div>
                  <p className="font-semibold text-slate-200 text-sm">{s.name}</p>
                  <p className="text-xs text-slate-400 mt-0.5">{s.rationale}</p>
                  {s.timeline && (
                    <p className="text-[10px] text-slate-500 mt-1">Timeline: {s.timeline}</p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Next Steps */}
      {nextSteps.length > 0 && (
        <IntelCard>
          <SectionHeader icon={ChevronRight} title="Next Steps" />
          <div className="space-y-3">
            {nextSteps.map((step, i) => (
              <div key={i} className="flex items-start gap-3 p-3 rounded-lg bg-slate-800/40 border border-slate-700/40">
                <CheckCircle className="w-4 h-4 text-emerald-500 mt-0.5 shrink-0" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm text-slate-200">{step.action}</p>
                  <div className="flex items-center gap-2 mt-1 text-xs text-slate-500">
                    {step.owner && <span>Owner: {step.owner}</span>}
                    {step.owner && step.deadline && <span>·</span>}
                    {step.deadline && <span>By: {step.deadline}</span>}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Sources & References */}
      {sources.length > 0 && (
        <IntelCard>
          <SectionHeader icon={Globe} title="Sources & References" />
          <div className="space-y-2">
            {sources.map((src, i) => (
              <div key={i} className="flex items-start gap-3 py-2 border-b border-slate-800/60 last:border-0">
                <span className="text-xs text-slate-600 w-5 pt-0.5 shrink-0">{i + 1}.</span>
                <div className="min-w-0 flex-1">
                  <a
                    href={src.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-sm text-indigo-400 hover:text-indigo-300 flex items-center gap-1 group"
                  >
                    {src.title || src.url}
                    <ExternalLink className="w-3 h-3 opacity-0 group-hover:opacity-100 transition-opacity" />
                  </a>
                  {src.excerpt && (
                    <p className="text-xs text-slate-500 mt-0.5 italic">{src.excerpt}</p>
                  )}
                </div>
              </div>
            ))}
          </div>
        </IntelCard>
      )}

      {/* Full Report Markdown */}
      {report.full_report_md && (
        <IntelCard className="print:block">
          <SectionHeader icon={BookOpen} title="Full Narrative Report" />
          <div className="prose prose-invert prose-sm max-w-none prose-headings:text-slate-200 prose-p:text-slate-400 prose-li:text-slate-400 prose-strong:text-slate-200">
            <pre className="whitespace-pre-wrap text-sm text-slate-400 font-sans leading-relaxed">
              {report.full_report_md}
            </pre>
          </div>
        </IntelCard>
      )}
    </div>
  );
}

// ── Report List page ───────────────────────────────────────────────────────────

export default function BusinessReportsPage() {
  const { data: reports = [], isLoading, refetch, isFetching } = useQuery({
    queryKey: ['business-reports'],
    queryFn: reportsApi.list,
    refetchInterval: 30000,
  });

  return (
    <div className="p-6 max-w-5xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-9 h-9 rounded-xl bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center">
            <ClipboardList className="w-5 h-5 text-indigo-400" />
          </div>
          <div>
            <h1 className="text-xl font-semibold text-white">Business Analytics</h1>
            <p className="text-xs text-slate-500">{reports.length} report{reports.length !== 1 ? 's' : ''}</p>
          </div>
        </div>
        <Button variant="ghost" size="sm" onClick={() => refetch()} disabled={isFetching}
          className="gap-2 text-slate-400">
          <RefreshCw className={`w-4 h-4 ${isFetching ? 'animate-spin' : ''}`} />
          Refresh
        </Button>
      </div>

      {/* List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
        </div>
      ) : reports.length === 0 ? (
        <div className="text-center py-20 text-slate-500">
          <ClipboardList className="w-10 h-10 mx-auto mb-3 opacity-30" />
          <p>No analytics reports yet. Process call transcripts via Call Intake to generate reports.</p>
        </div>
      ) : (
        <div className="space-y-3">
          {reports.map((r) => (
            <Link key={r.id} to={`/business-reports/${r.id}`} className="block group">
              <div className="rounded-xl border border-slate-800 bg-slate-900/50 hover:border-slate-700 hover:bg-slate-900 transition-all p-5">
                <div className="flex items-start justify-between gap-4">
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <p className="font-semibold text-slate-200 group-hover:text-white transition-colors truncate">
                        {r.title}
                      </p>
                      <Badge variant="outline" className={`shrink-0 text-[10px] ${
                        r.status === 'ready' ? 'border-emerald-700 text-emerald-400' :
                        r.status === 'draft' ? 'border-amber-700 text-amber-400' :
                        'border-slate-700 text-slate-500'
                      }`}>
                        {r.status}
                      </Badge>
                      {r.review_status && r.review_status !== 'pending_review' && (
                        <Badge variant="outline" className={`shrink-0 text-[10px] ${
                          r.review_status === 'approved' ? 'border-emerald-700 text-emerald-400' :
                          r.review_status === 'rejected' ? 'border-amber-700 text-amber-400' :
                          'border-slate-700 text-slate-500'
                        }`}>
                          {r.review_status === 'approved' ? 'approved' : 'revision needed'}
                        </Badge>
                      )}
                      {r.review_status === 'pending_review' && (
                        <Badge variant="outline" className="shrink-0 text-[10px] border-yellow-700 text-yellow-400">
                          awaiting review
                        </Badge>
                      )}
                    </div>
                    {r.executive_summary && (
                      <p className="text-sm text-slate-500 leading-relaxed line-clamp-2">
                        {r.executive_summary}
                      </p>
                    )}
                    <div className="flex items-center gap-3 mt-2 text-xs text-slate-600">
                      <span>{fmtDate(r.created_at)}</span>
                      {(() => {
                        const pp = parseJson<unknown[]>(r.pain_points, []).length;
                        const op = parseJson<unknown[]>(r.opportunities, []).length;
                        const src = parseJson<unknown[]>(r.sources, []).length;
                        return (
                          <>
                            {pp > 0 && <span className="text-red-600">{pp} pain points</span>}
                            {op > 0 && <span className="text-emerald-600">{op} opportunities</span>}
                            {src > 0 && <span className="text-indigo-600">{src} sources</span>}
                          </>
                        );
                      })()}
                    </div>
                  </div>
                  <ChevronRight className="w-4 h-4 text-slate-600 group-hover:text-slate-400 transition-colors shrink-0 mt-1" />
                </div>
              </div>
            </Link>
          ))}
        </div>
      )}
    </div>
  );
}
