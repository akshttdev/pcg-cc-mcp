import { parseArr, textOn } from '../helpers';
import type { BrandPageProps } from '../types';
import { PageLabel } from './PagePrimitives';

export function AudienceMarketPage({ profile, primary, accent }: BrandPageProps) {
  const competitors = parseArr(profile.competitorBrands);
  const differentiators = parseArr(profile.differentiators);
  const icpIndustries = parseArr(profile.icpIndustries);

  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>05 -- Audience & Market</PageLabel>

      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Audience<br />
          <span style={{ color: accent }}>& Market</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-10 mb-12">
        {/* ICP Card */}
        {profile.icpDescription && (
          <div>
            <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Ideal Customer Profile</p>
            <div
              className="rounded-3xl overflow-hidden shadow-sm border"
              style={{ borderColor: `${accent}30` }}
            >
              <div className="px-6 py-3 flex items-center gap-2" style={{ background: primary }}>
                <div className="h-6 w-6 rounded-full flex items-center justify-center text-xs font-bold" style={{ background: accent, color: textOn(accent) }}>{'\u2726'}</div>
                <span className="text-xs font-semibold tracking-widest uppercase" style={{ color: accent }}>ICP</span>
                {profile.icpCompanySize && (
                  <span className="ml-auto text-[10px] capitalize rounded-full px-2 py-0.5" style={{ background: `${accent}30`, color: accent }}>
                    {profile.icpCompanySize}
                  </span>
                )}
              </div>
              <div className="p-6 space-y-4">
                <p className="text-sm text-gray-700 leading-relaxed">{profile.icpDescription}</p>
                {icpIndustries.length > 0 && (
                  <div className="flex flex-wrap gap-1.5">
                    {icpIndustries.map((ind, i) => (
                      <span key={i} className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-gray-100 text-gray-600">{ind}</span>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        )}

        {/* Target audience */}
        {profile.targetAudience && (
          <div>
            <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Target Audience</p>
            <p className="text-base text-gray-700 leading-relaxed">{profile.targetAudience}</p>
          </div>
        )}
      </div>

      <div className="h-px bg-gray-100 mb-12" />

      {/* Competitive landscape */}
      {(competitors.length > 0 || differentiators.length > 0) && (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-10">
          {competitors.length > 0 && (
            <div>
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Competitive Set</p>
              <div className="space-y-2">
                {competitors.map((c, i) => (
                  <div key={i} className="flex items-center gap-3 p-3 rounded-xl bg-gray-50">
                    <span className="h-2 w-2 rounded-full shrink-0 bg-gray-300" />
                    <span className="text-sm text-gray-700 font-medium">{c}</span>
                  </div>
                ))}
              </div>
            </div>
          )}

          {differentiators.length > 0 && (
            <div>
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Key Differentiators</p>
              <div className="space-y-3">
                {differentiators.map((d, i) => (
                  <div key={i} className="flex items-start gap-3">
                    <div className="h-1.5 w-1.5 rounded-full mt-1.5 shrink-0" style={{ background: accent }} />
                    <p className="text-sm text-gray-700">{d}</p>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
