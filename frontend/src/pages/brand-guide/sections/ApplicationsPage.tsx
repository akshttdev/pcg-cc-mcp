import type { BrandPageProps } from '../types';
import { BusinessCardMockup, SocialPostMockup, EmailSignatureMockup, LetterheadMockup } from './Mockups';
import { PageLabel } from './PagePrimitives';

export function ApplicationsPage({ org, profile, primary, accent, effectiveLogo }: BrandPageProps) {
  return (
    <div className="print-page min-h-screen px-16 py-16 bg-white">
      <PageLabel>06 -- Brand Applications</PageLabel>

      <div className="flex items-end gap-5 mb-14">
        <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
          Brand<br />
          <span style={{ color: accent }}>Applications</span>
        </h2>
        <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-14">
        {/* Business card */}
        <div>
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Business Card</p>
          <BusinessCardMockup org={org.name} primary={primary} accent={accent} logoUrl={effectiveLogo} />
          <p className="text-[10px] text-gray-400 mt-3">Standard 3.5" &times; 2" &middot; Matte laminate &middot; Foil accent recommended</p>
        </div>

        {/* Social post */}
        <div>
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Social Media &middot; Instagram</p>
          <SocialPostMockup org={org.name} tagline={profile.tagline} primary={primary} accent={accent} logoUrl={effectiveLogo} />
        </div>

        {/* Email signature */}
        <div>
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Email Signature</p>
          <EmailSignatureMockup org={org.name} primary={primary} accent={accent} logoUrl={effectiveLogo} />
        </div>

        {/* Letterhead */}
        <div>
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Letterhead</p>
          <LetterheadMockup org={org.name} primary={primary} accent={accent} logoUrl={effectiveLogo} address={org.address} />
        </div>
      </div>
    </div>
  );
}
