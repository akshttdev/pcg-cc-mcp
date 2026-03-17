import { makeRequest, handleApiResponse } from './client';

// ── Types ───────────────────────────────────────────────────────────────────

export interface TopiClipSession {
  id: string;
  projectId: string;
  title: string;
  dayNumber: number;
  triggerType: 'daily' | 'event' | 'manual';
  primaryTheme?: string;
  emotionalArc?: string;
  narrativeSummary?: string;
  artisticPrompt?: string;
  status: 'pending' | 'analyzing' | 'interpreting' | 'rendering' | 'delivered' | 'failed' | 'cancelled';
  outputAssetIds?: string[];
  eventsAnalyzed: number;
  significanceScore?: number;
  createdAt: string;
  deliveredAt?: string;
}

export interface TopiClipGalleryResponse {
  sessions: TopiClipSession[];
  schedule?: TopiClipDailySchedule;
  currentStreak: number;
  longestStreak: number;
  totalClips: number;
}

export interface TopiClipDailySchedule {
  id: string;
  projectId: string;
  scheduledTime: string;
  timezone?: string;
  isEnabled: boolean;
  currentStreak: number;
  longestStreak: number;
  totalClipsGenerated: number;
  lastGenerationDate?: string;
}

export interface TopiClipTimelineEntry {
  session: TopiClipSession;
  events: TopiClipCapturedEvent[];
  assetUrls: string[];
}

export interface TopiClipCapturedEvent {
  id: string;
  sessionId: string;
  eventType: string;
  narrativeRole?: string;
  significanceScore?: number;
  assignedSymbol?: string;
  symbolPrompt?: string;
  occurredAt: string;
}

export interface TopiClipSymbol {
  id: string;
  eventPattern: string;
  symbolName: string;
  symbolDescription?: string;
  promptTemplate: string;
  themeAffinity?: string;
  motionType?: string;
}

// ── API Module ──────────────────────────────────────────────────────────────

export const topiclipsApi = {
  getGallery: (projectId: string): Promise<{ data: TopiClipGalleryResponse }> =>
    makeRequest(`/api/topiclips/gallery?projectId=${projectId}`).then(
      handleApiResponse<{ data: TopiClipGalleryResponse }>,
    ),

  getSymbols: (): Promise<{ data: TopiClipSymbol[] }> =>
    makeRequest('/api/topiclips/symbols').then(handleApiResponse<{ data: TopiClipSymbol[] }>),

  getSessionTimeline: (sessionId: string): Promise<{ data: TopiClipTimelineEntry }> =>
    makeRequest(`/api/topiclips/sessions/${sessionId}/timeline`).then(
      handleApiResponse<{ data: TopiClipTimelineEntry }>,
    ),

  createSession: (projectId: string, triggerType: string): Promise<{ data: TopiClipSession }> =>
    makeRequest('/api/topiclips/sessions', {
      method: 'POST',
      body: JSON.stringify({ projectId, triggerType }),
    }).then(handleApiResponse<{ data: TopiClipSession }>),

  generateSession: (sessionId: string): Promise<unknown> =>
    makeRequest(`/api/topiclips/sessions/${sessionId}/generate`, {
      method: 'POST',
    }).then(handleApiResponse),

  generateDaily: (projectId: string): Promise<unknown> =>
    makeRequest(`/api/topiclips/daily/${projectId}/generate`, {
      method: 'POST',
    }).then(handleApiResponse),

  saveSchedule: (projectId: string, scheduledTime: string, timezone: string): Promise<unknown> =>
    makeRequest('/api/topiclips/daily', {
      method: 'POST',
      body: JSON.stringify({ projectId, scheduledTime, timezone }),
    }).then(handleApiResponse),
};
