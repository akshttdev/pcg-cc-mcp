/**
 * Company Brand Guide — mirrors the Organization Brand Guide
 * but fetches from /api/companies/:id/brand-profile.
 * Reuses all section components via BrandPageProps.
 */
import { useQuery } from '@tanstack/react-query';
import { ArrowLeft, Loader2, Palette, Pencil, Printer } from 'lucide-react';
import { useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';

import { Button } from '@/components/ui/button';
import { companiesApi, type OrgBrandProfile, resolveApiUrl } from '@/lib/api';
import { entityKeys } from '@/lib/query-keys';

import type { BrandApiOverride } from './BrandSetupWizard';
import { BrandSetupWizard } from './BrandSetupWizard';
import { ApplicationsPage } from './sections/ApplicationsPage';
import { AudienceMarketPage } from './sections/AudienceMarketPage';
import { BackCover } from './sections/BackCover';
import { CoverPage } from './sections/CoverPage';
import { DigitalPresencePage } from './sections/DigitalPresencePage';
import { FoundationPage } from './sections/FoundationPage';
import { LogoSystemPage } from './sections/LogoSystemPage';
import { PageBreak } from './sections/PagePrimitives';
import { VisualIdentityPage } from './sections/VisualIdentityPage';
import { VoicePersonalityPage } from './sections/VoicePersonalityPage';
import type { BrandPageProps } from './types';

export function CompanyBrandGuidePage() {
  const { companyId } = useParams<{ companyId: string }>();
  const navigate = useNavigate();
  const [wizardOpen, setWizardOpen] = useState(false);

  const companyBrandApi: BrandApiOverride = {
    upsertBrandProfile: companiesApi.upsertBrandProfile,
    invalidateKey: entityKeys.companyBrandProfile(companyId!),
  };

  const {
    data: company,
    isLoading: companyLoading,
    isError: companyError,
  } = useQuery({
    queryKey: entityKeys.company(companyId!),
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
    retry: 1,
  });

  const { data: profile, isLoading: profileLoading } =
    useQuery<OrgBrandProfile | null>({
      queryKey: entityKeys.companyBrandProfile(companyId!),
      queryFn: () => companiesApi.getBrandProfile(companyId!),
      enabled: !!companyId && !!company,
      retry: 1,
    });

  if (companyError) {
    return (
      <div className="min-h-screen flex flex-col items-center bg-black text-gray-400 gap-6 pt-[20vh]">
        <div className="text-center space-y-3">
          <Palette className="h-12 w-12 mx-auto text-gray-600" />
          <h2 className="text-xl font-semibold text-gray-300">
            Company not found
          </h2>
          <p className="text-sm text-gray-500 max-w-md">
            The company you&apos;re looking for doesn&apos;t exist or you
            don&apos;t have access.
          </p>
        </div>
        <Button
          variant="outline"
          onClick={() => navigate(-1)}
          className="gap-2"
        >
          <ArrowLeft className="h-4 w-4" /> Go Back
        </Button>
      </div>
    );
  }

  if (companyLoading || profileLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-black">
        <Loader2 className="h-6 w-6 animate-spin text-yellow-600" />
      </div>
    );
  }

  if (!company || !profile) {
    return (
      <div className="min-h-screen flex flex-col items-center bg-black text-gray-400 gap-6 pt-[20vh]">
        <div className="text-center space-y-3">
          <Palette className="h-12 w-12 mx-auto text-gray-600" />
          <h2 className="text-xl font-semibold text-gray-300">
            {company
              ? `No brand guide for ${company.name}`
              : 'Brand guide not found'}
          </h2>
          <p className="text-sm text-gray-500 max-w-md">
            Set up a brand profile to generate a comprehensive brand guide with
            colors, typography, voice, and positioning.
          </p>
        </div>
        {company && companyId && (
          <>
            <Button
              onClick={() => setWizardOpen(true)}
              className="gap-2"
              variant="outline"
            >
              <Palette className="h-4 w-4" />
              Set Up Brand Guide
            </Button>
            <BrandSetupWizard
              orgId={companyId}
              orgName={company.name}
              open={wizardOpen}
              onOpenChange={setWizardOpen}
              apiOverride={companyBrandApi}
            />
          </>
        )}
        <button
          onClick={() => navigate(-1)}
          className="text-xs text-gray-600 hover:text-gray-400 transition-colors"
        >
          <ArrowLeft className="h-3 w-3 inline mr-1" />
          Back
        </button>
      </div>
    );
  }

  const primary = profile.primaryColor || '#000000';
  const secondary = profile.secondaryColor || '#FFFFFF';
  const accent = profile.accentColor || '#AF9041';
  // Sanitize font names to prevent CSS injection via stored profile data
  const sanitizeFont = (f: string) => f.replace(/['"\\;{}()<>]/g, '');
  const headingFont = sanitizeFont(profile.typographyHeading || 'serif');
  const bodyFont = sanitizeFont(profile.typographyBody || 'sans-serif');
  const logoUrl = profile.logoUrl ? resolveApiUrl(profile.logoUrl) : null;
  const clearbitLogo = profile.clearbitLogoUrl;
  const effectiveLogo = logoUrl || clearbitLogo || company.logo_url || null;

  const pageProps: BrandPageProps = {
    org: { name: company.name, address: company.headquarters ?? null },
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
      <style>{`
        @import url('https://fonts.googleapis.com/css2?family=Cinzel:wght@400;600;700;900&family=Inter:wght@300;400;500;600;700&display=swap');
        .brand-guide * { box-sizing: border-box; }
        .brand-guide { font-family: '${bodyFont}', Inter, sans-serif; }
        .brand-guide h1, .brand-guide h2, .brand-guide h3, .brand-guide .heading-font {
          font-family: '${headingFont}', Cinzel, Georgia, serif;
        }
        .page-break { display: none; }
        @media print {
          @page { size: A4 landscape; margin: 0; }
          html, body { margin: 0; padding: 0; }
          .no-print { display: none !important; }
          .page-break { display: block; page-break-before: always; break-before: page; }
          .print-page { min-height: 210mm !important; max-height: 210mm !important; page-break-after: always; break-after: page; overflow: hidden; }
          .cover-page { min-height: 210mm !important; max-height: 210mm !important; }
          .min-h-screen { min-height: 210mm !important; }
        }
      `}</style>

      <div className="brand-guide min-h-screen bg-white">
        {/* Navigation bar */}
        <div className="no-print sticky top-0 z-50 bg-black/95 backdrop-blur border-b border-yellow-900/30 px-6 py-3 flex items-center justify-between">
          <button
            onClick={() => navigate(`/companies/${companyId}`)}
            className="flex items-center gap-1.5 text-sm text-gray-400 hover:text-white transition-colors"
          >
            <ArrowLeft className="h-4 w-4" />
            Back to {company.name}
          </button>
          <div className="flex items-center gap-2">
            <span className="text-xs uppercase tracking-widest text-yellow-600 font-semibold">
              Brand Guide
            </span>
            <Button
              size="sm"
              variant="outline"
              className="text-xs gap-1.5 border-gray-700 text-gray-400 hover:bg-gray-800 hover:text-white"
              onClick={() => setWizardOpen(true)}
            >
              <Pencil className="h-3.5 w-3.5" />
              Edit
            </Button>
            <Button
              size="sm"
              variant="outline"
              className="text-xs gap-1.5 border-yellow-700/40 text-yellow-500 hover:bg-yellow-900/20 hover:text-yellow-400"
              onClick={() => {
                const style = document.createElement('style');
                style.id = '__print-override';
                style.textContent =
                  '@page { size: A4 landscape; margin: 0; } .no-print { display: none !important; } .print-page { min-height: 210mm !important; max-height: 210mm !important; page-break-after: always; break-after: page; overflow: hidden; } .cover-page, .min-h-screen { min-height: 210mm !important; }';
                document.head.appendChild(style);
                window.print();
                setTimeout(
                  () => document.getElementById('__print-override')?.remove(),
                  1000
                );
              }}
            >
              <Printer className="h-3.5 w-3.5" />
              Print / PDF
            </Button>
          </div>
        </div>

        <CoverPage {...pageProps} />
        <PageBreak />
        <FoundationPage {...pageProps} />
        <PageBreak />
        <VisualIdentityPage {...pageProps} />
        <PageBreak />
        <LogoSystemPage {...pageProps} />
        <PageBreak />
        <VoicePersonalityPage {...pageProps} />
        <PageBreak />
        <AudienceMarketPage {...pageProps} />
        <PageBreak />
        <ApplicationsPage {...pageProps} />
        <PageBreak />
        <DigitalPresencePage
          {...pageProps}
          socials={[]}
          knowledge={undefined}
        />
        <PageBreak />
        <BackCover {...pageProps} />
      </div>

      <BrandSetupWizard
        orgId={companyId!}
        orgName={company.name}
        open={wizardOpen}
        onOpenChange={setWizardOpen}
        apiOverride={companyBrandApi}
      />
    </>
  );
}
