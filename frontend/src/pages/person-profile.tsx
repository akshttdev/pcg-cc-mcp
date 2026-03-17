import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { personsApi, intelligenceApi, reportsApi, type PersonRecord } from '@/lib/api';
import { entityKeys } from '@/lib/query-keys';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  ArrowLeft, User, Building2, Mail, Phone, Globe,
  Layers, TrendingUp, FileText, ChevronRight, Loader2,
  Zap,
} from 'lucide-react';

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtDate(dt: string | null | undefined) {
  if (!dt) return '—';
  return new Date(dt).toLocaleString('en-GB', { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
}

function depthColor(depth: string | undefined) {
  if (depth === 'deep')     return 'text-emerald-400 border-emerald-700';
  if (depth === 'moderate') return 'text-amber-400 border-amber-700';
  return 'text-slate-500 border-slate-700';
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

function TabBtn({ active, onClick, children }: { active: boolean; onClick: () => void; children: React.ReactNode }) {
  return (
    <button
      onClick={onClick}
      className={`px-4 py-2 text-sm font-medium rounded-lg transition-colors ${
        active ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800'
      }`}
    >
      {children}
    </button>
  );
}

// ── Tabs ──────────────────────────────────────────────────────────────────────

function OverviewTab({ person }: { person: PersonRecord }) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      {/* Contact info */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
        <p className="text-xs font-semibold uppercase tracking-widest text-slate-500 mb-4">Contact</p>
        <div className="space-y-3">
          {person.email && (
            <div className="flex items-center gap-3 text-sm">
              <Mail className="w-4 h-4 text-slate-500 shrink-0" />
              <a href={`mailto:${person.email}`} className="text-indigo-400 hover:text-indigo-300">{person.email}</a>
            </div>
          )}
          {person.phone && (
            <div className="flex items-center gap-3 text-sm">
              <Phone className="w-4 h-4 text-slate-500 shrink-0" />
              <span className="text-slate-300">{person.phone}</span>
            </div>
          )}
          {person.website && (
            <div className="flex items-center gap-3 text-sm">
              <Globe className="w-4 h-4 text-slate-500 shrink-0" />
              <a href={person.website} target="_blank" rel="noopener noreferrer" className="text-indigo-400 hover:text-indigo-300 truncate">{person.website}</a>
            </div>
          )}
          {person.company_name && (
            <div className="flex items-center gap-3 text-sm">
              <Building2 className="w-4 h-4 text-slate-500 shrink-0" />
              <span className="text-slate-300">{person.company_name}</span>
            </div>
          )}
          {person.job_title && (
            <div className="flex items-center gap-3 text-sm">
              <User className="w-4 h-4 text-slate-500 shrink-0" />
              <span className="text-slate-300">{person.job_title}</span>
            </div>
          )}
        </div>
      </div>

      {/* Research snapshot */}
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
        <p className="text-xs font-semibold uppercase tracking-widest text-slate-500 mb-4">Research</p>
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-400">Depth</span>
            <Badge variant="outline" className={depthColor(person.research_depth)}>
              {person.research_depth ?? 'shallow'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-400">Passes</span>
            <span className="text-sm font-semibold text-slate-200">{person.research_pass_count ?? 0}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-400">Status</span>
            <Badge variant="outline" className={statusBadge(person.intelligence_status)}>
              {person.intelligence_status ?? 'idle'}
            </Badge>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-400">Channel</span>
            <span className="text-sm text-slate-400 capitalize">{person.onboarding_channel ?? '—'}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-sm text-slate-400">Added</span>
            <span className="text-xs text-slate-500">{fmtDate(person.created_at)}</span>
          </div>
        </div>
      </div>

      {/* Intelligence summary */}
      {person.intelligence_summary && (
        <div className="md:col-span-2 rounded-xl border border-slate-800 bg-slate-900/50 p-5">
          <p className="text-xs font-semibold uppercase tracking-widest text-slate-500 mb-3">Intelligence Summary</p>
          <p className="text-sm text-slate-300 leading-relaxed whitespace-pre-wrap">{person.intelligence_summary}</p>
        </div>
      )}
    </div>
  );
}

function ResearchTab({ personId }: { personId: string }) {
  const queryClient = useQueryClient();
  const { data: passes = [], isLoading } = useQuery({
    queryKey: entityKeys.researchPasses(personId),
    queryFn: () => intelligenceApi.listResearchPasses(personId),
  });

  const triggerMut = useMutation({
    mutationFn: () => intelligenceApi.triggerNextPass(personId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: entityKeys.researchPasses(personId) });
      queryClient.invalidateQueries({ queryKey: entityKeys.person(personId) });
    },
  });

  const focusColors: Record<string, string> = {
    identity:        'text-blue-400',
    market_position: 'text-emerald-400',
    competitors:     'text-amber-400',
    target_clients:  'text-purple-400',
    deep_strategy:   'text-red-400',
  };

  if (isLoading) return <div className="flex justify-center py-12"><Loader2 className="w-5 h-5 animate-spin text-indigo-400" /></div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-slate-400">{passes.length} research pass{passes.length !== 1 ? 'es' : ''} completed</p>
        <Button
          size="sm"
          onClick={() => triggerMut.mutate()}
          disabled={triggerMut.isPending}
          className="gap-2 bg-indigo-600 hover:bg-indigo-500"
        >
          {triggerMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Layers className="w-4 h-4" />}
          Run Next Pass
        </Button>
      </div>

      {passes.length === 0 ? (
        <div className="text-center py-12 text-slate-500">
          <Layers className="w-8 h-8 mx-auto mb-2 opacity-30" />
          <p className="text-sm">No research passes yet. Run the first pass to start building intelligence.</p>
        </div>
      ) : (
        passes.map((pass) => (
          <div key={pass.id} className="rounded-xl border border-slate-800 bg-slate-900/50 p-5">
            <div className="flex items-center justify-between mb-3">
              <div className="flex items-center gap-3">
                <div className="w-7 h-7 rounded-full bg-slate-800 border border-slate-700 flex items-center justify-center">
                  <span className="text-xs font-bold text-slate-300">{pass.pass_number}</span>
                </div>
                <span className={`text-sm font-medium capitalize ${focusColors[pass.research_focus ?? ''] ?? 'text-slate-300'}`}>
                  {(pass.research_focus ?? '').replace(/_/g, ' ')}
                </span>
              </div>
              <Badge variant="outline" className={statusBadge(pass.status)}>
                {pass.status}
              </Badge>
            </div>
            {pass.summary && (
              <p className="text-sm text-slate-400 leading-relaxed">{pass.summary}</p>
            )}
            <p className="text-xs text-slate-600 mt-2">{fmtDate(pass.created_at)}</p>
          </div>
        ))
      )}
    </div>
  );
}

