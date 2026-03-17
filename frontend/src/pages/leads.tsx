import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { personsApi, type PersonRecord } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import {
  Users, Search, Building2, Mail, TrendingUp, Layers,
  RefreshCw, Loader2, ChevronRight, Radio, Clock,
} from 'lucide-react';


// Sirak Studios org ID (fixed seed)
const SIRAK_ORG = '02020202-0202-0202-0202-020202020202';
const PCG_ORG   = '01010101-0101-0101-0101-010101010101';

const ORG_LABELS: Record<string, { label: string; color: string }> = {
  [SIRAK_ORG]: { label: 'Sirak Studios', color: 'border-violet-700 text-violet-400' },
  [PCG_ORG]:   { label: 'PCG',           color: 'border-blue-700 text-blue-400' },
};

function researchColor(depth: string | undefined) {
  if (depth === 'deep')     return 'text-emerald-400';
  if (depth === 'moderate') return 'text-amber-400';
  return 'text-slate-500';
}

function statusIcon(status: string | undefined) {
  if (status === 'done' || status === 'running')  return <Radio className="w-3 h-3 text-emerald-500 animate-pulse" />;
  if (status === 'queued')  return <Clock className="w-3 h-3 text-amber-500" />;
  if (status === 'failed')  return <Radio className="w-3 h-3 text-red-500" />;
  return <Clock className="w-3 h-3 text-slate-600" />;
}

function initials(name: string) {
  return name.split(' ').slice(0, 2).map(w => w[0]?.toUpperCase() ?? '').join('');
}

function LeadCard({ person }: { person: PersonRecord }) {
  const orgEntry = person.organization_id ? ORG_LABELS[person.organization_id] : null;

  return (
    <Link to={`/people/${person.id}`} className="block group">
      <div className="rounded-xl border border-slate-800 bg-slate-900/50 hover:border-indigo-700/50 hover:bg-slate-900 transition-all p-4">
        <div className="flex items-start gap-4">
          {/* Avatar */}
          <div className="w-10 h-10 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white text-sm font-bold shrink-0">
            {initials(person.full_name)}
          </div>

          {/* Main info */}
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 mb-0.5 flex-wrap">
              <p className="font-semibold text-slate-200 group-hover:text-white transition-colors">
                {person.full_name}
              </p>
              <Badge variant="outline" className={`text-[10px] shrink-0 ${
                person.person_type === 'lead'   ? 'border-amber-700 text-amber-400' :
                person.person_type === 'client' ? 'border-emerald-700 text-emerald-400' :
                'border-slate-700 text-slate-500'
              }`}>
                {person.person_type}
              </Badge>
              {orgEntry && (
                <Badge variant="outline" className={`text-[10px] shrink-0 ${orgEntry.color}`}>
                  {orgEntry.label}
                </Badge>
              )}
            </div>

            <div className="flex items-center gap-2 text-xs text-slate-500 flex-wrap">
              {person.company_name && (
                <span className="flex items-center gap-1">
                  <Building2 className="w-3 h-3" /> {person.company_name}
                </span>
              )}
              {person.job_title && !person.company_name && (
                <span>{person.job_title}</span>
              )}
              {person.email && (
                <span className="flex items-center gap-1">
                  <Mail className="w-3 h-3" /> {person.email}
                </span>
              )}
            </div>

            {/* Research & report status row */}
            <div className="flex items-center gap-3 mt-2 text-xs">
              {/* Research depth */}
              <span className={`flex items-center gap-1 ${researchColor(person.research_depth)}`}>
                <Layers className="w-3 h-3" />
                {person.research_pass_count ?? 0} passes · {person.research_depth ?? 'shallow'}
              </span>

              {/* Intel status */}
              <span className="flex items-center gap-1 text-slate-500">
                {statusIcon(person.intelligence_status)}
                {person.intelligence_status ?? 'idle'}
              </span>

              {/* Onboarding channel */}
              {person.onboarding_channel && (
                <span className="text-slate-600 capitalize">{person.onboarding_channel}</span>
              )}
            </div>
          </div>

          <ChevronRight className="w-4 h-4 text-slate-600 group-hover:text-slate-400 transition-colors shrink-0 mt-1" />
        </div>
      </div>
    </Link>
  );
}

