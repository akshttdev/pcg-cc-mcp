import { useState } from 'react';
import { useParams, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { organizationsApi, type OrgBrandProfile, resolveApiUrl } from '@/lib/api';
import { Loader2, ArrowLeft, Instagram, Linkedin, Twitter, Facebook, Youtube, Printer, Palette, Wand2 } from 'lucide-react';
import { Button } from '@/components/ui/button';

import type { BrandPageProps, OrgKnowledgeData } from './types';
import { BrandSetupWizard } from './BrandSetupWizard';
import { PageBreak } from './sections/PagePrimitives';
import { CoverPage } from './sections/CoverPage';
import { FoundationPage } from './sections/FoundationPage';
import { VisualIdentityPage } from './sections/VisualIdentityPage';
import { LogoSystemPage } from './sections/LogoSystemPage';
import { VoicePersonalityPage } from './sections/VoicePersonalityPage';
import { MoodBoardPage } from './sections/MoodBoardPage';
import { AudienceMarketPage } from './sections/AudienceMarketPage';
import { ApplicationsPage } from './sections/ApplicationsPage';
import { MerchandisePage } from './sections/MerchandisePage';
import { DigitalPresencePage } from './sections/DigitalPresencePage';
import { BackCover } from './sections/BackCover';

export function BrandGuidePage() {
  const { orgId } = useParams<{ orgId: string }>();
  const [wizardOpen, setWizardOpen] = useState(false);

  const { data: org, isLoading: orgLoading } = useQuery({
    queryKey: ['org', orgId],
    queryFn: () => organizationsApi.getById(orgId!),
    enabled: !!orgId,
  });

  const { data: profile, isLoading: profileLoading } = useQuery<OrgBrandProfile | null>({
    queryKey: ['orgBrandProfile', orgId],
    queryFn: () => organizationsApi.getBrandProfile(orgId!),
    enabled: !!orgId,
  });

  const { data: knowledge } = useQuery<OrgKnowledgeData>({
    queryKey: ['orgKnowledge', orgId],
    queryFn: () => organizationsApi.getKnowledge(orgId!) as Promise<OrgKnowledgeData>,
    enabled: !!orgId,
  });

  if (orgLoading || profileLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-black">
        <Loader2 className="h-6 w-6 animate-spin text-yellow-600" />
      </div>
    );
  }

  if (!org || !profile) {
    return (
      <div className="min-h-screen flex flex-col items-center bg-black text-gray-400 gap-6 pt-[20vh]">
        <div className="text-center space-y-3">
          <Palette className="h-12 w-12 mx-auto text-gray-600" />
          <h2 className="text-xl font-semibold text-gray-300">
            {org ? `No brand guide for ${org.name}` : 'Brand guide not found'}
          </h2>
          <p className="text-sm text-gray-500 max-w-md">
            Set up a brand profile to generate a comprehensive brand guide with colors, typography, voice, and positioning.
          </p>
        </div>
        {org && orgId && (
          <>
            <Button
              onClick={() => setWizardOpen(true)}
              className="gap-2"
              variant="outline"
            >
              <Wand2 className="h-4 w-4" />
              Set Up Brand Guide
            </Button>
            <BrandSetupWizard
              orgId={orgId}
              orgName={org.name}
              open={wizardOpen}
              onOpenChange={setWizardOpen}
            />
          </>
        )}
        <Link
          to={orgId ? `/organizations/${orgId}` : '/'}
          className="text-xs text-gray-600 hover:text-gray-400 transition-colors"
        >
          <ArrowLeft className="h-3 w-3 inline mr-1" />
          Back to organization
        </Link>
      </div>
    );
  }

  const primary = profile.primaryColor || '#000000';
  const secondary = profile.secondaryColor || '#FFFFFF';
  const accent = profile.accentColor || '#AF9041';
  const headingFont = profile.typographyHeading || 'serif';
  const bodyFont = profile.typographyBody || 'sans-serif';
  const logoUrl = profile.logoUrl ? resolveApiUrl(profile.logoUrl) : null;
  const clearbitLogo = profile.clearbitLogoUrl;
  const effectiveLogo = logoUrl || clearbitLogo || null;

  const socials = [
    { icon: Instagram, handle: profile.socialInstagram, label: 'Instagram', url: `https://instagram.com/${profile.socialInstagram?.replace('@', '')}` },
    { icon: Linkedin, handle: profile.socialLinkedin, label: 'LinkedIn', url: `https://linkedin.com/company/${profile.socialLinkedin?.replace('@', '')}` },
    { icon: Twitter, handle: profile.socialTwitter, label: 'X / Twitter', url: `https://x.com/${profile.socialTwitter?.replace('@', '')}` },
    { icon: Facebook, handle: profile.socialFacebook, label: 'Facebook', url: `https://facebook.com/${profile.socialFacebook?.replace('@', '')}` },
    { icon: Youtube, handle: profile.socialYoutube, label: 'YouTube', url: `https://youtube.com/${profile.socialYoutube}` },
  ].filter(s => s.handle);

  const pageProps: BrandPageProps = {
    org: { name: org.name, address: (org as any).address ?? null },
    profile,
    primary,
    secondary,
    accent,
    headingFont,
    bodyFont,
    effectiveLogo,
  };

  return (
    <>
      {/* Google Fonts */}
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600;700;900&family=Inter:wght@300;400;500;600;700&display=swap');

        .brand-guide * { box-sizing: border-box; }
        .brand-guide { font-family: '${bodyFont}', Inter, sans-serif; }
        .brand-guide h1, .brand-guide h2, .brand-guide h3, .brand-guide .heading-font {
          font-family: '${headingFont}', Cinzel, Georgia, serif;
        }
        .page-break { display: none; }

        @media print {
          @page { size: A4; margin: 0; }
          html, body { margin: 0; padding: 0; }
          .no-print { display: none !important; }
          .page-break { display: block; page-break-before: always; break-before: page; }
          .print-page {
            min-height: 297mm;
            page-break-after: always;
            break-after: page;
          }
          .cover-page {
            min-height: 297mm;
          }
        }
      `}</style>

      <div className="brand-guide min-h-screen bg-white">

        {/* Navigation bar (screen only) */}
        <div className="no-print sticky top-0 z-50 bg-black/95 backdrop-blur border-b border-yellow-900/30 px-6 py-3 flex items-center justify-between">
          <Link
            to={`/organizations/${orgId}`}
            className="flex items-center gap-1.5 text-sm text-gray-400 hover:text-white transition-colors"
          >
            <ArrowLeft className="h-4 w-4" />
            Back to {org.name}
          </Link>
          <div className="flex items-center gap-2">
            <span className="text-[10px] uppercase tracking-widest text-yellow-600 font-semibold">Brand Guide</span>
            <Button size="sm" variant="outline" className="text-xs gap-1.5 border-yellow-700/40 text-yellow-500 hover:bg-yellow-900/20 hover:text-yellow-400" onClick={() => window.print()}>
              <Printer className="h-3.5 w-3.5" />
              Print / PDF
            </Button>
          </div>
        </div>

        {/* Page 1 -- Cover */}
        <CoverPage {...pageProps} />

        <PageBreak />

        {/* Page 2 -- Foundation */}
        <FoundationPage {...pageProps} />

        <PageBreak />

        {/* Page 3 -- Colour System & Typography */}
        <VisualIdentityPage {...pageProps} />

        <PageBreak />

        {/* Page 3b -- Logo System */}
        <LogoSystemPage {...pageProps} />

        <PageBreak />

        {/* Page 4 -- Voice & Personality */}
        <VoicePersonalityPage {...pageProps} />

        {/* Page 4b -- Mood Board (conditionally rendered with its own PageBreak) */}
        <MoodBoardPage {...pageProps} />

        <PageBreak />

        {/* Page 5 -- Audience & Market */}
        <AudienceMarketPage {...pageProps} />

        <PageBreak />

        {/* Page 6 -- Brand Applications */}
        <ApplicationsPage {...pageProps} />

        <PageBreak />

        {/* Page 7 -- Merchandise & Print */}
        <MerchandisePage {...pageProps} />

        <PageBreak />

        {/* Page 8 -- Digital Presence */}
        <DigitalPresencePage {...pageProps} socials={socials} knowledge={knowledge} />

        <PageBreak />

        {/* Back Cover */}
        <BackCover {...pageProps} />

      </div>
    </>
  );
}
