import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { personsApi, companiesApi, type PersonRecord, type CompanyRecord } from '@/lib/api';
import { Badge } from '@/components/ui/badge';
import {
  Globe, ExternalLink,
  Brain, Fingerprint, ShieldAlert,
} from 'lucide-react';

interface IntelSourcePanelsProps {
  personId?: string;
  companyId?: string;
}

export function IntelSourcePanels({ personId, companyId }: IntelSourcePanelsProps) {
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
                <Badge variant="outline" className="text-xs px-1.5 border-emerald-700 text-emerald-400">verified</Badge>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Link to={`/people/${person.id}?tab=reports`} className="text-xs text-slate-500 hover:text-slate-300 flex items-center gap-0.5">
                <Brain className="w-3 h-3" /> Intel
              </Link>
              <Link to={`/people/${person.id}`} className="text-xs text-slate-500 hover:text-slate-300 flex items-center gap-0.5">
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
              <span className="text-xs text-slate-500">{Math.round((person.intelligence_confidence ?? 0) * 100)}% confidence</span>
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
                <Badge variant="outline" className="text-xs px-1.5 border-emerald-700 text-emerald-400">verified</Badge>
              )}
            </div>
            <div className="flex items-center gap-2">
              <Link to={`/companies/${company.id}`} className="text-xs text-slate-500 hover:text-slate-300 flex items-center gap-0.5">
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
                className="text-xs text-violet-400 hover:text-violet-300 flex items-center gap-0.5 mt-0.5">
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
              <span className="text-xs text-slate-500">{Math.round((company.intelligence_confidence ?? 0) * 100)}% confidence</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
