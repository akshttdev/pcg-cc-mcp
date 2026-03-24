import { parseArr, textOn } from '../helpers';
import type { BrandPageProps } from '../types';
import { PageLabel } from './PagePrimitives';

const VOICE_ATTRIBUTES = [
  'Authoritative', 'Ambitious', 'Precise', 'Visionary', 'Elite',
] as const;

export function VoicePersonalityPage({ profile, primary, secondary, accent, headingFont }: BrandPageProps) {
  const pillars = parseArr(profile.contentPillars);

  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>04 -- Brand Personality</PageLabel>

      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Voice &<br />
          <span style={{ color: accent }}>Personality</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      {/* Archetype hero */}
      {profile.brandArchetype && (
        <div
          className="rounded-3xl p-10 mb-10 flex items-center gap-8"
          style={{ background: primary }}
        >
          <div
            className="h-20 w-20 rounded-2xl flex items-center justify-center text-3xl font-black shrink-0"
            style={{ background: accent, color: textOn(accent), fontFamily: `'${headingFont}', serif` }}
          >
            {profile.brandArchetype[0]}
          </div>
          <div>
            <p className="text-xs uppercase tracking-[0.3em] mb-2" style={{ color: `${accent}80` }}>Brand Archetype</p>
            <h3
              className="heading-font text-3xl font-black mb-1"
              style={{ color: secondary }}
            >
              The {profile.brandArchetype}
            </h3>
            <p className="text-sm" style={{ color: `${secondary}70` }}>
              Commands with authority. Sets and enforces standards. Believes in structure, excellence, and high performance.
            </p>
          </div>
        </div>
      )}

      {/* Voice attributes */}
      <div className="mb-10">
        <p className="text-xs uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Voice Attributes</p>
        <div className="grid grid-cols-3 md:grid-cols-5 gap-3">
          {VOICE_ATTRIBUTES.map((attr, i) => (
            <div
              key={i}
              className="rounded-xl p-4 text-center border"
              style={{ borderColor: i === 0 ? accent : `${accent}30`, background: i === 0 ? `${accent}15` : 'white' }}
            >
              <p
                className="heading-font text-xs font-bold uppercase tracking-wider"
                style={{ color: i === 0 ? accent : '#374151' }}
              >
                {attr}
              </p>
            </div>
          ))}
        </div>
      </div>

      {/* Tone of voice + pull quote */}
      {profile.tagline && (
        <div className="relative mb-10">
          <div
            className="absolute left-0 top-0 bottom-0 w-1 rounded-full"
            style={{ background: accent }}
          />
          <div className="pl-8">
            <p className="text-xs uppercase tracking-[0.25em] font-semibold text-gray-400 mb-3">Brand Voice in Action</p>
            <p
              className="heading-font text-3xl font-bold text-gray-900 leading-relaxed italic"
            >
              &ldquo;{profile.tagline}&rdquo;
            </p>
          </div>
        </div>
      )}

      {/* Content pillars */}
      {pillars.length > 0 && (
        <div>
          <p className="text-xs uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Content Pillars</p>
          <div className="flex gap-3 flex-wrap">
            {pillars.map((p, i) => (
              <div
                key={i}
                className="flex items-center gap-2 px-5 py-3 rounded-full border"
                style={{ borderColor: `${accent}40`, background: i % 2 === 0 ? `${accent}0D` : 'white' }}
              >
                <div className="h-1.5 w-1.5 rounded-full" style={{ background: accent }} />
                <span className="text-xs font-semibold text-gray-700">{p}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
