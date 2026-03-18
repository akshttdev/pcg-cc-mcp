// ── Meeting Mode Types & Helpers ────────────────────────────────────────────

export interface TranscriptEntry {
  text: string;
  speakerLabel?: string;
  isTopsiAddressed: boolean;
  topsiResponse?: string;
  timestamp: number;
  segmentIndex: number;
}

export interface MeetingNotes {
  summary: string;
  topics: string[];
  decisions: string[];
  actionItems: { description: string; assignee?: string; deadline?: string; priority?: string }[];
  openQuestions: string[];
  participants: string[];
}

export interface ActiveMeeting {
  id: string;
  title: string;
  projectId: string;
  status: string;
  startedBy: string;
  startedAt: string;
  participantCount?: number;
}

export interface MeetingModeProps {
  projectId?: string;      // pre-select a project (e.g. from project detail page)
  onClose?: () => void;
  className?: string;
}

export type MeetingState = 'idle' | 'active' | 'paused' | 'ended';
export type MeetingRole = 'host' | 'observer'; // host started the meeting; observer joined

// ── Helpers ───────────────────────────────────────────────────────────────────

export function formatDuration(seconds: number) {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}`;
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function isUrl(text: string): boolean {
  try {
    const url = new URL(text);
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch {
    return false;
  }
}

export function getAuthHeaders(): Record<string, string> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  const token = localStorage.getItem('session_id') || sessionStorage.getItem('session_id') ||
    localStorage.getItem('auth_token') || sessionStorage.getItem('auth_token');
  if (token) headers['Authorization'] = `Bearer ${token}`;
  return headers;
}
