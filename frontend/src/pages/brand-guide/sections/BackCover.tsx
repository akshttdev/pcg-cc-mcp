import { textOn } from '../helpers';
import type { BrandPageProps } from '../types';

export function BackCover({ org, profile, primary, secondary, accent, effectiveLogo }: BrandPageProps) {
  return (
    <div
      className="cover-page print-page relative flex flex-col items-center justify-center min-h-screen overflow-hidden"
      style={{ background: primary }}
    >
      {/* Decorations */}
      <div
        className="absolute bottom-0 right-0 rounded-tl-full opacity-10"
        style={{ width: '50vw', height: '50vw', maxWidth: 520, maxHeight: 520, background: accent }}
      />
      <div
        className="absolute top-0 left-0 rounded-br-full opacity-5"
        style={{ width: '30vw', height: '30vw', maxWidth: 320, maxHeight: 320, background: accent }}
      />

      <div className="relative text-center px-16 space-y-8">
        {effectiveLogo ? (
          <img src={effectiveLogo} alt={org.name} className="h-16 object-contain mx-auto drop-shadow-2xl opacity-90" />
        ) : (
          <div
            className="heading-font h-16 w-16 rounded-2xl flex items-center justify-center text-2xl font-black mx-auto shadow-xl"
            style={{ background: accent, color: textOn(accent) }}
          >
            {org.name.split(' ').slice(0, 2).map(w => w[0]).join('')}
          </div>
        )}

        <h2
          className="heading-font text-4xl font-black tracking-[0.15em] uppercase"
          style={{ color: secondary }}
        >
          {org.name}
        </h2>

        {profile.tagline && (
          <p
            className="heading-font text-base italic font-light"
            style={{ color: `${accent}CC` }}
          >
            &ldquo;{profile.tagline}&rdquo;
          </p>
        )}

        <div className="h-px w-24 mx-auto" style={{ background: accent, opacity: 0.4 }} />

        {profile.websiteUrl && (
          <p className="text-sm tracking-widest" style={{ color: `${secondary}50` }}>
            {profile.websiteUrl.replace('https://', '')}
          </p>
        )}
      </div>

      {/* Bottom label */}
      <div className="absolute bottom-10 left-0 right-0 text-center">
        <p className="text-[9px] uppercase tracking-[0.4em]" style={{ color: `${accent}50` }}>
          Brand Standards Guide &middot; {new Date().getFullYear()} &middot; Confidential
        </p>
      </div>
    </div>
  );
}
