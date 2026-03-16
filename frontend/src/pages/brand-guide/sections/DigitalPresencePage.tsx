import { Globe, ExternalLink, MapPin } from 'lucide-react';
import type { OrgKnowledgeSource } from '@/lib/api';
import type { BrandPageProps, SocialEntry, OrgKnowledgeData } from '../types';
import { PageLabel } from './PagePrimitives';

interface DigitalPresencePageProps extends BrandPageProps {
  socials: SocialEntry[];
  knowledge: OrgKnowledgeData | undefined;
}

export function DigitalPresencePage({ org, profile, primary, accent, socials, knowledge }: DigitalPresencePageProps) {
  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>08 -- Digital Presence</PageLabel>

      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Digital<br />
          <span style={{ color: accent }}>Presence</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      {/* HQ address */}
      {org.address && (
        <div
          className="flex items-center gap-4 rounded-2xl px-6 py-4 mb-8 border"
          style={{ borderColor: `${accent}30` }}
        >
          <MapPin className="h-5 w-5 shrink-0" style={{ color: accent }} />
          <div>
            <p className="text-[10px] uppercase tracking-widest text-gray-400 mb-0.5">Headquarters</p>
            <p className="text-sm font-semibold text-gray-800">{org.address}</p>
          </div>
        </div>
      )}

      {/* Website */}
      {profile.websiteUrl && (
        <a
          href={profile.websiteUrl}
          target="_blank"
          rel="noopener noreferrer"
          className="flex items-center gap-4 rounded-2xl px-6 py-4 mb-4 border hover:shadow-sm transition-shadow group"
          style={{ borderColor: `${accent}20` }}
        >
          <Globe className="h-5 w-5 shrink-0 text-gray-400" />
          <div className="flex-1">
            <p className="text-[10px] uppercase tracking-widest text-gray-400 mb-0.5">Website</p>
            <p className="text-sm font-semibold text-gray-800">{profile.websiteUrl}</p>
          </div>
          <ExternalLink className="h-4 w-4 text-gray-300 group-hover:text-gray-500" />
        </a>
      )}

      {/* Social handles */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-14">
        {socials.map((s, i) => {
          const Icon = s.icon;
          return (
            <a
              key={i}
              href={s.url}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-4 rounded-2xl px-5 py-4 border hover:shadow-sm transition-shadow group"
              style={{ borderColor: `${accent}20` }}
            >
              <Icon className="h-4 w-4 shrink-0 text-gray-400" />
              <div className="flex-1">
                <p className="text-[10px] text-gray-400">{s.label}</p>
                <p className="text-xs font-semibold text-gray-800">{s.handle}</p>
              </div>
              <ExternalLink className="h-3.5 w-3.5 text-gray-300 group-hover:text-gray-500" />
            </a>
          );
        })}
      </div>

      {/* Research summary */}
      {profile.researchSummary && (
        <div
          className="rounded-3xl p-8"
          style={{ background: `${accent}10`, border: `1px solid ${accent}30` }}
        >
          <p className="text-[10px] uppercase tracking-[0.3em] mb-3" style={{ color: accent }}>
            Brand Intelligence Report &middot; {profile.researchRanAt ? new Date(profile.researchRanAt).toLocaleDateString('en-GB', { year: 'numeric', month: 'long' }) : 'Recent'}
          </p>
          <p className="text-sm text-gray-700 leading-relaxed">{profile.researchSummary}</p>
        </div>
      )}

      {/* Knowledge graph entities */}
      {knowledge?.knowledge_entries && knowledge.knowledge_entries.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-[0.3em] mb-4 mt-8" style={{ color: accent }}>
            Knowledge Graph &middot; {knowledge.knowledge_entries.length} Intelligence Sources
          </p>
          <div className="grid grid-cols-2 gap-3">
            {knowledge.knowledge_entries.map((src: OrgKnowledgeSource) => (
              <div
                key={src.id}
                className="rounded-2xl p-5"
                style={{ background: `${primary}08`, border: `1px solid ${primary}15` }}
              >
                <div className="flex items-start justify-between gap-2 mb-2">
                  <p className="text-xs font-semibold text-gray-800 leading-tight">{src.source_title}</p>
                  <span
                    className="text-[10px] font-mono shrink-0 px-1.5 py-0.5 rounded-full"
                    style={{ background: `${accent}20`, color: accent }}
                  >
                    {Math.round(src.coverage_score * 100)}%
                  </span>
                </div>
                {src.source_summary && (
                  <p className="text-[11px] text-gray-500 leading-relaxed line-clamp-3">{src.source_summary}</p>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
