import type { TopsiStatusResponse, TopologyOverview, DetectedIssue, ProjectAccess } from '@/lib/api';

export interface TopsiChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
  hasAudio?: boolean;
}

export interface TopsiPageData {
  status: TopsiStatusResponse | null;
  topology: TopologyOverview | null;
  issues: DetectedIssue[];
  projects: ProjectAccess[];
}
