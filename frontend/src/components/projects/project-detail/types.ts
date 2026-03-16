import type { ComponentType } from 'react';

export type ProviderConnectionState = 'connected' | 'manual' | 'missing';

export type ProviderStatus = {
  key: string;
  label: string;
  status: ProviderConnectionState;
  detail: string;
  meta?: string;
  actionLabel?: string;
  onAction?: () => void;
};

export type IntegrationCategory = {
  key: string;
  label: string;
  icon: ComponentType<{ className?: string }>;
  accent: string;
  connectors: ProviderStatus[];
};

export type BoardMeta = {
  icon: ComponentType<{ className?: string }>;
  accentBorder: string;
  accentBackground: string;
  iconBg: string;
  iconColor: string;
  tagline: string;
  prompts: string[];
};

export interface ProjectDetailProps {
  projectId: string;
  onBack: () => void;
}

export const integrationStatusStyles: Record<ProviderConnectionState, string> = {
  connected: 'bg-emerald-100 text-emerald-700 border-emerald-200',
  manual: 'bg-amber-100 text-amber-700 border-amber-200',
  missing: 'bg-slate-100 text-slate-500 border-slate-200 dark:bg-slate-800 dark:text-slate-400 dark:border-slate-700',
};

/** Keys for integrations that are recommended (shown prominently when missing) */
export const RECOMMENDED_INTEGRATION_KEYS = new Set(['github', 'gmail', 'email']);

export const BRAND_PALETTES = [
  {
    primary: '#2563EB',
    secondary: '#EC4899',
    accent: 'from-sky-50 via-white to-indigo-100',
  },
  {
    primary: '#0EA5E9',
    secondary: '#10B981',
    accent: 'from-cyan-50 via-white to-emerald-100',
  },
  {
    primary: '#F97316',
    secondary: '#8B5CF6',
    accent: 'from-orange-50 via-white to-violet-100',
  },
  {
    primary: '#F43F5E',
    secondary: '#6366F1',
    accent: 'from-rose-50 via-white to-indigo-100',
  },
] as const;

/** Legacy localStorage key prefix for brand profile migration */
export const BRAND_PROFILE_STORAGE_PREFIX = 'brand-profile:';
