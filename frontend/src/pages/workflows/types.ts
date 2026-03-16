// ─── Shared Types for Workflows Page ──────────────────────────────────────────

export interface AutomationDefinition {
  id: string;
  name: string;
  description: string;
  trigger: string;
  action: string;
  schedule: string;
}

export interface ConferenceWorkflow {
  id: string;
  conferenceName: string;
  status: string;
  startDate: string;
  endDate: string;
  location: string | null;
  createdAt: string;
}

export interface CinematicBrief {
  id: string;
  project_id: string;
  title: string;
  status: string;
  created_at: string;
}

export type GlobalFilterType = 'all' | 'valid' | 'duplicates' | 'approved' | 'rejected' | 'issues' | 'error';
