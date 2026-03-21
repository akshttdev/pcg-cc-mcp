import type {
  CrmPipeline,
  CrmPipelineStage,
  CrmPipelineWithStages,
  CreateCrmPipeline,
  UpdateCrmPipeline,
  CreateCrmPipelineStage,
  UpdateCrmPipelineStage,
  KanbanBoardData,
  CrmDealRecord,
  CrmDealWithContact,
  CreateCrmDeal,
  UpdateCrmDeal,
  MoveDealRequest,
  PipelineType,
} from '@/types/crm';
import { makeRequest, handleApiResponse } from './client';

// =============================================================================
// CRM Contact Records
// =============================================================================

export interface CrmContactRecord {
  id: string;
  organization_id: string;
  project_id: string | null;
  client_id: string | null;
  first_name: string | null;
  last_name: string | null;
  full_name: string | null;
  email: string | null;
  phone: string | null;
  mobile: string | null;
  avatar_url: string | null;
  company_name: string | null;
  job_title: string | null;
  department: string | null;
  linkedin_url: string | null;
  twitter_handle: string | null;
  website: string | null;
  source: string | null;
  lifecycle_stage: string;
  lead_score: number;
  last_activity_at: string | null;
  last_contacted_at: string | null;
  last_replied_at: string | null;
  owner_user_id: string | null;
  assigned_agent_id: string | null;
  zoho_contact_id: string | null;
  gmail_contact_id: string | null;
  external_ids: string | null;
  tags: string | null;
  lists: string | null;
  custom_fields: string | null;
  address_line1: string | null;
  address_line2: string | null;
  city: string | null;
  state: string | null;
  postal_code: string | null;
  country: string | null;
  email_opt_in: number | null;
  sms_opt_in: number | null;
  do_not_contact: number | null;
  email_count: number;
  meeting_count: number;
  deal_count: number;
  total_revenue: number;
  created_at: string;
  updated_at: string;
}

export interface CreateCrmContactRequest {
  organization_id: string;
  client_id?: string;
  first_name?: string;
  last_name?: string;
  email?: string;
  phone?: string;
  mobile?: string;
  avatar_url?: string;
  company_name?: string;
  job_title?: string;
  department?: string;
  linkedin_url?: string;
  twitter_handle?: string;
  website?: string;
  source?: string;
  lifecycle_stage?: string;
  tags?: string[];
  custom_fields?: Record<string, unknown>;
  zoho_contact_id?: string;
  gmail_contact_id?: string;
}

export interface UpdateCrmContactRequest {
  first_name?: string;
  last_name?: string;
  email?: string;
  phone?: string;
  mobile?: string;
  avatar_url?: string;
  company_name?: string;
  job_title?: string;
  department?: string;
  linkedin_url?: string;
  twitter_handle?: string;
  website?: string;
  source?: string;
  lifecycle_stage?: string;
  lead_score?: number;
  owner_user_id?: string;
  assigned_agent_id?: string;
  tags?: string[];
  custom_fields?: Record<string, unknown>;
  address_line1?: string;
  address_line2?: string;
  city?: string;
  state?: string;
  postal_code?: string;
  country?: string;
  email_opt_in?: boolean;
  sms_opt_in?: boolean;
  do_not_contact?: boolean;
  zoho_contact_id?: string;
  gmail_contact_id?: string;
}

export interface CrmContactStats {
  total: number;
  by_stage: Array<{ stage: string; count: number }>;
  avg_lead_score: number;
  needs_follow_up: number;
}

// =============================================================================
// QuickBooks API
// =============================================================================

export interface QuickBooksAccountRecord {
  id: string;
  organization_id: string;
  realm_id: string;
  company_name?: string;
  environment: string;
  sync_enabled: number;
  sync_frequency_minutes: number;
  last_sync_at?: string;
  sync_invoices: number;
  sync_customers: number;
  sync_payments: number;
  sync_expenses: number;
  sync_time_tracking: number;
  status: string;
  last_error?: string;
  metadata?: string;
  connected_by?: string;
  created_at: string;
  updated_at: string;
}

