import { parseArr } from '../helpers';
import type { BrandPageProps } from '../types';
import { PageLabel } from './PagePrimitives';

export function FoundationPage({ profile, primary, secondary, accent }: BrandPageProps) {
  const values = parseArr(profile.brandValues);

  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>02 -- Foundation</PageLabel>

      {/* Section title */}
      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Brand<br />
          <span style={{ color: accent }}>Foundation</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-10 mb-14">
        {/* Mission */}
        {profile.missionStatement && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <div className="h-5 w-1 rounded-full" style={{ background: accent }} />
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Mission</p>
            </div>
            <p
              className="heading-font text-xl font-semibold text-gray-900 leading-relaxed"
              style={{ lineHeight: 1.6 }}
            >
              {profile.missionStatement}
            </p>
          </div>
        )}

        {/* Vision */}
        {profile.visionStatement && (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <div className="h-5 w-1 rounded-full" style={{ background: primary }} />
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Vision</p>
            </div>
            <p
              className="heading-font text-xl font-semibold text-gray-900 leading-relaxed"
              style={{ lineHeight: 1.6 }}
            >
              {profile.visionStatement}
            </p>
          </div>
        )}
      </div>

      {/* UVP -- large callout */}
      {profile.uniqueValueProposition && (
        <div
          className="rounded-3xl p-10 mb-14"
          style={{ background: primary }}
        >
          <p className="text-[10px] uppercase tracking-[0.3em] mb-4" style={{ color: accent }}>Unique Value Proposition</p>
          <p
            className="heading-font text-xl font-semibold leading-relaxed"
            style={{ color: secondary, lineHeight: 1.7 }}
          >
            &ldquo;{profile.uniqueValueProposition}&rdquo;
          </p>
        </div>
      )}

      {/* Core Values */}
      {values.length > 0 && (
        <div>
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-6">Core Values</p>
          <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
            {values.map((v, i) => (
              <div
                key={i}
                className="rounded-2xl p-5 flex flex-col items-center justify-center text-center gap-2"
                style={{ background: i === 0 ? primary : `${accent}12`, borderTop: `3px solid ${accent}` }}
              >
                <span
                  className="heading-font text-xs font-bold uppercase tracking-wider"
                  style={{ color: i === 0 ? secondary : primary }}
                >
                  {v}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
