import { makeRequest, handleApiResponse } from './client';

// ============================================================================
// EMAIL MESSAGES
// ============================================================================

export interface EmailMessageRecord {
  id: string;
  email_account_id: string;
  project_id: string;
  provider_message_id: string;
  thread_id: string | null;
  from_address: string;
  from_name: string | null;
  to_addresses: string;
  subject: string | null;
  body_text: string | null;
  body_html: string | null;
  snippet: string | null;
  has_attachments: number;
  is_read: number;
  is_starred: number;
  is_draft: number;
  is_sent: number;
  is_archived: number;
  is_trash: number;
  sentiment: string | null;
  priority: string | null;
  needs_response: number;
  received_at: string;
  created_at: string;
  updated_at: string;
}

export interface EmailInboxStats {
  total: number;
  unread: number;
  starred: number;
  needs_response: number;
  by_account: Array<{ account_id: string; email_address: string; total: number; unread: number }>;
}

export const emailMessagesApi = {
  listMessages: async (params: {
    project_id: string;
    email_account_id?: string;
    is_read?: boolean;
    is_starred?: boolean;
    needs_response?: boolean;
    limit?: number;
  }): Promise<EmailMessageRecord[]> => {
    const qs = new URLSearchParams({ project_id: params.project_id });
    if (params.email_account_id) qs.set('email_account_id', params.email_account_id);
    if (params.is_read !== undefined) qs.set('is_read', String(params.is_read));
    if (params.is_starred !== undefined) qs.set('is_starred', String(params.is_starred));
    if (params.needs_response !== undefined) qs.set('needs_response', String(params.needs_response));
    if (params.limit) qs.set('limit', String(params.limit));
    const response = await makeRequest(`/api/email/messages?${qs}`);
    return handleApiResponse<EmailMessageRecord[]>(response);
  },
  getInboxStats: async (projectId: string): Promise<EmailInboxStats> => {
    const response = await makeRequest(`/api/email/messages/stats/${projectId}`);
    return handleApiResponse<EmailInboxStats>(response);
  },
  markAsRead: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/read`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
  toggleStar: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/star`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
  moveToTrash: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/trash`, { method: 'POST' });
    return handleApiResponse<void>(response);
  },
};

// ============================================================
// Universal Persons API
// ============================================================

export interface PersonRecord {
  id: string;
  full_name: string;
  email?: string;
  phone?: string;
  avatar_url?: string;
  person_type: string;
  financial_role: string;
  client_profile?: string;
  business_stage?: string;
  lifecycle_stage: string;
  lead_score: number;
  company_name?: string;
  job_title?: string;
  website?: string;
  user_id?: string;
  crm_contact_id?: string;
  organization_id?: string;
  intelligence_summary?: string;
  intelligence_raw?: string;
  intelligence_last_run_at?: string;
  intelligence_confidence: number;
  intelligence_status?: 'idle' | 'queued' | 'running' | 'done' | 'failed';
  intelligence_agent?: string;
  research_pass_count?: number;
  research_depth?: 'shallow' | 'moderate' | 'deep';
  notes?: string;
  tags: string;
  custom_fields: string;
  /** How this person first engaged: 'email'|'instagram'|'whatsapp'|'linkedin'|'twitter'|'sms'|'phone'|'in_person' */
  onboarding_channel?: string;
  /** Preferred outbound contact channel */
  preferred_contact?: string;
  created_at: string;
  updated_at: string;
}

export interface PersonSocialProfile {
  id: string;
  person_id: string;
  platform: string;
  handle?: string;
  profile_url?: string;
  follower_count?: number;
  following_count?: number;
  bio?: string;
  verified: number;
  last_synced_at?: string;
  created_at: string;
  updated_at: string;
}

export interface PersonCompanyRole {
  id: string;
  person_id: string;
  company_id: string;
  role: string;
  title?: string;
  is_primary: number;
  start_date?: string;
  end_date?: string;
  notes?: string;
  created_at: string;
  company_name?: string;
  company_slug?: string;
}

export interface PersonOrgContact {
  id: string;
  person_id: string;
  organization_id: string;
  context: string;
  notes?: string;
  added_at: string;
  org_name?: string;
}

export interface CompanyContactMethod {
  id: string;
  company_id: string;
  method_type: string;
  label?: string;
  value: string;
  is_primary: number;
  created_at: string;
}

export interface PersonWithSocials extends PersonRecord {
  social_profiles: PersonSocialProfile[];
  company_roles: PersonCompanyRole[];
  org_contacts: PersonOrgContact[];
}

export interface InvoiceRecord {
  id: string;
  invoice_number: string;
  person_id?: string;
  organization_id?: string;
  project_id?: string;
  invoice_type: string;
  status: string;
  amount_usd: number;
  amount_vibe: number;
  currency: string;
  title?: string;
  description?: string;
  line_items: string;
  issue_date?: string;
  due_date?: string;
  paid_at?: string;
  payment_method?: string;
  payment_reference?: string;
  notes?: string;
  created_at: string;
  updated_at: string;
}

export interface CreatePersonInput {
  full_name: string;
  email?: string;
  phone?: string;
  person_type?: string;
  financial_role?: string;
  client_profile?: string;
  business_stage?: string;
  lifecycle_stage?: string;
  company_name?: string;
  job_title?: string;
  website?: string;
  notes?: string;
  tags?: string[];
}

export interface UpdatePersonInput {
  full_name?: string;
  email?: string;
  phone?: string;
  person_type?: string;
  financial_role?: string;
  client_profile?: string;
  business_stage?: string;
  lifecycle_stage?: string;
  lead_score?: number;
  company_name?: string;
  job_title?: string;
  website?: string;
  notes?: string;
  tags?: string[];
  intelligence_summary?: string;
  onboarding_channel?: string;
  preferred_contact?: string;
}

export const personsApi = {
  list: async (params?: {
    person_type?: string;
    financial_role?: string;
    lifecycle_stage?: string;
    organization_id?: string;
    q?: string;
    limit?: number;
    offset?: number;
  }): Promise<PersonRecord[]> => {
    const qs = new URLSearchParams();
    if (params?.person_type) qs.set('person_type', params.person_type);
    if (params?.financial_role) qs.set('financial_role', params.financial_role);
    if (params?.lifecycle_stage) qs.set('lifecycle_stage', params.lifecycle_stage);
    if (params?.organization_id) qs.set('organization_id', params.organization_id);
    if (params?.q) qs.set('q', params.q);
    if (params?.limit) qs.set('limit', String(params.limit));
    if (params?.offset) qs.set('offset', String(params.offset));
    const response = await makeRequest(`/api/persons?${qs}`);
    return handleApiResponse<PersonRecord[]>(response);
  },

  get: async (id: string): Promise<PersonWithSocials> => {
    const response = await makeRequest(`/api/persons/${id}`);
    return handleApiResponse<PersonWithSocials>(response);
  },

  create: async (data: CreatePersonInput): Promise<PersonRecord> => {
    const response = await makeRequest('/api/persons', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonRecord>(response);
  },

  update: async (id: string, data: UpdatePersonInput): Promise<PersonRecord> => {
    const response = await makeRequest(`/api/persons/${id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonRecord>(response);
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  listSocialProfiles: async (id: string): Promise<PersonSocialProfile[]> => {
    const response = await makeRequest(`/api/persons/${id}/social-profiles`);
    return handleApiResponse<PersonSocialProfile[]>(response);
  },

  upsertSocialProfile: async (
    id: string,
    data: {
      platform: string;
      handle?: string;
      profile_url?: string;
      follower_count?: number;
      bio?: string;
      verified?: boolean;
    }
  ): Promise<PersonSocialProfile> => {
    const response = await makeRequest(`/api/persons/${id}/social-profiles`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonSocialProfile>(response);
  },

  deleteSocialProfile: async (id: string, platform: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}/social-profiles/${platform}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  listInvoices: async (id: string): Promise<InvoiceRecord[]> => {
    const response = await makeRequest(`/api/persons/${id}/invoices`);
    return handleApiResponse<InvoiceRecord[]>(response);
  },

  // Company affiliations
  listCompanies: async (id: string): Promise<PersonCompanyRole[]> => {
    const response = await makeRequest(`/api/persons/${id}/companies`);
    return handleApiResponse<PersonCompanyRole[]>(response);
  },
  addCompany: async (id: string, data: { company_id: string; role?: string; title?: string; is_primary?: boolean }): Promise<PersonCompanyRole> => {
    const response = await makeRequest(`/api/persons/${id}/companies`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonCompanyRole>(response);
  },
  updateCompanyRole: async (id: string, company_id: string, data: { role?: string; title?: string; is_primary?: boolean }): Promise<PersonCompanyRole> => {
    const response = await makeRequest(`/api/persons/${id}/companies/${company_id}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonCompanyRole>(response);
  },
  removeCompany: async (id: string, company_id: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}/companies/${company_id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },

  // Org affiliations
  listOrgs: async (id: string): Promise<PersonOrgContact[]> => {
    const response = await makeRequest(`/api/persons/${id}/organizations`);
    return handleApiResponse<PersonOrgContact[]>(response);
  },
  addOrg: async (id: string, data: { organization_id: string; context?: string }): Promise<PersonOrgContact> => {
    const response = await makeRequest(`/api/persons/${id}/organizations`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonOrgContact>(response);
  },
  removeOrg: async (id: string, org_id: string): Promise<void> => {
    const response = await makeRequest(`/api/persons/${id}/organizations/${org_id}`, { method: 'DELETE' });
    return handleApiResponse<void>(response);
  },
};

// ============================================================================
// Brand Guide Types
// ============================================================================

export interface OrgBrandProfile {
  id: string;
  organizationId: string;
  tagline?: string | null;
  primaryColor: string;
  secondaryColor: string;
  accentColor?: string | null;
  typographyHeading?: string | null;
  typographyBody?: string | null;
  logoUrl?: string | null;
  industry?: string | null;
  marketPosition?: string | null;
  uniqueValueProposition?: string | null;
  missionStatement?: string | null;
  visionStatement?: string | null;
  brandValues?: string | null;
  brandVoice?: string | null;
  brandArchetype?: string | null;
  targetAudience?: string | null;
  icpDescription?: string | null;
  icpCompanySize?: string | null;
  icpIndustries?: string | null;
  competitorBrands?: string | null;
  differentiators?: string | null;
  contentPillars?: string | null;
  contentTone?: string | null;
  websiteUrl?: string | null;
  socialInstagram?: string | null;
  socialTwitter?: string | null;
  socialLinkedin?: string | null;
  socialFacebook?: string | null;
  socialYoutube?: string | null;
  socialTiktok?: string | null;
  researchStatus: string;
  researchRanAt?: string | null;
  researchSummary?: string | null;
  moodBoardUrls?: string | null;
  clearbitLogoUrl?: string | null;
  brandPhotographyNotes?: string | null;
  researchIterations?: number;
  researchDepth?: number;
  founderName?: string | null;
  foundingYear?: string | null;
  keyClients?: string | null;
  estimatedTeamSize?: string | null;
  techStack?: string | null;
  geographicFocus?: string | null;
  fundingStage?: string | null;
  contentStrategyNotes?: string | null;
  awardsAndRecognition?: string | null;
  brandGapNotes?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface OrgKnowledgeSource {
  id: string;
  source_type: string;
  source_title: string;
  source_summary?: string | null;
  coverage_score: number;
  is_active: boolean;
  is_stale: boolean;
  project_id?: string | null;
  owner_type?: string | null;
  created_at: string;
  updated_at: string;
}
