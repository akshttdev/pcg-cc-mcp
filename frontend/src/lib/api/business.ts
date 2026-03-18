import { makeRequest, handleApiResponse } from './client';
import type { PersonRecord, CompanyContactMethod, InvoiceRecord, OrgBrandProfile } from './communication';

// ── Proposals ─────────────────────────────────────────────────────────────────

export type ProposalStatus =
  | 'drafted'
  | 'pending_approval'
  | 'approved'
  | 'meeting_scheduled'
  | 'sent'
  | 'seen'
  | 'verbal'
  | 'contract_signed'
  | 'declined'
  | 'deferred';

export type DealType = 'one-off' | 'retainer' | 'hybrid';

export interface ProposalRecord {
  id: string;
  lead_id?: string;
  organization_id?: string;
  owner_id?: string;
  project_id?: string;
  company_id?: string;
  contact_ids: string;
  status: ProposalStatus;
  title: string;
  description: string;
  quote_amount_vibe: number;
  deal_type: DealType;
  sent_at?: string;
  seen_at?: string;
  verbal_at?: string;
  signed_at?: string;
  declined_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateProposalInput {
  title: string;
  lead_id?: string;
  organization_id?: string;
  owner_id?: string;
  project_id?: string;
  company_id?: string;
  description?: string;
  quote_amount_vibe?: number;
  deal_type?: DealType;
}

export interface UpdateProposalInput {
  title?: string;
  description?: string;
  quote_amount_vibe?: number;
  deal_type?: DealType;
  lead_id?: string;
  project_id?: string;
  owner_id?: string;
}

export const proposalsApi = {
  list: async (params?: {
    status?: string;
    lead_id?: string;
    project_id?: string;
    owner_id?: string;
    organization_id?: string;
    limit?: number;
  }): Promise<ProposalRecord[]> => {
    const qs = params ? '?' + new URLSearchParams(
      Object.entries(params)
        .filter(([, v]) => v != null)
        .map(([k, v]) => [k, String(v)])
    ) : '';
    const response = await makeRequest(`/api/proposals${qs}`);
    return handleApiResponse<ProposalRecord[]>(response);
  },

  get: async (id: string): Promise<ProposalRecord> => {
    const response = await makeRequest(`/api/proposals/${id}`);
    return handleApiResponse<ProposalRecord>(response);
  },

  create: async (data: CreateProposalInput): Promise<ProposalRecord> => {
    const response = await makeRequest('/api/proposals', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProposalRecord>(response);
  },

  update: async (id: string, data: UpdateProposalInput): Promise<ProposalRecord> => {
    const response = await makeRequest(`/api/proposals/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<ProposalRecord>(response);
  },

  moveStatus: async (id: string, status: ProposalStatus): Promise<ProposalRecord> => {
    const response = await makeRequest(`/api/proposals/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<ProposalRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/proposals/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  scheduleMeeting: async (
    id: string,
    data: {
      scheduled_at: string;
      duration_min?: number;
      location?: string;
      agenda?: string;
      channel?: string;
      invitees: Array<{ person_id: string; channel?: string; channel_address?: string }>;
    }
  ): Promise<{ meeting: ScheduledMeetingRecord; dispatched: InviteDispatchResult[] }> => {
    const response = await makeRequest(`/api/proposals/${id}/schedule-meeting`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse(response);
  },

  listScheduledMeetings: async (id: string): Promise<ScheduledMeetingRecord[]> => {
    const response = await makeRequest(`/api/proposals/${id}/scheduled-meetings`);
    return handleApiResponse(response);
  },
};

// ── Deliverables ──────────────────────────────────────────────────────────────

export type DeliverableType = 'video' | 'audio' | 'graphic' | 'copy' | 'code' | 'document' | 'other';
export type DeliverableStatus =
  | 'working'
  | 'internal_review'
  | 'client_review'
  | 'revision'
  | 'client_revision'
  | 'done';

export interface DeliverableRecord {
  id: string;
  project_id: string;
  proposal_id?: string;
  deliverable_type: DeliverableType;
  title: string;
  description: string;
  status: DeliverableStatus;
  revision_rounds_allowed: number;
  revision_rounds_used: number;
  working_file_url?: string;
  final_link?: string;
  due_date?: string;
  delivered_at?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateDeliverableInput {
  project_id: string;
  proposal_id?: string;
  deliverable_type?: DeliverableType;
  title: string;
  description?: string;
  revision_rounds_allowed?: number;
  working_file_url?: string;
  due_date?: string;
}

export const deliverablesApi = {
  listForProject: async (projectId: string): Promise<DeliverableRecord[]> => {
    const response = await makeRequest(`/api/projects/${projectId}/deliverables`);
    return handleApiResponse<DeliverableRecord[]>(response);
  },

  create: async (data: CreateDeliverableInput): Promise<DeliverableRecord> => {
    const response = await makeRequest(`/api/projects/${data.project_id}/deliverables`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<DeliverableRecord>(response);
  },

  update: async (id: string, data: Partial<CreateDeliverableInput> & { final_link?: string }): Promise<DeliverableRecord> => {
    const response = await makeRequest(`/api/deliverables/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<DeliverableRecord>(response);
  },

  moveStatus: async (id: string, status: DeliverableStatus): Promise<DeliverableRecord> => {
    const response = await makeRequest(`/api/deliverables/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<DeliverableRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/deliverables/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ── Command Center ────────────────────────────────────────────────────────────

export interface CommandCenterSnapshot {
  overdue_tasks: Array<{
    id: string; title: string; project_name: string; due_date: string; assignee_name?: string;
  }>;
  deliverables_due_this_week: Array<{
    id: string; title: string; deliverable_type: string; project_name: string;
    status: string; due_date: string;
  }>;
  waiting_on_client_projects: Array<{
    id: string; name: string; client_name?: string; updated_at: string;
  }>;
  follow_up_required: Array<{
    id: string; full_name: string; email?: string;
    follow_up_attempts: number; lifecycle_stage: string;
  }>;
  proposals_awaiting_approval: Array<{
    id: string; title: string; lead_name?: string;
    quote_amount_vibe: number; created_at: string;
  }>;
  closed_unpaid_projects: Array<{
    id: string; name: string; client_name?: string; updated_at: string;
  }>;
}

export const commandCenterApi = {
  get: async (orgId?: string): Promise<CommandCenterSnapshot> => {
    const qs = orgId ? `?org_id=${orgId}` : '';
    const response = await makeRequest(`/api/command-center${qs}`);
    return handleApiResponse<CommandCenterSnapshot>(response);
  },
};

// ── Invoices ──────────────────────────────────────────────────────────────────

export interface CreateInvoiceInput {
  person_id?: string;
  organization_id?: string;
  project_id?: string;
  invoice_type?: 'ar' | 'ap';
  title?: string;
  description?: string;
  amount_usd?: number;
  amount_vibe?: number;
  currency?: string;
  issue_date?: string;
  due_date?: string;
  notes?: string;
}

export const invoicesApi = {
  list: async (params?: { invoice_type?: string; status?: string; person_id?: string; project_id?: string; limit?: number }): Promise<InvoiceRecord[]> => {
    const qs = params ? '?' + new URLSearchParams(Object.fromEntries(Object.entries(params).filter(([, v]) => v != null).map(([k, v]) => [k, String(v)]))).toString() : '';
    const response = await makeRequest(`/api/invoices${qs}`);
    return handleApiResponse<InvoiceRecord[]>(response);
  },

  create: async (data: CreateInvoiceInput): Promise<InvoiceRecord> => {
    const response = await makeRequest('/api/invoices', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<InvoiceRecord>(response);
  },

  get: async (id: string): Promise<InvoiceRecord> => {
    const response = await makeRequest(`/api/invoices/${id}`);
    return handleApiResponse<InvoiceRecord>(response);
  },

  update: async (id: string, data: Partial<CreateInvoiceInput>): Promise<InvoiceRecord> => {
    const response = await makeRequest(`/api/invoices/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<InvoiceRecord>(response);
  },

  moveStatus: async (id: string, status: string): Promise<InvoiceRecord> => {
    const response = await makeRequest(`/api/invoices/${id}/status`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ status }),
    });
    return handleApiResponse<InvoiceRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/invoices/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ============================================================
// Companies API (knowledge-graph company entities)
// ============================================================

export interface CompanyRecord {
  id: string;
  name: string;
  slug?: string | null;
  website?: string | null;
  industry?: string | null;
  description?: string | null;
  logo_url?: string | null;
  cover_image_url?: string | null;
  headquarters?: string | null;
  address?: string | null;
  city?: string | null;
  country?: string | null;
  phone?: string | null;
  email?: string | null;
  whatsapp?: string | null;
  instagram_handle?: string | null;
  linkedin_url?: string | null;
  twitter_handle?: string | null;
  facebook_url?: string | null;
  founded_year?: number | null;
  employee_count?: string | null;
  tags?: string | null;
  business_hours?: string | null;
  notes?: string | null;
  gmb_rating?: number | null;
  gmb_review_count?: number | null;
  gmb_place_id?: string | null;
  intelligence_summary?: string | null;
  intelligence_raw?: string | null;
  intelligence_status: string;
  intelligence_last_run_at?: string | null;
  intelligence_confidence?: number | null;
  intelligence_agent?: string | null;
  organization_id?: string | null;
  created_by_org_id?: string | null;
  created_at: string;
  updated_at: string;
}

export const companiesApi = {
  list: async (params?: { limit?: number; has_platform_org?: boolean; created_by_org_id?: string }): Promise<CompanyRecord[]> => {
    const qs = new URLSearchParams();
    if (params?.limit != null) qs.set('limit', String(params.limit));
    if (params?.has_platform_org != null) qs.set('has_platform_org', String(params.has_platform_org));
    if (params?.created_by_org_id != null) qs.set('created_by_org_id', params.created_by_org_id);
    const response = await makeRequest(`/api/companies?${qs.toString()}`);
    return handleApiResponse<CompanyRecord[]>(response);
  },

  get: async (id: string): Promise<CompanyRecord> => {
    const response = await makeRequest(`/api/companies/${id}`);
    return handleApiResponse<CompanyRecord>(response);
  },

  create: async (data: { name: string; website?: string; industry?: string; description?: string; headquarters?: string; created_by_org_id?: string }): Promise<CompanyRecord> => {
    const response = await makeRequest('/api/companies', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CompanyRecord>(response);
  },

  update: async (id: string, data: Partial<CompanyRecord>): Promise<CompanyRecord> => {
    const response = await makeRequest(`/api/companies/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CompanyRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/companies/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  listProposals: async (id: string): Promise<ProposalRecord[]> => {
    const response = await makeRequest(`/api/companies/${id}/proposals`);
    return handleApiResponse<ProposalRecord[]>(response);
  },

  listPersons: async (id: string): Promise<PersonRecord[]> => {
    const response = await makeRequest(`/api/companies/${id}/persons`);
    return handleApiResponse<PersonRecord[]>(response);
  },

  getIntelligenceStatus: async (id: string): Promise<{ status: string; summary?: string; confidence: number; agent?: string; last_run_at?: string }> => {
    const company = await companiesApi.get(id);
    return {
      status: company.intelligence_status,
      summary: company.intelligence_summary ?? undefined,
      confidence: company.intelligence_confidence ?? 0,
      agent: company.intelligence_agent ?? undefined,
      last_run_at: company.intelligence_last_run_at ?? undefined,
    };
  },

  research: async (id: string): Promise<{ status: string; message: string }> => {
    const response = await makeRequest(`/api/companies/${id}/research`, {
      method: 'POST',
      body: JSON.stringify({}),
    });
    return handleApiResponse<{ status: string; message: string }>(response);
  },

  exportAnalysis: async (id: string, companyName?: string): Promise<void> => {
    const response = await makeRequest(`/api/companies/${id}/export-analysis`);
    if (!response.ok) throw new Error('Export failed');
    const blob = await response.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = companyName ? `PCG_Analysis_${companyName.replace(/\s+/g, '_')}.md` : 'PCG_Analysis.md';
    a.click();
    URL.revokeObjectURL(url);
  },

  listContactMethods: async (id: string): Promise<CompanyContactMethod[]> => {
    const response = await makeRequest(`/api/companies/${id}/contact-methods`);
    return handleApiResponse<CompanyContactMethod[]>(response);
  },
  addContactMethod: async (id: string, data: { method_type: string; label?: string; value: string; is_primary?: boolean }): Promise<CompanyContactMethod> => {
    const response = await makeRequest(`/api/companies/${id}/contact-methods`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CompanyContactMethod>(response);
  },
  removeContactMethod: async (id: string, method_id: string): Promise<void> => {
    const response = await makeRequest(`/api/companies/${id}/contact-methods/${method_id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  getBrandProfile: async (id: string): Promise<OrgBrandProfile | null> => {
    const response = await makeRequest(`/api/companies/${id}/brand-profile`);
    return handleApiResponse<OrgBrandProfile | null>(response);
  },

  upsertBrandProfile: async (id: string, data: Partial<OrgBrandProfile>): Promise<OrgBrandProfile> => {
    const response = await makeRequest(`/api/companies/${id}/brand-profile`, {
      method: 'PUT',
      body: JSON.stringify(data),
    });
    return handleApiResponse<OrgBrandProfile>(response);
  },
};

// ── Scheduled Meeting types ───────────────────────────────────────────────────

export interface ScheduledMeetingInviteeRecord {
  id: string;
  scheduled_meeting_id: string;
  person_id: string;
  channel: string;
  channel_address: string;
  status: string;
  sent_at?: string;
  created_at: string;
}

export interface ScheduledMeetingRecord {
  id: string;
  proposal_id: string;
  scheduled_at: string;
  duration_min: number;
  location?: string;
  agenda?: string;
  channel: string;
  invite_status: string;
  invite_sent_at?: string;
  notes?: string;
  invitees: ScheduledMeetingInviteeRecord[];
  created_at: string;
  updated_at: string;
}

export interface InviteDispatchResult {
  person_id: string;
  channel: string;
  status: string;
  message: string;
}

export const meetingsApi = {
  publish: async (
    sessionId: string,
    data: {
      project_id: string;
      company_id?: string;
      proposal_id?: string;
      attendee_person_ids?: string[];
      source_title?: string;
    }
  ): Promise<{ session_id: string; knowledge_source_id: string; message: string }> => {
    const response = await makeRequest(`/api/topsi/meeting/${sessionId}/publish`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse(response);
  },
};

// ── Call Intake ──────────────────────────────────────────────────────────────

export const callIntakeApi = {
  list: async (): Promise<unknown[]> => {
    const response = await makeRequest('/api/call-intake');
    return handleApiResponse<unknown[]>(response);
  },

  submitEmail: async (data: {
    raw_content: string;
    subject?: string | null;
    from_name?: string | null;
    from_email?: string | null;
    source_type?: string;
    auto_process?: boolean;
  }): Promise<unknown> => {
    const response = await makeRequest('/api/call-intake/email', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<unknown>(response);
  },

  process: async (id: string): Promise<unknown> => {
    const response = await makeRequest(`/api/call-intake/${id}/process`, {
      method: 'POST',
    });
    return handleApiResponse<unknown>(response);
  },
};

// ── Discord Voice ─────────────────────────────────────────────────────────────

export interface DiscordSessionSummary {
  meeting_session_id: string;
  guild_id: string;
  channel_id: string;
  channel_name: string;
  project_id: string;
  agent: string;
  started_at: string;
  elapsed_seconds: number;
  segment_count: number;
  participant_count: number;
}

export interface DiscordSegment {
  id: string;
  segment_index: number;
  speaker_label?: string;
  text: string;
  confidence?: number;
  start_time_ms: number;
  end_time_ms: number;
  is_topsi_addressed: boolean;
  metadata?: string;
  created_at: string;
}

export interface DiscordTranscript {
  meeting_session_id: string;
  segment_count: number;
  segments: DiscordSegment[];
}

export const discordApi = {
  activeSessions: async (): Promise<DiscordSessionSummary[]> => {
    const response = await makeRequest('/api/discord/sessions');
    return handleApiResponse<DiscordSessionSummary[]>(response);
  },

  archivedSessions: async (params?: { limit?: number; offset?: number }): Promise<any[]> => {
    const qs = params ? '?' + new URLSearchParams(Object.fromEntries(Object.entries(params).filter(([, v]) => v != null).map(([k, v]) => [k, String(v)]))).toString() : '';
    const response = await makeRequest(`/api/discord/archive${qs}`);
    return handleApiResponse<any[]>(response);
  },

  getTranscript: async (sessionId: string): Promise<DiscordTranscript> => {
    const response = await makeRequest(`/api/discord/sessions/${sessionId}/transcript`);
    return handleApiResponse<DiscordTranscript>(response);
  },

  leave: async (guildId: string): Promise<{ success: boolean; meeting_session_id: string }> => {
    const response = await makeRequest('/api/discord/leave', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ guild_id: guildId }),
    });
    return handleApiResponse(response);
  },
};
