import { useParams, Link, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { personsApi, intelligenceApi, reportsApi } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { toast } from 'sonner';
import {
  ArrowLeft, Layers, FileText, Loader2, Zap,
  ChevronRight, Brain, Target, Users, Lightbulb,
  AlertTriangle, Network, Briefcase, Clock,
  TrendingUp, Fingerprint, BookOpen, Star,
  User, Printer,
} from 'lucide-react';

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtDate(dt: string | null | undefined) {
  if (!dt) return '—';
  return new Date(dt).toLocaleDateString('en-GB', { day: 'numeric', month: 'short', year: 'numeric' });
}

function parseJson<T>(s: string | undefined | null, fallback: T): T {
  if (!s) return fallback;
  try { return JSON.parse(s) as T; } catch { return fallback; }
}

function depthColor(depth: string | undefined) {
  if (depth === 'deep')     return 'border-emerald-700 text-emerald-400';
  if (depth === 'moderate') return 'border-amber-700 text-amber-400';
  return 'border-slate-700 text-slate-500';
}

function statusBadge(status: string | undefined) {
  const map: Record<string, string> = {
    done:    'border-emerald-700 text-emerald-400',
    running: 'border-blue-700 text-blue-400',
    queued:  'border-amber-700 text-amber-400',
    failed:  'border-red-700 text-red-400',
    idle:    'border-slate-700 text-slate-500',
  };
  return map[status ?? 'idle'] ?? 'border-slate-700 text-slate-500';
}

// ── Shared card components (matching business-reports.tsx) ─────────────────────

function IntelCard({ children, className = '' }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={`rounded-2xl border border-slate-800 bg-slate-900/60 p-6 ${className}`}>
      {children}
    </div>
  );
}

function SectionHeader({ icon: Icon, title, action, accent = 'text-indigo-400' }: {
  icon: React.ElementType;
  title: string;
  action?: React.ReactNode;
  accent?: string;
}) {
  return (
    <div className="flex items-center justify-between mb-5">
      <div className="flex items-center gap-3">
        <div className="w-8 h-8 rounded-lg bg-indigo-500/10 border border-indigo-500/20 flex items-center justify-center">
          <Icon className={`w-4 h-4 ${accent}`} />
        </div>
        <h2 className="text-sm font-semibold uppercase tracking-widest text-slate-400">{title}</h2>
      </div>
      {action}
    </div>
  );
}

// ── Wiki section ──────────────────────────────────────────────────────────────

function WikiSection({
  icon,
  title,
  content,
  accent = 'text-indigo-400',
}: {
  icon: React.ElementType;
  title: string;
  content: string | string[] | null | undefined;
  accent?: string;
}) {
  if (!content) return null;
  const items = Array.isArray(content) ? content : [content];
  const nonEmpty = items.filter(Boolean);
  if (!nonEmpty.length) return null;

  return (
    <IntelCard>
      <SectionHeader icon={icon} title={title} accent={accent} />
      <div className="space-y-3">
        {nonEmpty.map((item, i) => (
          <p key={i} className="text-slate-300 text-sm leading-relaxed">{item}</p>
        ))}
      </div>
    </IntelCard>
  );
}

// ── Pass focus colors ─────────────────────────────────────────────────────────

const focusColors: Record<string, string> = {
  identity:        'text-blue-400',
  market_position: 'text-emerald-400',
  competitors:     'text-amber-400',
  target_clients:  'text-purple-400',
  deep_strategy:   'text-red-400',
};

// ── Main page ─────────────────────────────────────────────────────────────────