export function LeadsPage() {
  const [search, setSearch] = useState('');
  const [orgFilter, setOrgFilter] = useState<string | undefined>(undefined);
  const [typeFilter, setTypeFilter] = useState<string | undefined>('lead');

  const { data: persons = [], isLoading, refetch, isFetching } = useQuery({
    queryKey: ['persons', 'leads', orgFilter, typeFilter, search],
    queryFn: () => personsApi.list({
      person_type: typeFilter,
      organization_id: orgFilter,
      q: search || undefined,
      limit: 200,
    }),
    refetchInterval: 30000,
  });

  // Separate by org
  const sirakLeads = persons.filter(p =>
    p.organization_id === SIRAK_ORG ||
    (!p.organization_id && orgFilter === SIRAK_ORG)
  );
  const otherLeads = persons.filter(p => p.organization_id !== SIRAK_ORG);

  const showGrouped = !orgFilter && !search;

  return (
    <div className="p-6 max-w-4xl mx-auto space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-xl bg-amber-500/10 border border-amber-500/20 flex items-center justify-center">
            <TrendingUp className="w-5 h-5 text-amber-400" />
          </div>
          <div>
            <h1 className="text-xl font-semibold text-white">Leads</h1>
            <p className="text-xs text-slate-500">{persons.length} contact{persons.length !== 1 ? 's' : ''}</p>
          </div>
        </div>
        <Button variant="ghost" size="sm" onClick={() => refetch()} disabled={isFetching}
          className="gap-2 text-slate-400">
          <RefreshCw className={`w-4 h-4 ${isFetching ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-3 flex-wrap">
        <div className="relative flex-1 min-w-[200px]">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-500" />
          <Input
            placeholder="Search leads..."
            value={search}
            onChange={e => setSearch(e.target.value)}
            className="pl-9 bg-slate-900 border-slate-700 text-slate-200"
          />
        </div>
        <div className="flex gap-2">
          {[
            { value: undefined,  label: 'All Types' },
            { value: 'lead',     label: 'Leads' },
            { value: 'client',   label: 'Clients' },
            { value: 'prospect', label: 'Prospects' },
          ].map(f => (
            <button
              key={f.label}
              onClick={() => setTypeFilter(f.value)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                typeFilter === f.value
                  ? 'bg-indigo-600 text-white'
                  : 'bg-slate-800 text-slate-400 hover:bg-slate-700'
              }`}
            >
              {f.label}
            </button>
          ))}
        </div>
        <div className="flex gap-2">
          {[
            { value: undefined,  label: 'All Orgs' },
            { value: SIRAK_ORG, label: 'Sirak Studios' },
            { value: PCG_ORG,   label: 'PCG' },
          ].map(f => (
            <button
              key={f.label}
              onClick={() => setOrgFilter(f.value)}
              className={`px-3 py-1.5 rounded-lg text-xs font-medium transition-colors ${
                orgFilter === f.value
                  ? 'bg-violet-600 text-white'
                  : 'bg-slate-800 text-slate-400 hover:bg-slate-700'
              }`}
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>

      {/* List */}
      {isLoading ? (
        <div className="flex items-center justify-center py-20">
          <Loader2 className="w-6 h-6 animate-spin text-indigo-400" />
        </div>
      ) : persons.length === 0 ? (
        <div className="text-center py-20 text-slate-500">
          <div className="rounded-full bg-slate-800 p-4 mb-4 inline-flex">
            <Users className="w-8 h-8 text-slate-500" />
          </div>
          <h3 className="text-base font-medium text-slate-300 mb-1">No leads found</h3>
          <p className="text-sm">Process call transcripts via Call Intake to create leads.</p>
          <Link to="/call-intake">
            <Button variant="outline" size="sm" className="mt-4">Go to Call Intake</Button>
          </Link>
        </div>
      ) : showGrouped ? (
        <>
          {sirakLeads.length > 0 && (
            <div>
              <div className="flex items-center gap-2 mb-3">
                <div className="w-2 h-2 rounded-full bg-violet-500" />
                <p className="text-xs font-semibold uppercase tracking-widest text-slate-400">
                  Sirak Studios · {sirakLeads.length}
                </p>
              </div>
              <div className="space-y-2">
                {sirakLeads.map(p => <LeadCard key={p.id} person={p} />)}
              </div>
            </div>
          )}
          {otherLeads.length > 0 && (
            <div>
              <div className="flex items-center gap-2 mb-3 mt-6">
                <div className="w-2 h-2 rounded-full bg-blue-500" />
                <p className="text-xs font-semibold uppercase tracking-widest text-slate-400">
                  Other · {otherLeads.length}
                </p>
              </div>
              <div className="space-y-2">
                {otherLeads.map(p => <LeadCard key={p.id} person={p} />)}
              </div>
            </div>
          )}
        </>
      ) : (
        <div className="space-y-2">
          {persons.map(p => <LeadCard key={p.id} person={p} />)}
        </div>
      )}
    </div>
  );
}
