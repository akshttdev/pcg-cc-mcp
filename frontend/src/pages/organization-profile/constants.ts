import type React from 'react';
import {
  BookOpen,
  MessageSquare,
  FileText,
  Radio,
  Pencil,
  Boxes,
  Network,
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

// ── Knowledge Source Types ───────────────────────────────────────────────────

export const SOURCE_TYPE_META: Record<string, { label: string; icon: typeof BookOpen }> = {
  conversation: { label: 'Conversations', icon: MessageSquare },
  artifact: { label: 'Artifacts', icon: FileText },
  pulse_content: { label: 'Pulse Content', icon: Radio },
  context_injection: { label: 'Context Injections', icon: Pencil },
  entity: { label: 'Entities', icon: Boxes },
  topology_snapshot: { label: 'Topology Snapshots', icon: Network },
};

// ── Pipeline Types ───────────────────────────────────────────────────────────

export interface PipelineNode {
  id: string;
  label: string;
  agent?: string;
  type: 'agent' | 'human' | 'parallel' | 'tool';
  description: string;
  tools?: string[];
  parallel?: boolean;
}

export interface PipelineBlueprint {
  id: string;
  name: string;
  description: string;
  category: string;
  color: string;
  nodes: PipelineNode[];
}

export const CONFERENCE_PIPELINE: PipelineBlueprint = {
  id: 'conference_research',
  name: 'Conference Research',
  description: 'Full pipeline: conference intel → speaker/brand/side-event research (parallel) → article writing → QA → social publishing.',
  category: 'Research',
  color: 'blue',
  nodes: [
    { id: 'conf_intel', label: 'Conference Intel', agent: 'Scout', type: 'agent', description: 'Research event, venue, organizers, agenda, and key themes.' },
    { id: 'speaker_res', label: 'Speaker Research', agent: 'Scout', type: 'parallel', description: 'Profile each speaker in parallel — bio, publications, LinkedIn, social presence.', parallel: true },
    { id: 'brand_res', label: 'Brand Research', agent: 'Scout', type: 'parallel', description: 'Profile sponsors and brands in parallel — positioning, news, key contacts.', parallel: true },
    { id: 'prod_team', label: 'Production Team', agent: 'Scout', type: 'agent', description: 'Identify AV production companies, photographers, and crew.' },
    { id: 'comp_intel', label: 'Competitive Intel', agent: 'Scout', type: 'agent', description: 'Analyze competing events, positioning, and attendee overlap.' },
    { id: 'side_events', label: 'Side Events Discovery', agent: 'Scout', type: 'parallel', description: 'Discover Lu.ma, Eventbrite, and Partiful side events in parallel.', parallel: true },
    { id: 'articles', label: 'Article Writing', agent: 'Astra', type: 'agent', description: 'Write thought-leadership articles per speaker using research context.' },
    { id: 'qa', label: 'QA Review', agent: 'Astra', type: 'agent', description: 'Quality-check all content for accuracy, tone, and brand alignment.' },
    { id: 'social', label: 'Social Publishing', agent: 'Creative', type: 'agent', description: 'Schedule and publish posts across connected social accounts.' },
  ],
};

export const EDITRON_PIPELINE: PipelineBlueprint = {
  id: 'editron',
  name: 'Editron Production',
  description: 'Video production pipeline: intake → scene detection (Maci) → audio/music (Sonix) → colour → assembly → review → export.',
  category: 'Production',
  color: 'amber',
  nodes: [
    { id: 'intake', label: 'Intake & Indexing', agent: 'Nora', type: 'agent', description: 'Receive footage from Nora task, generate proxy files for fast editing.', tools: ['FFmpeg', 'Proxy Manager'] },
    { id: 'scene', label: 'Scene Detection', agent: 'Maci', type: 'agent', description: 'Shot selection via visual QC — detect scenes, label content, rank clips by quality.', tools: ['Maci', 'Visual QC'] },
    { id: 'music', label: 'Music & Sound', agent: 'Sonix', type: 'tool', description: 'Audio engineering: music recommendations, loudness normalization, compression. Libraries: Artlist, Epidemic Sound, Soundstripe.', tools: ['Sonix', 'Artlist', 'Epidemic Sound', 'Soundstripe'] },
    { id: 'color', label: 'Colour Grading', agent: 'Editron', type: 'tool', description: 'Apply LUT and colour grade presets matched to project brand guide.', tools: ['Colour Engine', 'LUTs'] },
    { id: 'assembly', label: 'Edit Assembly', agent: 'Editron', type: 'agent', description: 'Assemble timeline — clips, transitions, music sync, markers. Output Premiere .prproj.', tools: ['Edit Assembly', 'Premiere Bridge'] },
    { id: 'review', label: 'Human Review', agent: undefined, type: 'human', description: 'Creative director reviews cut, provides revision notes.' },
    { id: 'export', label: 'Export & Deliver', agent: 'Editron', type: 'tool', description: 'Final encode via Media Encoder or FFmpeg. Deliver to client asset folder.', tools: ['Media Encoder', 'FFmpeg'] },
  ],
};

export const STATIC_PIPELINES = [CONFERENCE_PIPELINE, EDITRON_PIPELINE];

export const AGENT_COLORS: Record<string, string> = {
  Scout: 'bg-blue-500/15 border-blue-500/40 text-blue-400',
  Astra: 'bg-purple-500/15 border-purple-500/40 text-purple-400',
  Creative: 'bg-pink-500/15 border-pink-500/40 text-pink-400',
  Maci: 'bg-orange-500/15 border-orange-500/40 text-orange-400',
  Sonix: 'bg-green-500/15 border-green-500/40 text-green-400',
  Nora: 'bg-indigo-500/15 border-indigo-500/40 text-indigo-400',
  Editron: 'bg-amber-500/15 border-amber-500/40 text-amber-400',
  human: 'bg-emerald-500/15 border-emerald-500/40 text-emerald-400',
};

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
