// Social Content Studio Types

export type SocialPlatform =
  | 'linkedin'
  | 'instagram'
  | 'twitter'
  | 'facebook'
  | 'tiktok'
  | 'bluesky'
  | 'youtube'
  | 'pinterest'
  | 'threads';

export type ContentBlockType = 'hook' | 'body' | 'cta' | 'hashtags' | 'media' | 'emoji';

export interface ContentBlock {
  id: string;
  type: ContentBlockType;
  content: string;
  metadata?: Record<string, unknown>;
}

export type ContentCategory =
  | 'events'
  | 'drinks'
  | 'community'
  | 'vibe'
  | 'behind_scenes'
  | 'promo'
  | 'education'
  | 'entertainment';

export type ContentStatus =
  | 'draft'
  | 'review'
  | 'approved'
  | 'scheduled'
  | 'published'
  | 'analyzing'
  | 'evergreen';

export type PostStatus =
  | 'draft'
  | 'pending_approval'
  | 'approved'
  | 'scheduled'
  | 'publishing'
  | 'published'
  | 'failed'
  | 'archived';

export interface PlatformAdaptation {
  platform: SocialPlatform;
  content: string;
  characterCount: number;
  characterLimit: number;
  hashtagCount: number;
  hashtagLimit: number;
  mediaIds?: string[];
  isValid: boolean;
  warnings: string[];
}

export interface SocialAccount {
  id: string;
  platform: SocialPlatform;
  account_name: string;
  account_type: 'personal' | 'business' | 'creator';
  status: 'active' | 'disconnected' | 'expired' | 'pending';
  profile_url?: string;
  avatar_url?: string;
  follower_count?: number;
  connected_at: string;
  last_synced_at?: string;
}

export interface SocialPost {
  id: string;
  account_id: string;
  task_id?: string;
  content_blocks: ContentBlock[];
  platform_adaptations: Record<SocialPlatform, PlatformAdaptation>;
  category: ContentCategory;
  status: PostStatus;
  scheduled_for?: string;
  published_at?: string;
  is_evergreen: boolean;
  metrics?: PostMetrics;
  created_at: string;
  updated_at: string;
}

export interface PostMetrics {
  impressions: number;
  reach: number;
  engagement_rate: number;
  likes: number;
  comments: number;
  shares: number;
  saves: number;
  clicks: number;
}

export interface CategoryQueue {
  id: string;
  account_id: string;
  category: ContentCategory;
  posts_per_week: number;
  optimal_times: string[]; // HH:MM format
  preferred_days: number[]; // 0-6, Sunday = 0
  is_active: boolean;
}

export interface CalendarDay {
  date: Date;
  posts: SocialPost[];
  isToday: boolean;
  isCurrentMonth: boolean;
}

export interface ContentTemplate {
  id: string;
  name: string;
  category: ContentCategory;
  blocks: ContentBlock[];
  platforms: SocialPlatform[];
  created_at: string;
}

// Platform-specific limits
export const PLATFORM_LIMITS: Record<SocialPlatform, {
  characterLimit: number;
  hashtagLimit: number;
  mediaLimit: number;
  videoMaxSeconds: number;
}> = {
  linkedin: { characterLimit: 3000, hashtagLimit: 30, mediaLimit: 9, videoMaxSeconds: 600 },
  instagram: { characterLimit: 2200, hashtagLimit: 30, mediaLimit: 10, videoMaxSeconds: 90 },
  twitter: { characterLimit: 280, hashtagLimit: 5, mediaLimit: 4, videoMaxSeconds: 140 },
  facebook: { characterLimit: 63206, hashtagLimit: 30, mediaLimit: 10, videoMaxSeconds: 240 },
  tiktok: { characterLimit: 2200, hashtagLimit: 15, mediaLimit: 1, videoMaxSeconds: 600 },
  bluesky: { characterLimit: 300, hashtagLimit: 10, mediaLimit: 4, videoMaxSeconds: 0 },
  youtube: { characterLimit: 5000, hashtagLimit: 15, mediaLimit: 1, videoMaxSeconds: 43200 },
  pinterest: { characterLimit: 500, hashtagLimit: 20, mediaLimit: 5, videoMaxSeconds: 60 },
  threads: { characterLimit: 500, hashtagLimit: 10, mediaLimit: 10, videoMaxSeconds: 300 },
};

// Category colors for calendar view
export const CATEGORY_COLORS: Record<ContentCategory, string> = {
  events: '#ef4444',      // red
  drinks: '#f97316',      // orange
  community: '#22c55e',   // green
  vibe: '#8b5cf6',        // purple
  behind_scenes: '#06b6d4', // cyan
  promo: '#eab308',       // yellow
  education: '#3b82f6',   // blue
  entertainment: '#ec4899', // pink
};

// Category icons
export const CATEGORY_ICONS: Record<ContentCategory, string> = {
  events: '🎉',
  drinks: '🍸',
  community: '👥',
  vibe: '🎵',
  behind_scenes: '🎬',
  promo: '📣',
  education: '📚',
  entertainment: '🎭',
};

// Platform icons (for display)
export const PLATFORM_ICONS: Record<SocialPlatform, string> = {
  linkedin: 'LI',
  instagram: 'IG',
  twitter: 'X',
  facebook: 'FB',
  tiktok: 'TT',
  bluesky: 'BS',
  youtube: 'YT',
  pinterest: 'PI',
  threads: 'TH',
};
