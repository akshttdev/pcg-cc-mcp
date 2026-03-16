import { textOn } from '../helpers';
import type { BrandPageProps } from '../types';
import { PageLabel } from './PagePrimitives';

export function LogoSystemPage({ org, primary, accent, effectiveLogo }: BrandPageProps) {
  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>03b -- Logo System</PageLabel>

      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Logo<br />
          <span style={{ color: accent }}>System</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      {/* Three logo variants */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
        {/* Primary -- on dark */}
        <div className="space-y-3">
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Primary &middot; On Dark</p>
          <div
            className="rounded-2xl flex items-center justify-center p-10 min-h-[160px]"
            style={{ background: primary }}
          >
            {effectiveLogo ? (
              <img src={effectiveLogo} alt={org.name} className="max-h-16 object-contain" />
            ) : (
              <div className="heading-font text-2xl font-black tracking-widest uppercase" style={{ color: accent }}>
                {org.name}
              </div>
            )}
          </div>
          <p className="text-[10px] text-gray-400">Primary version. Use on dark backgrounds, print, and hero sections.</p>
        </div>

        {/* Reversed -- on light */}
        <div className="space-y-3">
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Reversed &middot; On Light</p>
          <div
            className="rounded-2xl flex items-center justify-center p-10 min-h-[160px] border border-gray-100"
            style={{ background: '#F8F7F4' }}
          >
            {effectiveLogo ? (
              <img src={effectiveLogo} alt={org.name} className="max-h-16 object-contain" style={{ filter: primary === '#000000' ? 'invert(1)' : 'none' }} />
            ) : (
              <div className="heading-font text-2xl font-black tracking-widest uppercase" style={{ color: primary }}>
                {org.name}
              </div>
            )}
          </div>
          <p className="text-[10px] text-gray-400">Reversed version. Use on light backgrounds, documents, and stationery.</p>
        </div>

        {/* Icon / monogram */}
        <div className="space-y-3">
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Icon &middot; Monogram</p>
          <div
            className="rounded-2xl flex items-center justify-center p-10 min-h-[160px]"
            style={{ background: `linear-gradient(135deg, ${primary} 0%, ${primary}e0 100%)`, border: `1px solid ${accent}30` }}
          >
            <div
              className="heading-font h-20 w-20 rounded-2xl flex items-center justify-center text-3xl font-black shadow-xl"
              style={{ background: accent, color: textOn(accent), letterSpacing: '0.05em' }}
            >
              {org.name.split(' ').slice(0, 2).map((w: string) => w[0]).join('').toUpperCase()}
            </div>
          </div>
          <p className="text-[10px] text-gray-400">Icon/monogram. App icons, favicons, social avatars, embossing.</p>
        </div>
      </div>

      {/* Clear space + minimum size */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-8 mb-12">
        <div className="p-6 rounded-2xl bg-gray-50 border border-gray-100">
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-4">Clear Space Rule</p>
          <div className="relative flex items-center justify-center mb-4" style={{ height: 100 }}>
            <div className="absolute inset-4 border border-dashed border-gray-300 rounded" />
            {effectiveLogo ? (
              <img src={effectiveLogo} alt="" className="h-8 object-contain relative z-10" />
            ) : (
              <div className="heading-font text-sm font-bold" style={{ color: primary }}>{org.name.split(' ')[0]}</div>
            )}
            <div className="absolute top-1 left-1/2 -translate-x-1/2 text-[8px] text-gray-400">&times; height</div>
            <div className="absolute left-1 top-1/2 -translate-y-1/2 text-[8px] text-gray-400" style={{ writingMode: 'vertical-rl' }}>&times; height</div>
          </div>
          <p className="text-[11px] text-gray-500">Maintain clear space equal to the cap-height of the logo on all sides. Never crowd the mark.</p>
        </div>

        <div className="p-6 rounded-2xl bg-gray-50 border border-gray-100">
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-4">Minimum Sizes</p>
          <div className="space-y-3">
            {[
              ['Print', '0.75" / 19mm wide'],
              ['Digital', '80px wide'],
              ['Icon / Monogram', '24px wide'],
              ['Favicon', '16 \u00d7 16px (monogram only)'],
            ].map(([label, size]) => (
              <div key={label} className="flex items-center justify-between text-xs">
                <span className="text-gray-600 font-medium">{label}</span>
                <span className="font-mono text-gray-400">{size}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* Do / Don't */}
      <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Logo Usage -- Do's & Don'ts</p>
      <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
        {/* DO examples */}
        {[
          { ok: true,  label: 'Use on brand dark background', bg: primary, fg: accent, text: 'USE ON DARK' },
          { ok: true,  label: 'Use at full opacity', bg: '#F8F7F4', fg: primary, text: 'FULL OPACITY' },
          { ok: true,  label: 'Use with ample clear space', bg: primary, fg: accent, text: '\u00b7 \u00b7 LOGO \u00b7 \u00b7', pad: true },
        ].map((item, i) => (
          <div key={i} className="space-y-2">
            <div
              className="rounded-xl flex items-center justify-center text-xs font-bold tracking-widest border-2 border-green-400/40"
              style={{ height: 80, background: item.bg, color: item.fg, padding: item.pad ? '0 24px' : undefined }}
            >
              {item.text}
            </div>
            <p className="text-[10px] text-green-600 font-semibold flex items-center gap-1">
              <span>{'\u2713'}</span> {item.label}
            </p>
          </div>
        ))}
        {/* DON'T examples */}
        {[
          { label: 'Don\'t stretch or distort', style: { transform: 'scaleX(1.5)', color: primary } },
          { label: 'Don\'t use on clashing colours', style: { background: '#FF0040', color: '#00FF88' } },
          { label: 'Don\'t add shadows or effects', style: { textShadow: '3px 3px 6px red', color: primary } },
        ].map((item, i) => (
          <div key={i} className="space-y-2">
            <div
              className="rounded-xl flex items-center justify-center text-xs font-bold tracking-widest border-2 border-red-300/40 bg-gray-50 overflow-hidden"
              style={{ height: 80 }}
            >
              <span style={item.style}>{org.name.split(' ')[0].toUpperCase()}</span>
            </div>
            <p className="text-[10px] text-red-500 font-semibold flex items-center gap-1">
              <span>{'\u2717'}</span> {item.label}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