export function PersonIntelPage({ personId: propPersonId, embedded = false }: { personId?: string; embedded?: boolean } = {}) {
  const params = useParams<{ personId: string }>();
  const personId = propPersonId ?? params.personId;
  const navigate = useNavigate();
  const queryClient = useQueryClient();

  const { data: person, isLoading } = useQuery({
    queryKey: ['person', personId],
    queryFn: () => personsApi.getById(personId!),
    enabled: !!personId,
  });

  const { data: passes = [] } = useQuery({
    queryKey: ['research-passes', personId],
    queryFn: () => intelligenceApi.listResearchPasses(personId!),
    enabled: !!personId,
  });

  const { data: reports = [] } = useQuery({
    queryKey: ['person-reports', personId],
    queryFn: () => intelligenceApi.listPersonReports(personId!),
    enabled: !!personId,
  });

  const triggerMut = useMutation({
    mutationFn: () => intelligenceApi.triggerNextPass(personId!),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['research-passes', personId] });
      queryClient.invalidateQueries({ queryKey: ['person', personId] });
      toast.success('Intel pass queued');
    },
    onError: () => toast.error('Failed to trigger intel pass'),
  });

  const generateReportMut = useMutation({
    mutationFn: () => reportsApi.generate(personId!, 'business_audit'),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['person-reports', personId] });
      toast.success('Report generation started');
    },
    onError: () => toast.error('Failed to generate report'),
  });

  if (isLoading) {
    return (
      <div className={embedded ? 'flex items-center justify-center py-12' : 'min-h-screen bg-slate-950 flex items-center justify-center'}>
        <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
      </div>
    );
  }

  if (!person) {
    return (
      <div className={embedded ? 'flex items-center justify-center py-12 text-slate-500' : 'min-h-screen bg-slate-950 flex items-center justify-center text-slate-500'}>
        Person not found
      </div>
    );
  }

  const intelRaw = parseJson<Record<string, unknown>>(person.intelligence_raw, {});

  const execSummary    = person.intelligence_summary;
  const background     = intelRaw.background     as string | undefined;
  const positioning    = intelRaw.positioning    as string | undefined;
  const specialization = intelRaw.specialization as string | undefined;
  const expertise      = intelRaw.expertise      as string | string[] | undefined;
  const targetClients  = intelRaw.target_clients as string | string[] | undefined;
  const competitors    = intelRaw.competitors    as string | string[] | undefined;
  const strategy       = intelRaw.strategy       as string | undefined;
  const risks          = intelRaw.risks          as string | string[] | undefined;
  const opportunities  = intelRaw.opportunities  as string | string[] | undefined;
  const network        = intelRaw.network        as string | undefined;
  const communication  = intelRaw.communication_style as string | undefined;

  const hasAnyIntel = execSummary || background || positioning || specialization ||
    expertise || targetClients || competitors || strategy || risks || opportunities;

  const isRunning = person.intelligence_status === 'running' || person.intelligence_status === 'queued';

  return (
    <div className={embedded ? 'text-slate-200' : 'min-h-screen bg-slate-950 text-slate-200'}>
      <div className={embedded ? 'space-y-8' : 'max-w-5xl mx-auto px-4 py-8 space-y-8 print:px-0 print:py-0'}>

        {/* Header bar */}
        <div className="flex items-start justify-between gap-4 print:hidden">
          <div className="flex items-center gap-3">
            {!embedded && (
              <Button
                variant="ghost"
                size="sm"
                className="gap-2 text-slate-400"
                onClick={() => navigate(`/people/${personId}`)}
              >
                <ArrowLeft className="w-4 h-4" /> Profile
              </Button>
            )}
          </div>
          <div className="flex items-center gap-2 flex-wrap">
            <Badge variant="outline" className={`text-xs ${depthColor(person.research_depth)}`}>
              <Layers className="w-3 h-3 mr-1" />
              {person.research_pass_count ?? 0} pass{(person.research_pass_count ?? 0) !== 1 ? 'es' : ''}
            </Badge>
            {person.intelligence_status && (
              <Badge variant="outline" className={`text-xs ${statusBadge(person.intelligence_status)}`}>
                {person.intelligence_status}
              </Badge>
            )}
            <Button
              size="sm"
              onClick={() => triggerMut.mutate()}
              disabled={triggerMut.isPending || isRunning}
              className="gap-1.5 bg-indigo-600 hover:bg-indigo-500"
            >
              {triggerMut.isPending || isRunning
                ? <Loader2 className="w-3.5 h-3.5 animate-spin" />
                : <Zap className="w-3.5 h-3.5" />}
              {isRunning ? 'Running…' : 'Run Intel Pass'}
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="gap-1.5 border-slate-700 text-slate-400"
              onClick={() => generateReportMut.mutate()}
              disabled={generateReportMut.isPending}
            >
              {generateReportMut.isPending ? <Loader2 className="w-3.5 h-3.5 animate-spin" /> : <FileText className="w-3.5 h-3.5" />}
              Report
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="gap-1.5 text-slate-400"
              onClick={() => window.print()}
            >
              <Printer className="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        {/* Title block */}
        <div className="print:mt-8">
          <p className="text-xs font-medium uppercase tracking-[0.2em] text-indigo-400 mb-2">
            Intelligence Profile · {fmtDate(person.updated_at ?? person.created_at)}
          </p>
          <h1 className="text-3xl font-bold text-white tracking-tight">{person.full_name}</h1>
          {person.job_title && (
            <p className="text-slate-400 mt-1">{person.job_title}{person.company_name ? ` · ${person.company_name}` : ''}</p>
          )}
          <div className="mt-4 h-px bg-gradient-to-r from-indigo-500/40 to-transparent" />
        </div>

        {/* No intel state */}
        {!hasAnyIntel ? (
          <IntelCard>
            <div className="text-center py-8">
              <Brain className="w-10 h-10 mx-auto mb-3 opacity-20 text-indigo-400" />
              <p className="text-slate-500 text-sm mb-1">No intelligence gathered yet</p>
              <p className="text-slate-600 text-xs mb-4">Run an intel pass to start building this person's knowledge profile.</p>
              <Button
                size="sm"
                onClick={() => triggerMut.mutate()}
                disabled={triggerMut.isPending || isRunning}
                className="gap-1.5 bg-indigo-600 hover:bg-indigo-500"
              >
                {triggerMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Zap className="w-4 h-4" />}
                Run First Intel Pass
              </Button>
            </div>
          </IntelCard>
        ) : (
          <>
            {/* Executive Summary */}
            {execSummary && (
              <IntelCard>
                <SectionHeader icon={BookOpen} title="Executive Summary" />
                <p className="text-slate-200 text-base leading-relaxed">{execSummary}</p>
              </IntelCard>
            )}

            {/* Background + Positioning row */}
            {(background || positioning) && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {background && (
                  <IntelCard>
                    <SectionHeader icon={Briefcase} title="Background" accent="text-blue-400" />
                    <p className="text-slate-300 text-sm leading-relaxed">{background}</p>
                  </IntelCard>
                )}
                {positioning && (
                  <IntelCard>
                    <SectionHeader icon={Target} title="Market Positioning" accent="text-emerald-400" />
                    <p className="text-slate-300 text-sm leading-relaxed">{positioning}</p>
                  </IntelCard>
                )}
              </div>
            )}

            {/* Expertise */}
            <WikiSection icon={Brain} title="Expertise & Specialization" content={
              [specialization, ...(Array.isArray(expertise) ? expertise : expertise ? [expertise] : [])].filter(Boolean).join('\n\n') || null
            } />

            {/* Target Clients + Network row */}
            {(targetClients || network) && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <WikiSection icon={Users} title="Target Clients" content={targetClients as string | string[]} accent="text-cyan-400" />
                <WikiSection icon={Network} title="Network & Relationships" content={network} accent="text-pink-400" />
              </div>
            )}

            {/* Strategy */}
            <WikiSection icon={TrendingUp} title="Strategy" content={strategy} accent="text-orange-400" />

            {/* Communication Style */}
            <WikiSection icon={Star} title="Communication Style" content={communication} accent="text-teal-400" />

            {/* Opportunities + Risks row */}
            {(opportunities || risks) && (
              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                <WikiSection icon={Lightbulb} title="Opportunities" content={opportunities as string | string[]} accent="text-emerald-400" />
                <WikiSection icon={AlertTriangle} title="Risk Signals" content={risks as string | string[]} accent="text-red-400" />
              </div>
            )}

            {/* Competitors */}
            <WikiSection icon={Target} title="Competitive Landscape" content={competitors as string | string[]} accent="text-amber-400" />
          </>
        )}

        {/* ── Research Passes ─────────────────────────────────────────────── */}
        {passes.length > 0 && (
          <IntelCard>
            <SectionHeader
              icon={Fingerprint}
              title={`Intel Sources — ${passes.length} Research Pass${passes.length !== 1 ? 'es' : ''}`}
              action={
                <Button
                  size="sm"
                  variant="ghost"
                  className="text-xs text-slate-500 gap-1"
                  onClick={() => triggerMut.mutate()}
                  disabled={triggerMut.isPending || isRunning}
                >
                  <Layers className="w-3 h-3" /> Run next
                </Button>
              }
            />
            <div className="space-y-4">
              {passes.map((pass) => {
                const findings = parseJson<string[]>(
                  typeof pass.key_findings === 'string' ? pass.key_findings : JSON.stringify(pass.key_findings),
                  []
                );
                return (
                  <div key={pass.id} className="flex gap-4 p-4 rounded-xl bg-slate-800/50 border border-slate-700/50">
                    <div className="w-8 h-8 rounded-full bg-indigo-500/20 border border-indigo-500/30 flex items-center justify-center shrink-0">
                      <span className="text-xs font-bold text-indigo-300">{pass.pass_number}</span>
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between mb-1">
                        <span className={`text-sm font-medium capitalize ${focusColors[pass.research_focus ?? ''] ?? 'text-slate-300'}`}>
                          {(pass.research_focus ?? '').replace(/_/g, ' ')}
                        </span>
                        <div className="flex items-center gap-2 shrink-0">
                          {pass.confidence_delta != null && pass.confidence_delta !== 0 && (
                            <span className={`text-xs ${pass.confidence_delta > 0 ? 'text-emerald-400' : 'text-red-400'}`}>
                              {pass.confidence_delta > 0 ? '+' : ''}{Math.round(pass.confidence_delta * 100)}%
                            </span>
                          )}
                          <span className="text-xs text-slate-600 flex items-center gap-1">
                            <Clock className="w-3 h-3" />{fmtDate(pass.created_at)}
                          </span>
                        </div>
                      </div>
                      {pass.summary && (
                        <p className="text-sm text-slate-400 leading-relaxed mb-2">{pass.summary}</p>
                      )}
                      {findings.length > 0 && (
                        <div className="space-y-1 border-l border-slate-700 pl-3">
                          {findings.map((f, i) => (
                            <p key={i} className="text-xs text-slate-500 leading-relaxed">
                              <span className="text-indigo-600 mr-1">·</span>{f}
                            </p>
                          ))}
                        </div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </IntelCard>
        )}

        {/* ── Business Reports ─────────────────────────────────────────────── */}
        {reports.length > 0 && (
          <IntelCard>
            <SectionHeader
              icon={FileText}
              title={`Reports (${reports.length})`}
              action={
                <Button
                  size="sm"
                  variant="ghost"
                  className="text-xs text-slate-500 gap-1"
                  onClick={() => generateReportMut.mutate()}
                  disabled={generateReportMut.isPending}
                >
                  {generateReportMut.isPending ? <Loader2 className="w-3 h-3 animate-spin" /> : <Zap className="w-3 h-3" />}
                  Generate
                </Button>
              }
            />
            <div className="space-y-2">
              {reports.map((r) => (
                <Link key={r.id} to={`/business-reports/${r.id}`} className="block group">
                  <div className="flex items-center justify-between gap-3 p-3 rounded-xl bg-slate-800/40 hover:bg-slate-800 border border-slate-700/50 hover:border-indigo-700/40 transition-all">
                    <div className="flex items-center gap-2 min-w-0">
                      <FileText className="w-4 h-4 text-slate-600 shrink-0" />
                      <span className="text-sm text-slate-300 group-hover:text-white transition-colors truncate">{r.title}</span>
                      <Badge variant="outline" className={`text-[10px] shrink-0 ${r.status === 'ready' ? 'border-emerald-700 text-emerald-400' : 'border-amber-700 text-amber-400'}`}>
                        {r.status}
                      </Badge>
                    </div>
                    <div className="flex items-center gap-2 shrink-0">
                      <span className="text-xs text-slate-600">{fmtDate(r.created_at)}</span>
                      <ChevronRight className="w-3.5 h-3.5 text-slate-600 group-hover:text-slate-400" />
                    </div>
                  </div>
                  {r.executive_summary && (
                    <p className="text-xs text-slate-600 mt-0.5 mb-1 line-clamp-1 pl-9">{r.executive_summary}</p>
                  )}
                </Link>
              ))}
            </div>
          </IntelCard>
        )}

        {/* Empty sources note */}
        {passes.length === 0 && reports.length === 0 && hasAnyIntel && (
          <p className="text-center text-xs text-slate-600 py-4">
            Run additional intel passes to deepen the knowledge profile.
          </p>
        )}

        {/* View full profile link */}
        <div className="flex items-center gap-4 pt-4 border-t border-slate-800/50 print:hidden">
          <Link
            to={`/people/${personId}`}
            className="flex items-center gap-1.5 text-sm text-slate-500 hover:text-slate-300 transition-colors"
          >
            <User className="w-4 h-4" /> Full Profile
          </Link>
          {reports.length === 0 && (
            <Button
              size="sm"
              variant="ghost"
              className="gap-1.5 text-slate-500"
              onClick={() => generateReportMut.mutate()}
              disabled={generateReportMut.isPending}
            >
              <FileText className="w-4 h-4" /> Generate Business Report
            </Button>
          )}
        </div>

      </div>
    </div>
  );
}

export default PersonIntelPage;
