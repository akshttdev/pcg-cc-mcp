import type { OrgBrandProfile, OrgKnowledgeSource } from '@/lib/api';

/** Common props shared by all brand guide page sections */
export interface BrandPageProps {
  org: { name: string; address?: string | null };
  profile: OrgBrandProfile;
  primary: string;
  secondary: string;
  accent: string;
  headingFont: string;
  bodyFont: string;
  effectiveLogo: string | null;
}

/** Social link entry used in Digital Presence section */
export interface SocialEntry {
  icon: React.ComponentType<{ className?: string }>;
  handle: string | null | undefined;
  label: string;
  url: string;
}

/** Knowledge data shape from the org knowledge query */
export interface OrgKnowledgeData {
  knowledge_entries?: OrgKnowledgeSource[];
  stats?: { knowledge_entry_count: number };
}
