import { handleApiResponse, makeRequest } from './client';
import type { CrmContactRecord } from './crm';

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
  by_account: Array<{
    account_id: string;
    email_address: string;
    total: number;
    unread: number;
  }>;
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
    if (params.email_account_id)
      qs.set('email_account_id', params.email_account_id);
    if (params.is_read !== undefined) qs.set('is_read', String(params.is_read));
    if (params.is_starred !== undefined)
      qs.set('is_starred', String(params.is_starred));
    if (params.needs_response !== undefined)
      qs.set('needs_response', String(params.needs_response));
    if (params.limit) qs.set('limit', String(params.limit));
    const response = await makeRequest(`/api/email/messages?${qs}`);
    return handleApiResponse<EmailMessageRecord[]>(response);
  },
  getInboxStats: async (projectId: string): Promise<EmailInboxStats> => {
    const response = await makeRequest(
      `/api/email/messages/stats/${projectId}`
    );
    return handleApiResponse<EmailInboxStats>(response);
  },
  markAsRead: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/read`, {
      method: 'POST',
    });
    return handleApiResponse<void>(response);
  },
  toggleStar: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/star`, {
      method: 'POST',
    });
    return handleApiResponse<void>(response);
  },
  moveToTrash: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/email/messages/${id}/trash`, {
      method: 'POST',
    });
    return handleApiResponse<void>(response);
  },
};

// ============================================================
// Backward-compatible type alias — use CrmContactRecord directly in new code
// ============================================================

/** @deprecated Use CrmContactRecord from '@/lib/api/crm' instead */
export type PersonRecord = CrmContactRecord;

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

export interface PersonWithSocials extends CrmContactRecord {
  social_profiles: PersonSocialProfile[];
  company_roles: PersonCompanyRole[];
  org_contacts: PersonOrgContact[];
}

export interface PersonNote {
  id: string;
  crm_contact_id: string | null;
  author_id?: string;
  text: string;
  status: string;
  attachments: string;
  proposal_id?: string;
  created_at: string;
  updated_at: string;
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

/** @deprecated Use CreateCrmContactRequest from '@/lib/api/crm' instead */
export type CreatePersonInput = import('./crm').CreateCrmContactRequest;

/** @deprecated Use UpdateCrmContactRequest from '@/lib/api/crm' instead */
export type UpdatePersonInput = import('./crm').UpdateCrmContactRequest;

/**
 * @deprecated Use crmApi from '@/lib/api/crm' for main CRUD.
 * Sub-resource endpoints (notes, social profiles, companies, orgs, invoices)
 * now point to /crm/contacts/* routes.
 */
export const personsApi = {
  // Main CRUD — delegate to crmApi
  list: async (params?: {
    person_type?: string;
    financial_role?: string;
    lifecycle_stage?: string;
    organization_id?: string;
    q?: string;
    limit?: number;
    offset?: number;
  }): Promise<CrmContactRecord[]> => {
    const { crmApi } = await import('./crm');
    return crmApi.listContacts(params?.organization_id ?? '', {
      lifecycleStage: params?.lifecycle_stage,
      limit: params?.limit,
    });
  },

  get: async (id: string): Promise<CrmContactRecord> => {
    const { crmApi } = await import('./crm');
    return crmApi.getContact(id);
  },

  // Sub-resource endpoints (migrated to /crm/contacts)
  listNotes: async (id: string): Promise<PersonNote[]> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/notes`);
    return handleApiResponse<PersonNote[]>(response);
  },
  createNote: async (
    id: string,
    text: string,
    status?: string
  ): Promise<PersonNote> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/notes`, {
      method: 'POST',
      body: JSON.stringify({ text, status: status ?? 'open' }),
    });
    return handleApiResponse<PersonNote>(response);
  },
  updateNote: async (
    noteId: string,
    data: { text?: string; status?: string }
  ): Promise<PersonNote> => {
    const response = await makeRequest(`/api/crm/contact-notes/${noteId}`, {
      method: 'PATCH',
      body: JSON.stringify(data),
    });
    return handleApiResponse<PersonNote>(response);
  },
  deleteNote: async (noteId: string): Promise<void> => {
    const response = await makeRequest(`/api/crm/contact-notes/${noteId}`, {
      method: 'DELETE',
    });
    return handleApiResponse<void>(response);
  },

  listSocialProfiles: async (id: string): Promise<PersonSocialProfile[]> => {
    const response = await makeRequest(
      `/api/crm/contacts/${id}/social-profiles`
    );
    return handleApiResponse<PersonSocialProfile[]>(response);
  },

  listCompanies: async (id: string): Promise<PersonCompanyRole[]> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/companies`);
    return handleApiResponse<PersonCompanyRole[]>(response);
  },

  listInvoices: async (id: string): Promise<InvoiceRecord[]> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/invoices`);
    return handleApiResponse<InvoiceRecord[]>(response);
  },

  listOrgs: async (id: string): Promise<PersonOrgContact[]> => {
    const response = await makeRequest(`/api/crm/contacts/${id}/organizations`);
    return handleApiResponse<PersonOrgContact[]>(response);
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