function ReportsTab({ personId }: { personId: string }) {
  const { data: reports = [], isLoading } = useQuery({
    queryKey: entityKeys.personReports(personId),
    queryFn: () => intelligenceApi.listPersonReports(personId),
  });

  const generateMut = useMutation({
    mutationFn: () => reportsApi.generate(personId, 'business_audit'),
  });

  if (isLoading) return <div className="flex justify-center py-12"><Loader2 className="w-5 h-5 animate-spin text-indigo-400" /></div>;

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <p className="text-sm text-slate-400">{reports.length} report{reports.length !== 1 ? 's' : ''}</p>
        <Button
          size="sm"
          onClick={() => generateMut.mutate()}
          disabled={generateMut.isPending}
          className="gap-2 bg-indigo-600 hover:bg-indigo-500"
        >
          {generateMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <Zap className="w-4 h-4" />}
          Generate Report
        </Button>
      </div>

      {reports.length === 0 ? (
        <div className="text-center py-12 text-slate-500">
          <FileText className="w-8 h-8 mx-auto mb-2 opacity-30" />
          <p className="text-sm">No reports yet. Generate a business analytics report for this lead.</p>
        </div>
      ) : (
        reports.map((r) => (
          <Link key={r.id} to={`/business-reports/${r.id}`} className="block group">
            <div className="rounded-xl border border-slate-800 bg-slate-900/50 hover:border-indigo-700/50 hover:bg-slate-900 transition-all p-5">
              <div className="flex items-start justify-between gap-4">
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <p className="font-semibold text-slate-200 group-hover:text-white transition-colors">{r.title}</p>
                    <Badge variant="outline" className={`shrink-0 text-[10px] ${
                      r.status === 'ready' ? 'border-emerald-700 text-emerald-400' : 'border-amber-700 text-amber-400'
                    }`}>{r.status}</Badge>
                  </div>
                  {r.executive_summary && (
                    <p className="text-sm text-slate-500 line-clamp-2">{r.executive_summary}</p>
                  )}
                  <p className="text-xs text-slate-600 mt-2">{fmtDate(r.created_at)}</p>
                </div>
                <ChevronRight className="w-4 h-4 text-slate-600 group-hover:text-slate-400 shrink-0 mt-1" />
              </div>
            </div>
          </Link>
        ))
      )}
    </div>
  );
}

