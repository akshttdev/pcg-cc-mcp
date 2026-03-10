import { useParams, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { organizationsApi, type OrgBrandProfile, type OrgKnowledgeSource, resolveApiUrl } from '@/lib/api';
import { Loader2, ArrowLeft, Globe, Instagram, Linkedin, Twitter, Facebook, Youtube, ExternalLink, MapPin, Printer } from 'lucide-react';
import { Button } from '@/components/ui/button';

// ── Utilities ─────────────────────────────────────────────────────────────────

function parseArr(v: string | null | undefined): string[] {
  if (!v) return [];
  try { return JSON.parse(v); } catch { return []; }
}

function parseUrls(v: string | null | undefined): string[] {
  return parseArr(v).filter(u => u.startsWith('http'));
}

function mixWithWhite(hex: string, amount: number): string {
  // amount 0.0=full color, 1.0=white
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  const mix = (c: number) => Math.round(c + (255 - c) * amount).toString(16).padStart(2, '0');
  return `#${mix(r)}${mix(g)}${mix(b)}`;
}

function hexToRgb(hex: string) {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `${r}, ${g}, ${b}`;
}

function luminance(hex: string) {
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function textOn(bg: string) {
  return luminance(bg) > 0.35 ? '#000000' : '#FFFFFF';
}

// ── Sub-components ─────────────────────────────────────────────────────────────

function PageBreak() {
  return <div className="page-break" />;
}

function PageLabel({ children }: { children: React.ReactNode }) {
  return (
    <p className="text-[9px] uppercase tracking-[0.25em] text-gray-300 mb-8 font-medium">{children}</p>
  );
}

// Large landscape colour swatch card
function ColourCard({ color, name, role }: { color: string; name: string; role: string }) {
  const fg = textOn(color);
  return (
    <div className="rounded-2xl overflow-hidden shadow-sm border border-black/8 flex flex-col" style={{ background: color }}>
      <div className="flex-1 min-h-[120px] p-6 flex flex-col justify-between">
        <p className="text-xs font-semibold tracking-[0.15em] uppercase" style={{ color: fg === '#FFFFFF' ? 'rgba(255,255,255,0.6)' : 'rgba(0,0,0,0.45)' }}>
          {role}
        </p>
        <p className="text-2xl font-bold font-mono" style={{ color: fg }}>{color.toUpperCase()}</p>
      </div>
      <div className="bg-white/10 backdrop-blur px-6 py-3 flex items-center justify-between">
        <p className="text-xs font-semibold" style={{ color: fg }}>{name}</p>
        <p className="text-[10px] font-mono" style={{ color: fg === '#FFFFFF' ? 'rgba(255,255,255,0.6)' : 'rgba(0,0,0,0.45)' }}>
          RGB {hexToRgb(color)}
        </p>
      </div>
    </div>
  );
}

// Business card mockup
function BusinessCardMockup({ org, primary, accent, logoUrl }: { org: string; primary: string; accent: string; logoUrl?: string | null }) {
  const fg = textOn(primary);
  return (
    <div className="relative" style={{ width: 340, height: 190 }}>
      {/* Shadow */}
      <div className="absolute inset-0 rounded-2xl shadow-2xl" style={{ background: primary, transform: 'translate(6px, 6px)', opacity: 0.25 }} />
      {/* Card */}
      <div
        className="absolute inset-0 rounded-2xl overflow-hidden flex flex-col justify-between p-6"
        style={{ background: primary, border: `1px solid ${accent}30` }}
      >
        {/* Gold rule top */}
        <div className="h-[2px] w-10 rounded-full" style={{ background: accent }} />

        {/* Logo / org name */}
        <div className="flex items-center gap-3">
          {logoUrl ? (
            <img src={logoUrl} alt={org} className="h-7 object-contain" />
          ) : (
            <div
              className="h-9 w-9 rounded-lg flex items-center justify-center text-xs font-bold"
              style={{ background: accent, color: textOn(accent) }}
            >
              {org.split(' ').slice(0, 2).map(w => w[0]).join('').toUpperCase()}
            </div>
          )}
        </div>

        {/* Contact info bottom */}
        <div>
          <p className="text-[10px] font-semibold tracking-[0.12em] uppercase mb-0.5" style={{ color: accent }}>
            Creative Direction
          </p>
          <p className="text-xs font-medium" style={{ color: fg === '#FFFFFF' ? 'rgba(255,255,255,0.5)' : 'rgba(0,0,0,0.4)' }}>
            powerclubglobal.com
          </p>
        </div>

        {/* Decorative corner */}
        <div
          className="absolute bottom-0 right-0 h-20 w-20 rounded-tl-full opacity-20"
          style={{ background: accent }}
        />
      </div>
    </div>
  );
}

// Instagram post mockup
function SocialPostMockup({ org, tagline, primary, accent, logoUrl }: { org: string; tagline?: string | null; primary: string; accent: string; logoUrl?: string | null }) {
  return (
    <div className="w-52 rounded-2xl overflow-hidden shadow-lg border border-gray-100 bg-white text-xs">
      {/* Instagram chrome */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-gray-100">
        <div className="h-6 w-6 rounded-full bg-gradient-to-tr from-yellow-400 via-red-400 to-purple-600 flex items-center justify-center">
          {logoUrl ? (
            <img src={logoUrl} alt="" className="h-5 w-5 rounded-full object-cover" />
          ) : (
            <span className="text-white font-bold text-[8px]">PCG</span>
          )}
        </div>
        <div>
          <p className="font-semibold text-[10px] text-gray-900 leading-none">powerclubglobal</p>
          <p className="text-[8px] text-gray-400">Sponsored</p>
        </div>
      </div>
      {/* Post image */}
      <div
        className="aspect-square flex flex-col items-center justify-center p-4 gap-2"
        style={{ background: `linear-gradient(135deg, ${primary} 0%, ${primary}dd 60%, ${accent}44 100%)` }}
      >
        {logoUrl ? (
          <img src={logoUrl} alt={org} className="h-12 object-contain drop-shadow-md" />
        ) : (
          <div className="h-10 w-10 rounded-xl flex items-center justify-center font-bold text-sm" style={{ background: accent, color: textOn(accent) }}>
            {org.split(' ').slice(0, 2).map(w => w[0]).join('').toUpperCase()}
          </div>
        )}
        {tagline && (
          <p className="text-center text-[9px] font-medium leading-tight max-w-full px-2" style={{ color: textOn(primary) === '#FFFFFF' ? 'rgba(255,255,255,0.85)' : 'rgba(0,0,0,0.7)' }}>
            {tagline}
          </p>
        )}
      </div>
      {/* Engagement row */}
      <div className="px-3 py-2 space-y-1">
        <div className="flex gap-2 text-gray-600">
          <span>♡</span><span>✈</span>
          <span className="ml-auto">🔖</span>
        </div>
        <p className="font-semibold text-[10px] text-gray-900">{org}</p>
        {tagline && <p className="text-[9px] text-gray-500 line-clamp-2">{tagline}</p>}
      </div>
    </div>
  );
}

// Email signature mockup
function EmailSignatureMockup({ org, primary, accent, logoUrl }: { org: string; primary: string; accent: string; logoUrl?: string | null }) {
  return (
    <div className="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden">
      <div className="bg-gray-50 px-4 py-2 text-[10px] text-gray-400 border-b border-gray-100 font-mono">
        From: firstname@powerclubglobal.com
      </div>
      <div className="p-5">
        <p className="text-[10px] text-gray-400 mb-4 italic">... previous message text ...</p>
        <div className="border-t pt-4 flex items-start gap-3">
          <div className="h-px w-0.5 self-stretch rounded-full" style={{ background: accent }} />
          <div>
            {logoUrl && <img src={logoUrl} alt={org} className="h-5 object-contain mb-2" style={{ filter: primary === '#000000' ? 'none' : undefined }} />}
            <p className="font-bold text-[11px] text-gray-900">Alex Martinez</p>
            <p className="text-[10px]" style={{ color: accent }}>Creative Director · {org}</p>
            <p className="text-[9px] text-gray-400 mt-1">powerclubglobal.com · @powerclubglobal</p>
          </div>
        </div>
      </div>
    </div>
  );
}

// Letterhead mockup
function LetterheadMockup({ org, primary, accent, logoUrl, address }: { org: string; primary: string; accent: string; logoUrl?: string | null; address?: string | null }) {
  const fg = textOn(primary);
  return (
    <div className="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden text-[9px]" style={{ aspectRatio: '8.5/11', maxWidth: 240 }}>
      {/* Header bar */}
      <div className="px-5 py-3 flex items-center justify-between" style={{ background: primary }}>
        {logoUrl ? (
          <img src={logoUrl} alt={org} className="h-4 object-contain" />
        ) : (
          <span className="font-bold text-xs tracking-wide" style={{ color: fg }}>{org}</span>
        )}
        <div className="h-[2px] w-8 rounded-full" style={{ background: accent }} />
      </div>
      {/* Body lines */}
      <div className="px-5 py-4 space-y-1">
        <div className="h-1.5 bg-gray-100 rounded w-1/3 mb-3" />
        {[80, 95, 72, 88, 60].map((w, i) => (
          <div key={i} className="h-1 bg-gray-100 rounded" style={{ width: `${w}%` }} />
        ))}
        <div className="h-3" />
        {[90, 78, 85, 70].map((w, i) => (
          <div key={i} className="h-1 bg-gray-100 rounded" style={{ width: `${w}%` }} />
        ))}
      </div>
      {/* Footer */}
      <div className="absolute bottom-0 left-0 right-0 px-5 py-3 border-t text-[8px] flex justify-between" style={{ borderColor: `${accent}40`, color: accent }}>
        <span>{address || 'powerclubglobal.com'}</span>
        <span>{org}</span>
      </div>
    </div>
  );
}

// T-shirt SVG mockup
function TshirtMockup({ primary, accent, org }: { primary: string; accent: string; org: string }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <svg viewBox="0 0 200 200" width="140" height="140" xmlns="http://www.w3.org/2000/svg">
        {/* Shirt body */}
        <path
          d="M60,30 L30,60 L50,70 L50,170 L150,170 L150,70 L170,60 L140,30 Q120,45 100,45 Q80,45 60,30 Z"
          fill={primary}
          stroke={accent}
          strokeWidth="1.5"
        />
        {/* Collar */}
        <path d="M80,30 Q100,55 120,30" fill="none" stroke={accent} strokeWidth="1.5" />
        {/* Left sleeve */}
        <path d="M60,30 L30,60 L50,70 L65,50 Z" fill={primary} stroke={accent} strokeWidth="1.5" />
        {/* Right sleeve */}
        <path d="M140,30 L170,60 L150,70 L135,50 Z" fill={primary} stroke={accent} strokeWidth="1.5" />
        {/* Brand mark on chest */}
        <text x="100" y="115" textAnchor="middle" fontSize="11" fontWeight="bold" fontFamily="serif" fill={accent}>
          {org.split(' ').slice(0, 2).map(w => w[0]).join('').toUpperCase()}
        </text>
        <line x1="75" y1="120" x2="125" y2="120" stroke={accent} strokeWidth="0.8" opacity="0.5" />
      </svg>
      <p className="text-[10px] text-gray-500 font-medium">Branded T-Shirt</p>
    </div>
  );
}

// Mug SVG mockup
function MugMockup({ primary, accent }: { primary: string; accent: string }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <svg viewBox="0 0 140 160" width="100" height="120" xmlns="http://www.w3.org/2000/svg">
        {/* Mug body */}
        <rect x="20" y="30" width="80" height="100" rx="8" fill={primary} stroke={accent} strokeWidth="1.5" />
        {/* Mug top ellipse */}
        <ellipse cx="60" cy="30" rx="40" ry="10" fill={primary} stroke={accent} strokeWidth="1.5" />
        {/* Liquid top */}
        <ellipse cx="60" cy="30" rx="37" ry="8" fill={accent} opacity="0.3" />
        {/* Handle */}
        <path d="M100,55 Q130,55 130,80 Q130,105 100,105" fill="none" stroke={accent} strokeWidth="1.5" />
        {/* Brand line on mug */}
        <text x="60" y="85" textAnchor="middle" fontSize="10" fontWeight="bold" fontFamily="serif" fill={accent}>PCG</text>
      </svg>
      <p className="text-[10px] text-gray-500 font-medium">Branded Mug</p>
    </div>
  );
}

// Tote bag SVG mockup
function ToteMockup({ primary, accent }: { primary: string; accent: string }) {
  return (
    <div className="flex flex-col items-center gap-2">
      <svg viewBox="0 0 160 180" width="110" height="130" xmlns="http://www.w3.org/2000/svg">
        {/* Bag body */}
        <path d="M30,60 L15,160 L145,160 L130,60 Z" fill={primary} stroke={accent} strokeWidth="1.5" />
        {/* Top rim */}
        <rect x="25" y="50" width="110" height="15" rx="4" fill={primary} stroke={accent} strokeWidth="1.5" />
        {/* Left handle */}
        <path d="M50,50 Q45,20 60,15 Q75,10 70,50" fill="none" stroke={accent} strokeWidth="2.5" strokeLinecap="round" />
        {/* Right handle */}
        <path d="M90,50 Q85,20 100,15 Q115,10 110,50" fill="none" stroke={accent} strokeWidth="2.5" strokeLinecap="round" />
        {/* Brand text */}
        <text x="80" y="115" textAnchor="middle" fontSize="10" fontWeight="bold" fontFamily="serif" fill={accent}>PCG</text>
        <line x1="55" y1="120" x2="105" y2="120" stroke={accent} strokeWidth="0.8" opacity="0.5" />
      </svg>
      <p className="text-[10px] text-gray-500 font-medium">Branded Tote</p>
    </div>
  );
}

// Roll-up banner mockup
function BannerMockup({ primary, accent, org, tagline }: { primary: string; accent: string; org: string; tagline?: string | null }) {
  const fg = textOn(primary);
  return (
    <div className="flex flex-col items-center gap-2">
      <div className="flex flex-col items-center">
        {/* Banner */}
        <div
          className="rounded-t-lg overflow-hidden flex flex-col items-center justify-between py-4 px-3"
          style={{ width: 70, height: 140, background: `linear-gradient(180deg, ${primary} 0%, ${primary}f0 70%, ${accent}33 100%)`, border: `1.5px solid ${accent}` }}
        >
          <div className="h-px w-8 rounded-full" style={{ background: accent }} />
          <div className="text-center">
            <p className="text-[8px] font-bold tracking-widest" style={{ color: accent }}>PCG</p>
            <p className="text-[6px] font-medium leading-tight mt-0.5" style={{ color: fg === '#FFFFFF' ? 'rgba(255,255,255,0.7)' : 'rgba(0,0,0,0.5)' }}>
              {org.split(' ').slice(0, 2).join('\n')}
            </p>
          </div>
          {tagline && (
            <p className="text-[5.5px] text-center leading-tight px-1" style={{ color: fg === '#FFFFFF' ? 'rgba(255,255,255,0.55)' : 'rgba(0,0,0,0.4)' }}>
              {tagline.slice(0, 40)}
            </p>
          )}
          <div className="h-px w-8 rounded-full" style={{ background: accent }} />
        </div>
        {/* Stand base */}
        <div className="h-1 w-16 rounded-full" style={{ background: accent, opacity: 0.5 }} />
        <div className="h-2 w-1 mx-auto" style={{ background: accent, opacity: 0.3 }} />
        <div className="h-0.5 w-20 rounded-full" style={{ background: accent, opacity: 0.25 }} />
      </div>
      <p className="text-[10px] text-gray-500 font-medium">Roll-Up Banner</p>
    </div>
  );
}

// ── Main Page ─────────────────────────────────────────────────────────────────

export function BrandGuidePage() {
  const { orgId } = useParams<{ orgId: string }>();

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

  const { data: knowledge } = useQuery<{ knowledge_entries?: OrgKnowledgeSource[]; stats?: { knowledge_entry_count: number } }>({
    queryKey: ['orgKnowledge', orgId],
    queryFn: () => organizationsApi.getKnowledge(orgId!) as Promise<{ knowledge_entries?: OrgKnowledgeSource[]; stats?: { knowledge_entry_count: number } }>,
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
      <div className="min-h-screen flex items-center justify-center bg-black text-gray-500">
        Brand guide not found.
      </div>
    );
  }

  const values = parseArr(profile.brandValues);
  const pillars = parseArr(profile.contentPillars);
  const competitors = parseArr(profile.competitorBrands);
  const differentiators = parseArr(profile.differentiators);
  const icpIndustries = parseArr(profile.icpIndustries);

  const primary = profile.primaryColor || '#000000';
  const secondary = profile.secondaryColor || '#FFFFFF';
  const accent = profile.accentColor || '#AF9041';
  const headingFont = profile.typographyHeading || 'serif';
  const bodyFont = profile.typographyBody || 'sans-serif';
  const logoUrl = profile.logoUrl ? resolveApiUrl(profile.logoUrl) : null;

  const moodUrls = parseUrls(profile.moodBoardUrls);
  const clearbitLogo = profile.clearbitLogoUrl;
  // Use clearbit as fallback if no stored logo
  const effectiveLogo = logoUrl || clearbitLogo;

  // Extended colour system — tints of primary and accent
  const primaryTints = [0.85, 0.65, 0.45, 0.25].map(a => mixWithWhite(primary === '#000000' ? '#1A1A2E' : primary, a));
  const accentTints  = [0.85, 0.65, 0.45, 0.25].map(a => mixWithWhite(accent, a));

  const socials = [
    { icon: Instagram, handle: profile.socialInstagram, label: 'Instagram', url: `https://instagram.com/${profile.socialInstagram?.replace('@', '')}` },
    { icon: Linkedin, handle: profile.socialLinkedin, label: 'LinkedIn', url: `https://linkedin.com/company/${profile.socialLinkedin?.replace('@', '')}` },
    { icon: Twitter, handle: profile.socialTwitter, label: 'X / Twitter', url: `https://x.com/${profile.socialTwitter?.replace('@', '')}` },
    { icon: Facebook, handle: profile.socialFacebook, label: 'Facebook', url: `https://facebook.com/${profile.socialFacebook?.replace('@', '')}` },
    { icon: Youtube, handle: profile.socialYoutube, label: 'YouTube', url: `https://youtube.com/${profile.socialYoutube}` },
  ].filter(s => s.handle);

  const voiceAttributes = [
    'Authoritative', 'Ambitious', 'Precise', 'Visionary', 'Elite',
  ];

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

        {/* ── NAVIGATION BAR (screen only) ─────────────────────────────────── */}
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

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 1 — COVER                                                    */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div
          className="cover-page print-page relative flex flex-col min-h-screen overflow-hidden"
          style={{ background: primary }}
        >
          {/* Geometric decoration — large gold arc */}
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
                  "{profile.tagline}"
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
              Brand Standards · Confidential
            </p>
            <div className="h-px flex-1" style={{ background: accent, opacity: 0.3 }} />
            {(org as any).address && (
              <p className="text-[9px] tracking-wider" style={{ color: `${secondary}40` }}>{(org as any).address}</p>
            )}
          </div>
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 2 — FOUNDATION                                               */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>02 — Foundation</PageLabel>

          {/* Section title */}
          <div className="flex items-end gap-5 mb-14">
            <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
              Brand<br />
              <span style={{ color: accent }}>Foundation</span>
            </h2>
            <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-10 mb-14">
            {/* Mission */}
            {profile.missionStatement && (
              <div className="space-y-4">
                <div className="flex items-center gap-3">
                  <div className="h-5 w-1 rounded-full" style={{ background: accent }} />
                  <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Mission</p>
                </div>
                <p
                  className="heading-font text-xl font-semibold text-gray-900 leading-relaxed"
                  style={{ lineHeight: 1.6 }}
                >
                  {profile.missionStatement}
                </p>
              </div>
            )}

            {/* Vision */}
            {profile.visionStatement && (
              <div className="space-y-4">
                <div className="flex items-center gap-3">
                  <div className="h-5 w-1 rounded-full" style={{ background: primary }} />
                  <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Vision</p>
                </div>
                <p
                  className="heading-font text-xl font-semibold text-gray-900 leading-relaxed"
                  style={{ lineHeight: 1.6 }}
                >
                  {profile.visionStatement}
                </p>
              </div>
            )}
          </div>

          {/* UVP — large callout */}
          {profile.uniqueValueProposition && (
            <div
              className="rounded-3xl p-10 mb-14"
              style={{ background: primary }}
            >
              <p className="text-[10px] uppercase tracking-[0.3em] mb-4" style={{ color: accent }}>Unique Value Proposition</p>
              <p
                className="heading-font text-xl font-semibold leading-relaxed"
                style={{ color: secondary, lineHeight: 1.7 }}
              >
                "{profile.uniqueValueProposition}"
              </p>
            </div>
          )}

          {/* Core Values */}
          {values.length > 0 && (
            <div>
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-6">Core Values</p>
              <div className="grid grid-cols-2 md:grid-cols-5 gap-3">
                {values.map((v, i) => (
                  <div
                    key={i}
                    className="rounded-2xl p-5 flex flex-col items-center justify-center text-center gap-2"
                    style={{ background: i === 0 ? primary : `${accent}12`, borderTop: `3px solid ${accent}` }}
                  >
                    <span
                      className="heading-font text-xs font-bold uppercase tracking-wider"
                      style={{ color: i === 0 ? secondary : primary }}
                    >
                      {v}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 3 — COLOUR SYSTEM                                            */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>03 — Visual Identity</PageLabel>

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
            <p className="text-[9px] uppercase tracking-[0.2em] text-gray-400 mb-3">Extended Palette — Tints</p>
            <div className="flex gap-2 flex-wrap">
              {[...primaryTints.map((t, i) => ({ color: t, label: `Primary ${(i + 1) * 25}%` })),
                ...accentTints.map((t, i) => ({ color: t, label: `Accent ${(i + 1) * 25}%` }))
              ].map(({ color, label: _label }, i) => (
                <div key={i} className="flex flex-col items-center gap-1">
                  <div className="h-10 w-10 rounded-lg border border-black/8 shadow-sm" style={{ background: color }} />
                  <p className="text-[8px] font-mono text-gray-400">{color}</p>
                </div>
              ))}
            </div>
          </div>

          {/* Usage guidance */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-5 mb-14 text-[11px]">
            <div className="rounded-xl border border-gray-100 p-4 space-y-1">
              <p className="font-semibold text-gray-700">Primary — Midnight</p>
              <p className="text-gray-500">Backgrounds, hero sections, primary CTAs, print media base.</p>
            </div>
            <div className="rounded-xl border border-gray-100 p-4 space-y-1">
              <p className="font-semibold text-gray-700">Secondary — Ivory</p>
              <p className="text-gray-500">Body text on dark backgrounds, content pages, whitespace.</p>
            </div>
            <div className="rounded-xl border border-gray-100 p-4 space-y-1">
              <p className="font-semibold text-gray-700">Accent — Gold</p>
              <p className="text-gray-500">Rules, highlights, borders, CTAs on dark, award elements.</p>
            </div>
          </div>

          {/* Typography section */}
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-8">Typography System</p>

          {/* Heading specimen */}
          <div className="overflow-hidden mb-6">
            <p className="text-[10px] text-gray-400 mb-2 uppercase tracking-widest">Display / Headings</p>
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
            <p className="text-[10px] text-gray-400 mt-1">{headingFont}</p>
          </div>

          <div className="h-px bg-gray-100 mb-6" />

          {/* Body specimen */}
          <div>
            <p className="text-[10px] text-gray-400 mb-2 uppercase tracking-widest">Body / Interface</p>
            <p
              className="text-2xl text-gray-700 leading-relaxed"
              style={{ fontFamily: `'${bodyFont}', Inter, sans-serif` }}
            >
              The quick brown fox jumps over the lazy dog. 0123456789
            </p>
            <p className="text-[10px] text-gray-400 mt-1">{bodyFont}</p>
          </div>
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 3b — LOGO SYSTEM                                             */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>03b — Logo System</PageLabel>

          <div className="flex items-end gap-5 mb-14">
            <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
              Logo<br />
              <span style={{ color: accent }}>System</span>
            </h2>
            <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
          </div>

          {/* Three logo variants */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-12">
            {/* Primary — on dark */}
            <div className="space-y-3">
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Primary · On Dark</p>
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

            {/* Reversed — on light */}
            <div className="space-y-3">
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Reversed · On Light</p>
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
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400">Icon · Monogram</p>
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
                <div className="absolute top-1 left-1/2 -translate-x-1/2 text-[8px] text-gray-400">× height</div>
                <div className="absolute left-1 top-1/2 -translate-y-1/2 text-[8px] text-gray-400" style={{ writingMode: 'vertical-rl' }}>× height</div>
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
                  ['Favicon', '16 × 16px (monogram only)'],
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
          <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Logo Usage — Do's & Don'ts</p>
          <div className="grid grid-cols-2 md:grid-cols-3 gap-4">
            {/* DO examples */}
            {[
              { ok: true,  label: 'Use on brand dark background', bg: primary, fg: accent, text: 'USE ON DARK' },
              { ok: true,  label: 'Use at full opacity', bg: '#F8F7F4', fg: primary, text: 'FULL OPACITY' },
              { ok: true,  label: 'Use with ample clear space', bg: primary, fg: accent, text: '· · LOGO · ·', pad: true },
            ].map((item, i) => (
              <div key={i} className="space-y-2">
                <div
                  className="rounded-xl flex items-center justify-center text-xs font-bold tracking-widest border-2 border-green-400/40"
                  style={{ height: 80, background: item.bg, color: item.fg, padding: item.pad ? '0 24px' : undefined }}
                >
                  {item.text}
                </div>
                <p className="text-[10px] text-green-600 font-semibold flex items-center gap-1">
                  <span>✓</span> {item.label}
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
                  <span>✗</span> {item.label}
                </p>
              </div>
            ))}
          </div>
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 4 — BRAND VOICE                                              */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>04 — Brand Personality</PageLabel>

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
                <p className="text-[10px] uppercase tracking-[0.3em] mb-2" style={{ color: `${accent}80` }}>Brand Archetype</p>
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
            <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Voice Attributes</p>
            <div className="grid grid-cols-3 md:grid-cols-5 gap-3">
              {voiceAttributes.map((attr, i) => (
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
                <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-3">Brand Voice in Action</p>
                <p
                  className="heading-font text-3xl font-bold text-gray-900 leading-relaxed italic"
                >
                  "{profile.tagline}"
                </p>
              </div>
            </div>
          )}

          {/* Content pillars */}
          {pillars.length > 0 && (
            <div>
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Content Pillars</p>
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

        {/* Mood Board — only renders if DALL-E images are available */}
        {moodUrls.length > 0 && (
          <>
            <PageBreak />
            <div className="print-page min-h-screen px-0 py-0 bg-black overflow-hidden">
              <div className="px-16 pt-16 pb-8">
                <PageLabel>04b — Brand Mood Board</PageLabel>
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
        )}

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 5 — AUDIENCE & COMPETITIVE                                   */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>05 — Audience & Market</PageLabel>

          <div className="flex items-end gap-5 mb-14">
            <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
              Audience<br />
              <span style={{ color: accent }}>& Market</span>
            </h2>
            <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-10 mb-12">
            {/* ICP Card */}
            {profile.icpDescription && (
              <div>
                <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Ideal Customer Profile</p>
                <div
                  className="rounded-3xl overflow-hidden shadow-sm border"
                  style={{ borderColor: `${accent}30` }}
                >
                  <div className="px-6 py-3 flex items-center gap-2" style={{ background: primary }}>
                    <div className="h-6 w-6 rounded-full flex items-center justify-center text-xs font-bold" style={{ background: accent, color: textOn(accent) }}>✦</div>
                    <span className="text-xs font-semibold tracking-widest uppercase" style={{ color: accent }}>ICP</span>
                    {profile.icpCompanySize && (
                      <span className="ml-auto text-[10px] capitalize rounded-full px-2 py-0.5" style={{ background: `${accent}30`, color: accent }}>
                        {profile.icpCompanySize}
                      </span>
                    )}
                  </div>
                  <div className="p-6 space-y-4">
                    <p className="text-sm text-gray-700 leading-relaxed">{profile.icpDescription}</p>
                    {icpIndustries.length > 0 && (
                      <div className="flex flex-wrap gap-1.5">
                        {icpIndustries.map((ind, i) => (
                          <span key={i} className="px-2 py-0.5 rounded-full text-[10px] font-medium bg-gray-100 text-gray-600">{ind}</span>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              </div>
            )}

            {/* Target audience */}
            {profile.targetAudience && (
              <div>
                <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Target Audience</p>
                <p className="text-base text-gray-700 leading-relaxed">{profile.targetAudience}</p>
              </div>
            )}
          </div>

          <div className="h-px bg-gray-100 mb-12" />

          {/* Competitive landscape */}
          {(competitors.length > 0 || differentiators.length > 0) && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-10">
              {competitors.length > 0 && (
                <div>
                  <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Competitive Set</p>
                  <div className="space-y-2">
                    {competitors.map((c, i) => (
                      <div key={i} className="flex items-center gap-3 p-3 rounded-xl bg-gray-50">
                        <span className="h-2 w-2 rounded-full shrink-0 bg-gray-300" />
                        <span className="text-sm text-gray-700 font-medium">{c}</span>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {differentiators.length > 0 && (
                <div>
                  <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Key Differentiators</p>
                  <div className="space-y-3">
                    {differentiators.map((d, i) => (
                      <div key={i} className="flex items-start gap-3">
                        <div className="h-1.5 w-1.5 rounded-full mt-1.5 shrink-0" style={{ background: accent }} />
                        <p className="text-sm text-gray-700">{d}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 6 — BRAND APPLICATIONS                                       */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>06 — Brand Applications</PageLabel>

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
              <p className="text-[10px] text-gray-400 mt-3">Standard 3.5" × 2" · Matte laminate · Foil accent recommended</p>
            </div>

            {/* Social post */}
            <div>
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold text-gray-400 mb-5">Social Media · Instagram</p>
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
              <LetterheadMockup org={org.name} primary={primary} accent={accent} logoUrl={effectiveLogo} address={(org as any).address} />
            </div>
          </div>
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 7 — MERCHANDISE & PRINT STORE                                */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>07 — Merchandise & Print</PageLabel>

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
              <p className="text-[10px] text-gray-500">All print materials and merchandise available at partner pricing through your Corporate Store.</p>
            </div>
            <div className="ml-auto">
              <span className="text-[10px] font-semibold uppercase tracking-wider px-3 py-1 rounded-full" style={{ background: `${accent}20`, color: accent }}>
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
              <p className="text-[10px] uppercase tracking-[0.25em] font-semibold" style={{ color: accent }}>
                Corporate Store · Available Products
              </p>
            </div>
            <table className="w-full text-xs">
              <thead>
                <tr className="border-b border-gray-100">
                  <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-[10px]">Product</th>
                  <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-[10px]">Variants</th>
                  <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-[10px]">Min. Order</th>
                  <th className="text-left px-5 py-3 font-semibold text-gray-500 uppercase tracking-wider text-[10px]">Use Case</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-50">
                {[
                  ['Business Cards', 'Matte · Gloss · Soft-Touch · Foil', '100', 'Networking, client meetings'],
                  ['Branded T-Shirts', 'S–3XL · 8 colours', '12', 'Events, team uniforms, merch'],
                  ['Premium Mugs', '11oz · 15oz · Travel', '6', 'Client gifts, office branding'],
                  ['Tote Bags', 'Canvas · Eco · Laminated', '10', 'Events, giveaways, retail'],
                  ['Roll-Up Banners', '33"×80" · 47"×80"', '1', 'Trade shows, conferences'],
                  ['Letterhead', 'A4 · Letter · Premium bond', '25', 'Official correspondence'],
                  ['Folders', 'A4 · Pocket · Gloss', '25', 'Proposals, client packages'],
                  ['Pens', 'Ballpoint · Stylus · Metal', '25', 'Office, events, gifting'],
                ].map(([product, variants, min, useCase], i) => (
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

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* PAGE 8 — DIGITAL PRESENCE & ASSETS                                */}
        {/* ══════════════════════════════════════════════════════════════════ */}
        <div className="print-page min-h-screen px-16 py-16 bg-white">
          <PageLabel>08 — Digital Presence</PageLabel>

          <div className="flex items-end gap-5 mb-14">
            <h2 className="heading-font text-6xl font-black text-gray-900 leading-none tracking-tight">
              Digital<br />
              <span style={{ color: accent }}>Presence</span>
            </h2>
            <div className="h-px flex-1 mb-3" style={{ background: accent, opacity: 0.3 }} />
          </div>

          {/* HQ address */}
          {(org as any).address && (
            <div
              className="flex items-center gap-4 rounded-2xl px-6 py-4 mb-8 border"
              style={{ borderColor: `${accent}30` }}
            >
              <MapPin className="h-5 w-5 shrink-0" style={{ color: accent }} />
              <div>
                <p className="text-[10px] uppercase tracking-widest text-gray-400 mb-0.5">Headquarters</p>
                <p className="text-sm font-semibold text-gray-800">{(org as any).address}</p>
              </div>
            </div>
          )}

          {/* Website */}
          {profile.websiteUrl && (
            <a
              href={profile.websiteUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="flex items-center gap-4 rounded-2xl px-6 py-4 mb-4 border hover:shadow-sm transition-shadow group"
              style={{ borderColor: `${accent}20` }}
            >
              <Globe className="h-5 w-5 shrink-0 text-gray-400" />
              <div className="flex-1">
                <p className="text-[10px] uppercase tracking-widest text-gray-400 mb-0.5">Website</p>
                <p className="text-sm font-semibold text-gray-800">{profile.websiteUrl}</p>
              </div>
              <ExternalLink className="h-4 w-4 text-gray-300 group-hover:text-gray-500" />
            </a>
          )}

          {/* Social handles */}
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mb-14">
            {socials.map((s, i) => {
              const Icon = s.icon;
              return (
                <a
                  key={i}
                  href={s.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="flex items-center gap-4 rounded-2xl px-5 py-4 border hover:shadow-sm transition-shadow group"
                  style={{ borderColor: `${accent}20` }}
                >
                  <Icon className="h-4 w-4 shrink-0 text-gray-400" />
                  <div className="flex-1">
                    <p className="text-[10px] text-gray-400">{s.label}</p>
                    <p className="text-xs font-semibold text-gray-800">{s.handle}</p>
                  </div>
                  <ExternalLink className="h-3.5 w-3.5 text-gray-300 group-hover:text-gray-500" />
                </a>
              );
            })}
          </div>

          {/* Research summary */}
          {profile.researchSummary && (
            <div
              className="rounded-3xl p-8"
              style={{ background: `${accent}10`, border: `1px solid ${accent}30` }}
            >
              <p className="text-[10px] uppercase tracking-[0.3em] mb-3" style={{ color: accent }}>
                Brand Intelligence Report · {profile.researchRanAt ? new Date(profile.researchRanAt).toLocaleDateString('en-GB', { year: 'numeric', month: 'long' }) : 'Recent'}
              </p>
              <p className="text-sm text-gray-700 leading-relaxed">{profile.researchSummary}</p>
            </div>
          )}

          {/* Knowledge graph entities */}
          {knowledge?.knowledge_entries && knowledge.knowledge_entries.length > 0 && (
            <div>
              <p className="text-[10px] uppercase tracking-[0.3em] mb-4 mt-8" style={{ color: accent }}>
                Knowledge Graph · {knowledge.knowledge_entries.length} Intelligence Sources
              </p>
              <div className="grid grid-cols-2 gap-3">
                {knowledge.knowledge_entries.map((src: OrgKnowledgeSource) => (
                  <div
                    key={src.id}
                    className="rounded-2xl p-5"
                    style={{ background: `${primary}08`, border: `1px solid ${primary}15` }}
                  >
                    <div className="flex items-start justify-between gap-2 mb-2">
                      <p className="text-xs font-semibold text-gray-800 leading-tight">{src.source_title}</p>
                      <span
                        className="text-[10px] font-mono shrink-0 px-1.5 py-0.5 rounded-full"
                        style={{ background: `${accent}20`, color: accent }}
                      >
                        {Math.round(src.coverage_score * 100)}%
                      </span>
                    </div>
                    {src.source_summary && (
                      <p className="text-[11px] text-gray-500 leading-relaxed line-clamp-3">{src.source_summary}</p>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <PageBreak />

        {/* ══════════════════════════════════════════════════════════════════ */}
        {/* BACK COVER                                                         */}
        {/* ══════════════════════════════════════════════════════════════════ */}
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
                "{profile.tagline}"
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
              Brand Standards Guide · {new Date().getFullYear()} · Confidential
            </p>
          </div>
        </div>

      </div>
    </>
  );
}
