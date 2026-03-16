import { parseUrls } from '../helpers';
import type { BrandPageProps } from '../types';
import { PageBreak, PageLabel } from './PagePrimitives';

export function MoodBoardPage({ profile, accent }: BrandPageProps) {
  const moodUrls = parseUrls(profile.moodBoardUrls);

  if (moodUrls.length === 0) return null;

  return (
    <>
      <PageBreak />
      <div className="print-page min-h-screen px-0 py-0 bg-black overflow-hidden">
        <div className="px-16 pt-16 pb-8">
          <PageLabel>04b -- Brand Mood Board</PageLabel>
          <div className="flex items-end gap-5 mb-8">
            <h2 className="heading-font text-6xl font-black text-white leading-none tracking-tight">
              Brand<br />
              <span style={{ color: accent }}>Aesthetic</span>
            </h2>
            <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
          </div>
          <p className="text-sm text-white/50 max-w-lg mb-10">
            AI-generated visual direction establishing the brand's photographic language, mood, and aesthetic territory.
          </p>
        </div>
        <div className="grid grid-cols-1 gap-0">
          {moodUrls.map((url, i) => (
            <div key={i} className="relative overflow-hidden" style={{ maxHeight: 420 }}>
              <img src={url} alt={`Brand mood board ${i + 1}`} className="w-full object-cover" style={{ maxHeight: 420 }} />
              <div className="absolute inset-0 bg-gradient-to-t from-black/60 via-transparent to-transparent" />
              {i === 0 && (
                <div className="absolute bottom-6 left-16">
                  <p className="text-[9px] uppercase tracking-[0.3em] mb-1" style={{ color: accent }}>Visual Language</p>
                  <p className="text-white text-sm font-semibold">Brand Atmosphere & Environment</p>
                </div>
              )}
              {i === 1 && (
                <div className="absolute bottom-6 left-16">
                  <p className="text-[9px] uppercase tracking-[0.3em] mb-1" style={{ color: accent }}>Brand Applications</p>
                  <p className="text-white text-sm font-semibold">Print & Collateral Direction</p>
                </div>
              )}
            </div>
          ))}
        </div>
        {profile.brandPhotographyNotes && (
          <div className="px-16 py-8">
            <p className="text-[9px] uppercase tracking-[0.25em] mb-2" style={{ color: accent }}>Photography Direction</p>
            <p className="text-sm text-white/60 leading-relaxed max-w-2xl">{profile.brandPhotographyNotes}</p>
          </div>
        )}
      </div>
    </>
  );
}
