// CRM Types

export type ContactSource =
  | 'manual'
  | 'email'
  | 'social'
  | 'website'
  | 'referral'
  | 'import'
  | 'api'
  | 'zoho_sync'
  | 'gmail_sync';

export type LifecycleStage =
  | 'subscriber'
  | 'lead'
  | 'mql'
  | 'sql'
  | 'opportunity'
  | 'customer'
  | 'evangelist'
  | 'churned';

export type DealStage =
  | 'qualification'
  | 'discovery'
  | 'proposal'
  | 'negotiation'
  | 'closed_won'
  | 'closed_lost';

export type CrmActivityType =
  | 'email_sent'
  | 'email_received'
  | 'email_opened'
  | 'email_clicked'
  | 'call_made'
  | 'call_received'
  | 'call_scheduled'
  | 'meeting_scheduled'
  | 'meeting_completed'
  | 'meeting_cancelled'
  | 'note_added'
  | 'task_created'
  | 'task_completed'
  | 'deal_stage_changed'
  | 'deal_created'
  | 'deal_won'
  | 'deal_lost'
  | 'social_mention'
  | 'social_dm'
  | 'social_comment'
  | 'form_submitted'
  | 'page_visited'
  | 'document_viewed'
  | 'custom';

export interface CrmContact {
  id: string;
  project_id: string;
  first_name?: string;
  last_name?: string;
  full_name?: string;
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
  source?: ContactSource;
  lifecycle_stage: LifecycleStage;
  lead_score: number;
  last_activity_at?: string;
  last_contacted_at?: string;
  last_replied_at?: string;
  owner_user_id?: string;
  assigned_agent_id?: string;
  zoho_contact_id?: string;
  gmail_contact_id?: string;
  external_ids?: Record<string, string>;
  tags?: string[];
  lists?: string[];
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
  email_count: number;
  meeting_count: number;
  deal_count: number;
  total_revenue: number;
  created_at: string;
  updated_at: string;
}

export interface CreateCrmContact {
  project_id: string;
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
  source?: ContactSource;
  lifecycle_stage?: LifecycleStage;
  tags?: string[];
  custom_fields?: Record<string, unknown>;
  zoho_contact_id?: string;
  gmail_contact_id?: string;
}

export interface UpdateCrmContact {
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
  source?: ContactSource;
  lifecycle_stage?: LifecycleStage;
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

export interface CrmDeal {
  id: string;
  project_id: string;
  crm_contact_id?: string;
  name: string;
  description?: string;
  amount?: number;
  currency: string;
  pipeline: string;
  stage: DealStage;
  probability: number;
  expected_close_date?: string;
  actual_close_date?: string;
  last_activity_at?: string;
  owner_user_id?: string;
  assigned_agent_id?: string;
  zoho_deal_id?: string;
  external_ids?: Record<string, string>;
  tags?: string[];
  custom_fields?: Record<string, unknown>;
  lost_reason?: string;
  win_reason?: string;
  created_at: string;
  updated_at: string;
}

export interface CrmActivity {
  id: string;
  project_id: string;
  crm_contact_id?: string;
  crm_deal_id?: string;
  activity_type: CrmActivityType;
  subject?: string;
  description?: string;
  outcome?: string;
  email_message_id?: string;
  social_mention_id?: string;
  task_id?: string;
  performed_by_user?: string;
  performed_by_agent_id?: string;
  metadata?: Record<string, unknown>;
  duration_minutes?: number;
  activity_at: string;
  created_at: string;
}

export interface ContactStats {
  total: number;
  by_stage: Array<{ stage: string; count: number }>;
  avg_lead_score: number;
  needs_follow_up: number;
}

// Lifecycle stage display info
export const LIFECYCLE_STAGE_INFO: Record<LifecycleStage, {
  label: string;
  description: string;
  color: string;
  icon: string;
}> = {
  subscriber: {
    label: 'Subscriber',
    description: 'Opted in to receive communications',
    color: '#6B7280',
    icon: 'mail',
  },
  lead: {
    label: 'Lead',
    description: 'Potential customer showing interest',
    color: '#3B82F6',
    icon: 'user-plus',
  },
  mql: {
    label: 'MQL',
    description: 'Marketing Qualified Lead',
    color: '#8B5CF6',
    icon: 'target',
  },
  sql: {
    label: 'SQL',
    description: 'Sales Qualified Lead',
    color: '#F59E0B',
    icon: 'phone',
  },
  opportunity: {
    label: 'Opportunity',
    description: 'Active sales opportunity',
    color: '#EF4444',
    icon: 'dollar-sign',
  },
  customer: {
    label: 'Customer',
    description: 'Paying customer',
    color: '#22C55E',
    icon: 'check-circle',
  },
  evangelist: {
    label: 'Evangelist',
    description: 'Active promoter and referrer',
    color: '#EC4899',
    icon: 'star',
  },
  churned: {
    label: 'Churned',
    description: 'Former customer',
    color: '#9CA3AF',
    icon: 'x-circle',
  },
};

// Deal stage display info
export const DEAL_STAGE_INFO: Record<DealStage, {
  label: string;
  description: string;
  color: string;
  probability: number;
}> = {
  qualification: {
    label: 'Qualification',
    description: 'Initial qualification of the opportunity',
    color: '#3B82F6',
    probability: 10,
  },
  discovery: {
    label: 'Discovery',
    description: 'Understanding needs and requirements',
    color: '#8B5CF6',
    probability: 25,
  },
  proposal: {
    label: 'Proposal',
    description: 'Proposal sent to prospect',
    color: '#F59E0B',
    probability: 50,
  },
  negotiation: {
    label: 'Negotiation',
    description: 'Negotiating terms and pricing',
    color: '#EF4444',
    probability: 75,
  },
  closed_won: {
    label: 'Closed Won',
    description: 'Deal successfully closed',
    color: '#22C55E',
    probability: 100,
  },
  closed_lost: {
    label: 'Closed Lost',
    description: 'Deal lost to competitor or no decision',
    color: '#9CA3AF',
    probability: 0,
  },
};

// Contact source display info
export const CONTACT_SOURCE_INFO: Record<ContactSource, {
  label: string;
  icon: string;
}> = {
  manual: { label: 'Manual Entry', icon: 'edit' },
  email: { label: 'Email', icon: 'mail' },
  social: { label: 'Social Media', icon: 'share-2' },
  website: { label: 'Website', icon: 'globe' },
  referral: { label: 'Referral', icon: 'users' },
  import: { label: 'Import', icon: 'upload' },
  api: { label: 'API', icon: 'code' },
  zoho_sync: { label: 'Zoho CRM Sync', icon: 'refresh-cw' },
  gmail_sync: { label: 'Gmail Sync', icon: 'mail' },
};
