import { hexToRgb, textOn } from '../helpers';

// ── Large landscape colour swatch card ──────────────────────────────────────

export function ColourCard({ color, name, role }: { color: string; name: string; role: string }) {
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

// ── Business card mockup ────────────────────────────────────────────────────

export function BusinessCardMockup({ org, primary, accent, logoUrl }: { org: string; primary: string; accent: string; logoUrl?: string | null }) {
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

// ── Instagram post mockup ───────────────────────────────────────────────────

export function SocialPostMockup({ org, tagline, primary, accent, logoUrl }: { org: string; tagline?: string | null; primary: string; accent: string; logoUrl?: string | null }) {
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
          <span>{'\u2661'}</span><span>{'\u2708'}</span>
          <span className="ml-auto">{'\uD83D\uDD16'}</span>
        </div>
        <p className="font-semibold text-[10px] text-gray-900">{org}</p>
        {tagline && <p className="text-[9px] text-gray-500 line-clamp-2">{tagline}</p>}
      </div>
    </div>
  );
}

// ── Email signature mockup ──────────────────────────────────────────────────

export function EmailSignatureMockup({ org, primary, accent, logoUrl }: { org: string; primary: string; accent: string; logoUrl?: string | null }) {
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
            <p className="text-[10px]" style={{ color: accent }}>Creative Director &middot; {org}</p>
            <p className="text-[9px] text-gray-400 mt-1">powerclubglobal.com &middot; @powerclubglobal</p>
          </div>
        </div>
      </div>
    </div>
  );
}

// ── Letterhead mockup ───────────────────────────────────────────────────────

export function LetterheadMockup({ org, primary, accent, logoUrl, address }: { org: string; primary: string; accent: string; logoUrl?: string | null; address?: string | null }) {
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

// ── T-shirt SVG mockup ──────────────────────────────────────────────────────

export function TshirtMockup({ primary, accent, org }: { primary: string; accent: string; org: string }) {
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

// ── Mug SVG mockup ──────────────────────────────────────────────────────────

export function MugMockup({ primary, accent }: { primary: string; accent: string }) {
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

// ── Tote bag SVG mockup ─────────────────────────────────────────────────────

export function ToteMockup({ primary, accent }: { primary: string; accent: string }) {
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

// ── Roll-up banner mockup ───────────────────────────────────────────────────

export function BannerMockup({ primary, accent, org, tagline }: { primary: string; accent: string; org: string; tagline?: string | null }) {
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
