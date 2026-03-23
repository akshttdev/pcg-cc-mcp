import { mixWithWhite } from '../helpers';
import type { BrandPageProps } from '../types';
import { ColourCard } from './Mockups';
import { PageLabel } from './PagePrimitives';

export function VisualIdentityPage({ primary, secondary, accent, headingFont, bodyFont }: BrandPageProps) {
  // Extended colour system -- tints of primary and accent
  const primaryTints = [0.85, 0.65, 0.45, 0.25].map(a => mixWithWhite(primary === '#000000' ? '#1A1A2E' : primary, a));
  const accentTints  = [0.85, 0.65, 0.45, 0.25].map(a => mixWithWhite(accent, a));

  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>03 -- Visual Identity</PageLabel>

      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Colour<br />
          <span style={{ color: accent }}>System</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      {/* Primary palette */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5 mb-10">
        <ColourCard color={primary} name="Midnight" role="Primary" />
        <ColourCard color={secondary === '#FFFFFF' || secondary === '#ffffff' ? '#F8F7F4' : secondary} name="Ivory" role="Secondary" />
        <ColourCard color={accent} name="Gold" role="Accent" />
      </div>

      {/* Extended tint palette */}
      <div className="mt-4">
        <p className="text-[9px] uppercase tracking-[0.2em] text-gray-400 mb-3">Extended Palette -- Tints</p>
        <div className="flex gap-2 flex-wrap">
          {[...primaryTints.map((t, i) => ({ color: t, label: `Primary ${(i + 1) * 25}%` })),
            ...accentTints.map((t, i) => ({ color: t, label: `Accent ${(i + 1) * 25}%` }))
          ].map(({ color }, i) => (
            <div key={i} className="flex flex-col items-center gap-1">
              <div className="h-10 w-10 rounded-lg border border-black/8 shadow-sm" style={{ background: color }} />
              <p className="text-[8px] font-mono text-gray-400">{color}</p>
            </div>
          ))}
        </div>
      </div>

      {/* Usage guidance */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-5 mb-14 text-xs">
        <div className="rounded-xl border border-gray-100 p-4 space-y-1">
          <p className="font-semibold text-gray-700">Primary -- Midnight</p>
          <p className="text-gray-500">Backgrounds, hero sections, primary CTAs, print media base.</p>
        </div>
        <div className="rounded-xl border border-gray-100 p-4 space-y-1">
          <p className="font-semibold text-gray-700">Secondary -- Ivory</p>
          <p className="text-gray-500">Body text on dark backgrounds, content pages, whitespace.</p>
        </div>
        <div className="rounded-xl border border-gray-100 p-4 space-y-1">
          <p className="font-semibold text-gray-700">Accent -- Gold</p>
          <p className="text-gray-500">Rules, highlights, borders, CTAs on dark, award elements.</p>
        </div>
      </div>

      {/* Typography section */}
      <p className="text-xs uppercase tracking-[0.25em] font-semibold text-gray-400 mb-8">Typography System</p>

      {/* Heading specimen */}
      <div className="overflow-hidden mb-6">
        <p className="text-xs text-gray-400 mb-2 uppercase tracking-widest">Display / Headings</p>
        <div
          className="heading-font text-[96px] leading-none font-black text-gray-900 select-none overflow-hidden whitespace-nowrap"
          style={{ fontFamily: `'${headingFont}', Cinzel, serif`, letterSpacing: '-0.02em' }}
        >
          Aa
        </div>
        <div className="flex items-baseline gap-6 mt-2">
          <span className="heading-font text-3xl font-bold text-gray-800" style={{ fontFamily: `'${headingFont}', Cinzel, serif` }}>H1 Bold</span>
          <span className="heading-font text-xl font-semibold text-gray-600" style={{ fontFamily: `'${headingFont}', Cinzel, serif` }}>H2 Semibold</span>
          <span className="heading-font text-base font-medium text-gray-500" style={{ fontFamily: `'${headingFont}', Cinzel, serif` }}>H3 Medium</span>
        </div>
        <p className="text-xs text-gray-400 mt-1">{headingFont}</p>
      </div>

      <div className="h-px bg-gray-100 mb-6" />

      {/* Body specimen */}
      <div>
        <p className="text-xs text-gray-400 mb-2 uppercase tracking-widest">Body / Interface</p>
        <p
          className="text-2xl text-gray-700 leading-relaxed"
          style={{ fontFamily: `'${bodyFont}', Inter, sans-serif` }}
        >
          The quick brown fox jumps over the lazy dog. 0123456789
        </p>
        <p className="text-xs text-gray-400 mt-1">{bodyFont}</p>
      </div>
    </div>
  );
}