// ── Main page ─────────────────────────────────────────────────────────────────

export function PersonProfilePage() {
  const { personId } = useParams<{ personId: string }>();
  const [tab, setTab] = useState<'overview' | 'research' | 'reports'>('overview');

  const { data: person, isLoading } = useQuery({
    queryKey: entityKeys.person(personId!),
    queryFn: () => personsApi.get(personId!),
    enabled: !!personId,
  });

  const triggerResearchMut = useMutation({
    mutationFn: () => intelligenceApi.triggerResearch(personId!),
  });

  if (isLoading) return (
    <div className="flex items-center justify-center h-64">
      <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
    </div>
  );

  if (!person) return <div className="p-8 text-slate-500">Person not found.</div>;

  const initials = person.full_name.split(' ').slice(0, 2).map((w: string) => w[0]?.toUpperCase() ?? '').join('');

  return (
    <div className="max-w-4xl mx-auto px-4 py-8 space-y-6">
      {/* Back */}
      <Link to="/leads">
        <Button variant="ghost" size="sm" className="gap-2 text-slate-400">
          <ArrowLeft className="w-4 h-4" /> Leads
        </Button>
      </Link>

      {/* Header card */}
      <div className="rounded-2xl border border-slate-800 bg-slate-900/60 p-6">
        <div className="flex items-start gap-5">
          {/* Avatar */}
          <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white text-xl font-bold shrink-0">
            {initials}
          </div>

          <div className="min-w-0 flex-1">
            <h1 className="text-2xl font-bold text-white mb-1">{person.full_name}</h1>
            <div className="flex items-center gap-2 flex-wrap text-sm text-slate-400">
              {person.job_title && <span>{person.job_title}</span>}
              {person.job_title && person.company_name && <span className="text-slate-600">·</span>}
              {person.company_name && (
                <span className="flex items-center gap-1">
                  <Building2 className="w-3.5 h-3.5" /> {person.company_name}
                </span>
              )}
            </div>
            <div className="flex items-center gap-2 mt-3 flex-wrap">
              <Badge variant="outline" className={`text-xs ${
                person.person_type === 'lead'   ? 'border-amber-700 text-amber-400' :
                person.person_type === 'client' ? 'border-emerald-700 text-emerald-400' :
                'border-slate-700 text-slate-400'
              }`}>
                {person.person_type}
              </Badge>
              <Badge variant="outline" className={`text-xs ${depthColor(person.research_depth)}`}>
                <Layers className="w-3 h-3 mr-1" />
                {person.research_pass_count ?? 0} passes · {person.research_depth ?? 'shallow'}
              </Badge>
              <Badge variant="outline" className={`text-xs ${statusBadge(person.intelligence_status)}`}>
                {person.intelligence_status ?? 'idle'}
              </Badge>
            </div>
          </div>

          {/* Action buttons */}
          <div className="flex flex-col gap-2 shrink-0">
            <Button
              size="sm"
              variant="outline"
              className="gap-2 text-slate-400 border-slate-700"
              onClick={() => triggerResearchMut.mutate()}
              disabled={triggerResearchMut.isPending}
            >
              {triggerResearchMut.isPending ? <Loader2 className="w-4 h-4 animate-spin" /> : <TrendingUp className="w-4 h-4" />}
              Research
            </Button>
            <Link to={`/business-reports?person=${personId}`}>
              <Button size="sm" variant="outline" className="gap-2 text-slate-400 border-slate-700 w-full">
                <FileText className="w-4 h-4" /> Reports
              </Button>
            </Link>
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex gap-2">
        <TabBtn active={tab === 'overview'} onClick={() => setTab('overview')}>Overview</TabBtn>
        <TabBtn active={tab === 'research'} onClick={() => setTab('research')}>
          Research {(person.research_pass_count ?? 0) > 0 && `(${person.research_pass_count})`}
        </TabBtn>
        <TabBtn active={tab === 'reports'} onClick={() => setTab('reports')}>Reports</TabBtn>
      </div>

      {/* Tab content */}
      {tab === 'overview' && <OverviewTab person={person} />}
      {tab === 'research' && <ResearchTab personId={personId!} />}
      {tab === 'reports' && <ReportsTab personId={personId!} />}
    </div>
  );
}
