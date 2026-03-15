import type React from 'react';
import {
  Linkedin,
  Instagram,
  Twitter,
  Facebook,
  Youtube,
  Globe,
} from 'lucide-react';

// ── Brand Constants ──────────────────────────────────────────────────────────

export const BRAND_VOICE_OPTIONS = ['formal', 'casual', 'playful', 'authoritative', 'bold', 'sophisticated'];
export const BRAND_ARCHETYPE_OPTIONS = ['Hero', 'Creator', 'Sage', 'Outlaw', 'Explorer', 'Ruler', 'Caregiver', 'Innocent', 'Jester', 'Lover', 'Magician', 'Regular Guy'];
export const MARKET_POSITION_OPTIONS = ['luxury', 'premium', 'mid-market', 'budget'];
export const ICP_COMPANY_SIZE_OPTIONS = ['solo', 'startup', 'smb', 'mid-market', 'enterprise'];

// ── Social Platform Constants ────────────────────────────────────────────────

export const PLATFORM_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  linkedin: Linkedin,
  instagram: Instagram,
  twitter: Twitter,
  facebook: Facebook,
  youtube: Youtube,
  tiktok: Globe,
  threads: Globe,
  bluesky: Globe,
  pinterest: Globe,
};

export const PLATFORM_COLORS: Record<string, string> = {
  linkedin: 'text-[#0A66C2]',
  instagram: 'text-[#E4405F]',
  twitter: 'text-foreground',
  facebook: 'text-[#1877F2]',
  youtube: 'text-[#FF0000]',
  tiktok: 'text-foreground',
  threads: 'text-foreground',
  bluesky: 'text-sky-500',
  pinterest: 'text-[#E60023]',
};

export const PLATFORM_BG: Record<string, string> = {
  linkedin: 'bg-[#0A66C2]/10',
  instagram: 'bg-[#E4405F]/10',
  twitter: 'bg-foreground/10',
  facebook: 'bg-[#1877F2]/10',
  youtube: 'bg-[#FF0000]/10',
  tiktok: 'bg-muted',
  bluesky: 'bg-sky-500/10',
  pinterest: 'bg-[#E60023]/10',
};

export const STATUS_COLORS: Record<string, string> = {
  draft: 'text-muted-foreground border-border',
  pending_review: 'text-amber-600 border-amber-300 bg-amber-50 dark:bg-amber-950/20',
  approved: 'text-blue-600 border-blue-300 bg-blue-50 dark:bg-blue-950/20',
  scheduled: 'text-purple-600 border-purple-300 bg-purple-50 dark:bg-purple-950/20',
  published: 'text-green-600 border-green-300 bg-green-50 dark:bg-green-950/20',
  failed: 'text-destructive border-destructive/30 bg-destructive/10',
  cancelled: 'text-muted-foreground border-border',
};

export const SENTIMENT_COLORS: Record<string, string> = {
  positive: 'text-green-600',
  negative: 'text-destructive',
  neutral: 'text-muted-foreground',
  unknown: 'text-muted-foreground',
};

export const PRIORITY_COLORS: Record<string, string> = {
  urgent: 'text-destructive',
  high: 'text-amber-600',
  normal: 'text-muted-foreground',
  low: 'text-muted-foreground/60',
};
