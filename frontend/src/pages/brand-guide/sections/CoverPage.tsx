import { textOn } from '../helpers';
import type { BrandPageProps } from '../types';

export function CoverPage({ org, profile, primary, secondary, accent, effectiveLogo }: BrandPageProps) {
  return (
    <div
      className="cover-page print-page relative flex flex-col min-h-screen overflow-hidden"
      style={{ background: primary }}
    >
      {/* Geometric decoration -- large gold arc */}
      <div
        className="absolute top-0 right-0 rounded-bl-full opacity-10"
        style={{ width: '55vw', height: '55vw', maxWidth: 600, maxHeight: 600, background: accent }}
      />
      <div
        className="absolute bottom-0 left-0 rounded-tr-full opacity-5"
        style={{ width: '40vw', height: '40vw', maxWidth: 440, maxHeight: 440, background: accent }}
      />

      {/* Top rule */}
      <div className="relative px-16 pt-16 flex items-center gap-4">
        <div className="h-px flex-1" style={{ background: accent, opacity: 0.4 }} />
        <p className="text-[9px] uppercase tracking-[0.35em] font-medium" style={{ color: `${accent}99` }}>
          {new Date().toLocaleDateString('en-GB', { year: 'numeric', month: 'long' })}
        </p>
      </div>

      {/* Logo */}
      <div className="relative flex-1 flex flex-col items-center justify-center text-center px-16 gap-8">
        {effectiveLogo ? (
          <img src={effectiveLogo} alt={org.name} className="h-24 object-contain drop-shadow-2xl" />
        ) : (
          <div
            className="heading-font h-24 w-24 rounded-3xl flex items-center justify-center text-3xl font-black shadow-2xl"
            style={{ background: accent, color: textOn(accent) }}
          >
            {org.name.split(' ').slice(0, 2).map(w => w[0]).join('')}
          </div>
        )}

        <div className="space-y-3">
          <h1
            className="heading-font text-5xl md:text-7xl font-black tracking-[0.12em] uppercase"
            style={{ color: secondary, letterSpacing: '0.12em', lineHeight: 1.05 }}
          >
            {org.name}
          </h1>
          {profile.tagline && (
            <p
              className="heading-font text-lg md:text-xl italic font-light tracking-wide"
              style={{ color: accent }}
            >
              &ldquo;{profile.tagline}&rdquo;
            </p>
          )}
        </div>

        {/* Pills */}
        <div className="flex items-center gap-3 flex-wrap justify-center">
          {profile.industry && (
            <span
              className="px-4 py-1.5 rounded-full text-xs font-semibold tracking-widest uppercase border"
              style={{ borderColor: `${accent}50`, color: accent }}
            >
              {profile.industry}
            </span>
          )}
          {profile.brandArchetype && (
            <span
              className="px-4 py-1.5 rounded-full text-xs font-semibold tracking-widest uppercase"
              style={{ background: `${accent}22`, color: accent, border: `1px solid ${accent}50` }}
            >
              The {profile.brandArchetype}
            </span>
          )}
          {profile.marketPosition && (
            <span
              className="px-4 py-1.5 rounded-full text-xs font-semibold tracking-widest uppercase border"
              style={{ borderColor: `${secondary}30`, color: `${secondary}80` }}
            >
              {profile.marketPosition}
            </span>
          )}
        </div>
      </div>

      {/* Bottom rule + address */}
      <div className="relative px-16 pb-16 flex items-center gap-4">
        <p className="text-[9px] uppercase tracking-[0.3em] font-medium" style={{ color: `${accent}70` }}>
          Brand Standards &middot; Confidential
        </p>
        <div className="h-px flex-1" style={{ background: accent, opacity: 0.3 }} />
        {org.address && (
          <p className="text-[9px] tracking-wider" style={{ color: `${secondary}40` }}>{org.address}</p>
        )}
      </div>
    </div>
  );
}