export interface QBConnectionStatus {
  connected: boolean;
  account?: QuickBooksAccountRecord;
  needs_reauth: boolean;
}

export const quickbooksApi = {
  getStatus: async (organizationId: string): Promise<QBConnectionStatus> => {
    const response = await makeRequest(`/api/quickbooks/status?organization_id=${organizationId}`);
    return handleApiResponse<QBConnectionStatus>(response);
  },

  getConnectUrl: (organizationId: string): string => {
    return `/api/quickbooks/connect?organization_id=${organizationId}`;
  },

  disconnect: async (accountId: string): Promise<void> => {
    const response = await makeRequest(`/api/quickbooks/accounts/${accountId}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  refreshToken: async (accountId: string): Promise<void> => {
    const response = await makeRequest(`/api/quickbooks/accounts/${accountId}/refresh`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },

  triggerSync: async (accountId: string): Promise<void> => {
    const response = await makeRequest(`/api/quickbooks/accounts/${accountId}/sync`, {
      method: 'POST',
      body: JSON.stringify({}),
    });
    return handleApiResponse<void>(response);
  },
};

export const crmApi = {
  listContacts: async (
    organizationId: string,
    options?: { lifecycleStage?: string; limit?: number }
  ): Promise<CrmContactRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('organization_id', organizationId);
    if (options?.lifecycleStage) searchParams.set('lifecycle_stage', options.lifecycleStage);
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    const response = await makeRequest(`/api/crm/contacts?${searchParams.toString()}`);
    return handleApiResponse<CrmContactRecord[]>(response);
  },

  searchContacts: async (
    organizationId: string,
    query?: string,
    options?: {
      lifecycleStage?: string;
      companyName?: string;
      minLeadScore?: number;
      limit?: number;
      offset?: number;
    }
  ): Promise<CrmContactRecord[]> => {
    const searchParams = new URLSearchParams();
    searchParams.set('organization_id', organizationId);
    if (query) searchParams.set('query', query);
    if (options?.lifecycleStage) searchParams.set('lifecycle_stage', options.lifecycleStage);
    if (options?.companyName) searchParams.set('company_name', options.companyName);
    if (options?.minLeadScore) searchParams.set('min_lead_score', options.minLeadScore.toString());
    if (options?.limit) searchParams.set('limit', options.limit.toString());
    if (options?.offset) searchParams.set('offset', options.offset.toString());
    const response = await makeRequest(`/api/crm/contacts/search?${searchParams.toString()}`);
    return handleApiResponse<CrmContactRecord[]>(response);
  },

  getContact: async (id: string): Promise<CrmContactRecord> => {
    const response = await makeRequest(`/api/crm/contacts/${id}`);
    return handleApiResponse<CrmContactRecord>(response);
  },

  getContactByEmail: async (projectId: string, email: string): Promise<CrmContactRecord | null> => {
    const response = await makeRequest(`/api/crm/contacts/by-email/${projectId}/${encodeURIComponent(email)}`);
    return handleApiResponse<CrmContactRecord | null>(response);
  },

  createContact: async (data: CreateCrmContactRequest): Promise<CrmContactRecord> => {
    const response = await makeRequest('/api/crm/contacts', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmContactRecord>(response);
  },

  updateContact: async (id: string, data: UpdateCrmContactRequest): Promise<CrmContactRecord> => {
    const response = await makeRequest(`/api/crm/contacts/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmContactRecord>(response);
  },

  deleteContact: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}`, {
      method: 'DELETE',
    });
    await handleApiResponse<void>(response);
  },

  getContactStats: async (organizationId: string): Promise<CrmContactStats> => {
    const response = await makeRequest(`/api/crm/contacts/stats/${organizationId}`);
    return handleApiResponse<CrmContactStats>(response);
  },

  recordActivity: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/activity`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  recordContacted: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/contacted`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  recordReplied: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/replied`, {
      method: 'POST',
    });
    await handleApiResponse<void>(response);
  },

  updateLeadScore: async (id: string, scoreDelta: number): Promise<CrmContactRecord> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/lead-score`, {
      method: 'POST',
      body: JSON.stringify({ score_delta: scoreDelta }),
    });
    return handleApiResponse<CrmContactRecord>(response);
  },
};

// ============================================================================
// CRM Pipeline & Deal API
// ============================================================================

export type {
  CrmPipeline,
  CrmPipelineStage,
  CrmPipelineWithStages,
  CreateCrmPipeline,
  UpdateCrmPipeline,
  CreateCrmPipelineStage,
  UpdateCrmPipelineStage,
  KanbanBoardData,
  CrmDealRecord,
  CreateCrmDeal,
  UpdateCrmDeal,
  MoveDealRequest,
  PipelineType,
};

export const crmPipelinesApi = {
  /** List pipelines for an organization */
  listPipelines: async (
    organizationId: string,
    options?: { pipelineType?: PipelineType }
  ): Promise<CrmPipeline[]> => {
    const params = new URLSearchParams();
    params.set('organization_id', organizationId);
    if (options?.pipelineType) params.set('pipeline_type', options.pipelineType);
    const response = await makeRequest(`/api/crm/pipelines?${params.toString()}`);
    return handleApiResponse<CrmPipeline[]>(response);
  },

  /** List pipelines for an organization */
  listOrgPipelines: async (orgId: string): Promise<CrmPipeline[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/crm/pipelines`);
    return handleApiResponse<CrmPipeline[]>(response);
  },

  /** Get a single pipeline with its stages */
  getPipeline: async (id: string): Promise<CrmPipelineWithStages> => {
    const response = await makeRequest(`/api/crm/pipelines/${id}`);
    return handleApiResponse<CrmPipelineWithStages>(response);
  },

  /** Get a pipeline with stages under an org (by pipeline id) */
  getOrgPipeline: async (orgId: string, pipelineId: string): Promise<CrmPipelineWithStages> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/crm/pipelines/${pipelineId}`
    );
    return handleApiResponse<CrmPipelineWithStages>(response);
  },

  createPipeline: async (data: CreateCrmPipeline): Promise<CrmPipeline> => {
    const response = await makeRequest('/api/crm/pipelines', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmPipeline>(response);
  },

  updatePipeline: async (id: string, data: UpdateCrmPipeline): Promise<CrmPipeline> => {
    const response = await makeRequest(`/api/crm/pipelines/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmPipeline>(response);
  },

  deletePipeline: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/pipelines/${id}`, { method: 'DELETE' });
    await handleApiResponse<void>(response);
  },

  listStages: async (pipelineId: string): Promise<CrmPipelineStage[]> => {
    const response = await makeRequest(`/api/crm/pipelines/${pipelineId}/stages`);
    return handleApiResponse<CrmPipelineStage[]>(response);
  },

  createStage: async (
    pipelineId: string,
    data: Omit<CreateCrmPipelineStage, 'pipeline_id'>
  ): Promise<CrmPipelineStage> => {
    const response = await makeRequest(`/api/crm/pipelines/${pipelineId}/stages`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmPipelineStage>(response);
  },

  updateStage: async (
    pipelineId: string,
    stageId: string,
    data: UpdateCrmPipelineStage
  ): Promise<CrmPipelineStage> => {
    const response = await makeRequest(
      `/api/crm/pipelines/${pipelineId}/stages/${stageId}`,
      { method: 'PATCH', body: JSON.stringify(data) }
    );
    return handleApiResponse<CrmPipelineStage>(response);
  },

  deleteStage: async (pipelineId: string, stageId: string): Promise<void> => {
    const response = await makeRequest(
      `/api/crm/pipelines/${pipelineId}/stages/${stageId}`,
      { method: 'DELETE' }
    );
    await handleApiResponse<void>(response);
  },

  reorderStages: async (
    pipelineId: string,
    stageIds: string[]
  ): Promise<CrmPipelineStage[]> => {
    const response = await makeRequest(`/api/crm/pipelines/${pipelineId}/stages/reorder`, {
      method: 'POST',
      body: JSON.stringify({ stage_ids: stageIds }),
    });
    return handleApiResponse<CrmPipelineStage[]>(response);
  },
};

/** Rich deal detail returned by GET /crm/deals/:id/rich */
export interface DealTask {
  id: string;
  title: string;
  description: string | null;
  status: string;
  created_at: string;
}

export interface DealKnowledgeSource {
  source_type: string;
  source_title: string;
  source_summary: string | null;
  coverage_score: number;
  last_refreshed_at: string | null;
}

export interface CrmDealRich extends CrmDealWithContact {
  company_id?: string;
  company_intelligence_summary?: string;
  company_intelligence_status: string | undefined;
  company_intelligence_confidence: number | null;
  company_intelligence_last_run_at: string | null;
  tasks: DealTask[];
  knowledge_sources: DealKnowledgeSource[];
}

export const crmDealsApi = {
  listDeals: async (options: {
    organization_id?: string;
    pipeline_id?: string;
    stage_id?: string;
    contact_id?: string;
  }): Promise<CrmDealRecord[]> => {
    const params = new URLSearchParams();
    if (options.organization_id) params.set('organization_id', options.organization_id);
    if (options.pipeline_id) params.set('pipeline_id', options.pipeline_id);
    if (options.stage_id) params.set('stage_id', options.stage_id);
    if (options.contact_id) params.set('contact_id', options.contact_id);
    const response = await makeRequest(`/api/crm/deals?${params}`);
    return handleApiResponse<CrmDealRecord[]>(response);
  },

  getDeal: async (id: string): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${id}`);
    return handleApiResponse<CrmDealRecord>(response);
  },

  createDeal: async (data: CreateCrmDeal): Promise<CrmDealRecord> => {
    const response = await makeRequest('/api/crm/deals', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmDealRecord>(response);
  },

  updateDeal: async (id: string, data: UpdateCrmDeal): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmDealRecord>(response);
  },

  moveDeal: async (dealId: string, data: MoveDealRequest): Promise<import('@/types/crm').TransitionResult> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/stage`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<import('@/types/crm').TransitionResult>(response);
  },

  advanceDeal: async (dealId: string): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/advance`, { method: 'POST' });
    return handleApiResponse<CrmDealRecord>(response);
  },

  cancelDealAgent: async (dealId: string): Promise<{ cancelled: boolean; message: string }> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/cancel-agent`, { method: 'POST' });
    return handleApiResponse<{ cancelled: boolean; message: string }>(response);
  },

  approveDealAgent: async (dealId: string): Promise<{ approved: boolean; message: string }> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/approve-agent`, { method: 'POST' });
    return handleApiResponse<{ approved: boolean; message: string }>(response);
  },

  getDealRich: async (dealId: string): Promise<CrmDealRich> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/rich`);
    return handleApiResponse<CrmDealRich>(response);
  },

  deleteDeal: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/deals/${id}`, { method: 'DELETE' });
    await handleApiResponse<void>(response);
  },

  getMetrics: async (organizationId: string, pipelineId?: string): Promise<PipelineMetricsRecord> => {
    const params = new URLSearchParams({ organization_id: organizationId });
    if (pipelineId) params.set('pipeline_id', pipelineId);
    const response = await makeRequest(`/api/crm/deals/metrics?${params}`);
    return handleApiResponse<PipelineMetricsRecord>(response);
  },

  /** Get Kanban board data for a pipeline (deals grouped by stage) */
  getKanbanData: async (pipelineId: string): Promise<KanbanBoardData> => {
    const response = await makeRequest(`/api/crm/deals/kanban/${pipelineId}`);
    return handleApiResponse<KanbanBoardData>(response);
  },

  /** Get Kanban board for an org-scoped pipeline */
  getOrgKanbanData: async (orgId: string, pipelineId: string): Promise<KanbanBoardData> => {
    const response = await makeRequest(
      `/api/organizations/${orgId}/crm/pipelines/${pipelineId}/kanban`
    );
    return handleApiResponse<KanbanBoardData>(response);
  },

  /** List all deals for an organization */
  listOrgDeals: async (orgId: string): Promise<CrmDealRecord[]> => {
    const response = await makeRequest(`/api/organizations/${orgId}/crm/deals`);
    return handleApiResponse<CrmDealRecord[]>(response);
  },

  /** Generate proposal text via Cash agent */
  generateProposal: async (dealId: string): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/generate-proposal`, { method: 'POST' });
    return handleApiResponse<CrmDealRecord>(response);
  },

  /** Approve the proposal, advancing proposal_status → 'approved' */
  approveProposal: async (dealId: string): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/approve-proposal`, { method: 'POST' });
    return handleApiResponse<CrmDealRecord>(response);
  },

  /** Generate deck script via Lux agent */
  generateDeck: async (dealId: string): Promise<CrmDealRecord> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/generate-deck`, { method: 'POST' });
    return handleApiResponse<CrmDealRecord>(response);
  },

  /** Send invoice from Present stage */
  sendInvoice: async (dealId: string, opts?: { notes?: string; due_days?: number }): Promise<import('@/types/crm').SendInvoiceResult> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/send-invoice`, {
      method: 'POST',
      body: JSON.stringify(opts ?? {}),
    });
    return handleApiResponse<import('@/types/crm').SendInvoiceResult>(response);
  },

  /** Mark deal as won — triggers full automation chain */
  markWon: async (dealId: string, winReason?: string): Promise<import('@/types/crm').MarkWonResult> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/mark-won`, {
      method: 'POST',
      body: JSON.stringify({ win_reason: winReason }),
    });
    return handleApiResponse<import('@/types/crm').MarkWonResult>(response);
  },

  /** List transcripts linked to a deal */
  listTranscripts: async (dealId: string): Promise<import('@/types/crm').DealTranscript[]> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/transcripts`);
    return handleApiResponse<import('@/types/crm').DealTranscript[]>(response);
  },

  /** Link a transcript to a deal */
  linkTranscript: async (dealId: string, data: {
    intake_item_id?: string;
    call_log_id?: string;
    transcript_text?: string;
    summary?: string;
    matched_by?: string;
  }): Promise<import('@/types/crm').DealTranscript> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/transcripts`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<import('@/types/crm').DealTranscript>(response);
  },
};

// CRM Activities API
export interface CrmActivityRecord {
  id: string;
  organization_id: string;
  project_id: string | null;
  client_id: string | null;
  crm_contact_id?: string;
  crm_deal_id?: string;
  activity_type: string;
  subject?: string;
  description?: string;
  outcome?: string;
  email_message_id?: string;
  social_mention_id?: string;
  task_id?: string;
  performed_by_user?: string;
  performed_by_agent_id?: string;
  metadata?: string;
  duration_minutes?: number;
  activity_at: string;
  created_at: string;
}

export interface PipelineMetricsRecord {
  pipeline_id: string;
  total_deals: number;
  total_value: number;
  weighted_value: number;
  avg_deal_size: number;
  win_rate: number;
  deals_by_stage: Array<{
    stage_id: string;
    stage_name: string;
    count: number;
    total_value: number;
  }>;
  monthly_summary: Array<{
    month: string;
    new_deals: number;
    won_deals: number;
    lost_deals: number;
    total_value: number;
  }>;
}

export const crmActivitiesApi = {
  listActivities: async (options: {
    organization_id?: string;
    contact_id?: string;
    deal_id?: string;
    limit?: number;
  }): Promise<CrmActivityRecord[]> => {
    const params = new URLSearchParams();
    if (options.organization_id) params.set('organization_id', options.organization_id);
    if (options.contact_id) params.set('contact_id', options.contact_id);
    if (options.deal_id) params.set('deal_id', options.deal_id);
    if (options.limit) params.set('limit', options.limit.toString());
    const response = await makeRequest(`/api/crm/activities?${params}`);
    return handleApiResponse<CrmActivityRecord[]>(response);
  },

  createActivity: async (data: {
    organization_id: string;
    crm_contact_id?: string;
    crm_deal_id?: string;
    activity_type: string;
    subject?: string;
    description?: string;
    outcome?: string;
    performed_by_user?: string;
    duration_minutes?: number;
    metadata?: Record<string, unknown>;
  }): Promise<CrmActivityRecord> => {
    const response = await makeRequest('/api/crm/activities', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<CrmActivityRecord>(response);
  },

  deleteActivity: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/activities/${id}`, { method: 'DELETE' });
    await handleApiResponse<void>(response);
  },
};
