import { textOn } from '../helpers';
import type { BrandPageProps } from '../types';
import { TshirtMockup, MugMockup, ToteMockup, BannerMockup } from './Mockups';
import { PageLabel } from './PagePrimitives';

const PRODUCT_CATALOG = [
  ['Business Cards', 'Matte \u00b7 Gloss \u00b7 Soft-Touch \u00b7 Foil', '100', 'Networking, client meetings'],
  ['Branded T-Shirts', 'S\u20133XL \u00b7 8 colours', '12', 'Events, team uniforms, merch'],
  ['Premium Mugs', '11oz \u00b7 15oz \u00b7 Travel', '6', 'Client gifts, office branding'],
  ['Tote Bags', 'Canvas \u00b7 Eco \u00b7 Laminated', '10', 'Events, giveaways, retail'],
  ['Roll-Up Banners', '33"\u00d780" \u00b7 47"\u00d780"', '1', 'Trade shows, conferences'],
  ['Letterhead', 'A4 \u00b7 Letter \u00b7 Premium bond', '25', 'Official correspondence'],
  ['Folders', 'A4 \u00b7 Pocket \u00b7 Gloss', '25', 'Proposals, client packages'],
  ['Pens', 'Ballpoint \u00b7 Stylus \u00b7 Metal', '25', 'Office, events, gifting'],
] as const;

export function MerchandisePage({ org, profile, primary, accent }: BrandPageProps) {
  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>07 -- Merchandise & Print</PageLabel>

      <div className="flex items-end gap-5 mb-5">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Brand<br />
          <span style={{ color: accent }}>Merchandise</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      {/* Vistaprint partnership badge */}
      <div
        className="flex items-center gap-4 rounded-2xl px-6 py-4 mb-12 border"
        style={{ borderColor: `${accent}30`, background: `${accent}08` }}
      >
        <div
          className="h-8 w-8 rounded-lg flex items-center justify-center text-xs font-black shrink-0"
          style={{ background: accent, color: textOn(accent) }}
        >
          VP
        </div>
        <div>
          <p className="text-xs font-semibold text-gray-900">Vistaprint Brand Partner</p>
          <p className="text-xs text-gray-500">All print materials and merchandise available at partner pricing through your Corporate Store.</p>
        </div>
        <div className="ml-auto">
          <span className="text-xs font-semibold uppercase tracking-wider px-3 py-1 rounded-full" style={{ background: `${accent}20`, color: accent }}>
            Exclusive Rates
          </span>
        </div>
      </div>

      {/* Product grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-10 mb-12 place-items-center">
        <TshirtMockup primary={primary} accent={accent} org={org.name} />
        <MugMockup primary={primary} accent={accent} />
        <ToteMockup primary={primary} accent={accent} />
        <BannerMockup primary={primary} accent={accent} org={org.name} tagline={profile.tagline} />
      </div>

      {/* Product table */}
      <div className="rounded-2xl border border-gray-100 overflow-hidden">
        <div className="px-5 py-3" style={{ background: primary }}>
          <p className="text-xs uppercase tracking-[0.25em] font-semibold" style={{ color: accent }}>
            Corporate Store &middot; Available Products
          </p>
        </div>
        <table className="w-full text-xs">
          <thead>
            <tr className="border-b border-gray-100">
              <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-xs">Product</th>
              <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-xs">Variants</th>
              <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-xs">Min. Order</th>
              <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-xs">Use Case</th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-50">
            {PRODUCT_CATALOG.map(([product, variants, min, useCase], i) => (
              <tr key={i} className={i % 2 === 0 ? 'bg-white' : 'bg-gray-50/50'}>
                <td className="px-5 py-3 font-medium text-gray-800">{product}</td>
                <td className="px-5 py-3 text-gray-500">{variants}</td>
                <td className="px-5 py-3 text-gray-500">{min}</td>
                <td className="px-5 py-3 text-gray-400">{useCase}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
