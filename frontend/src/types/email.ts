// Email Integration Types

export type EmailProvider = 'gmail' | 'zoho' | 'outlook' | 'imap_custom';

export type EmailAccountType = 'primary' | 'team' | 'notifications' | 'marketing' | 'support';

export type EmailAccountStatus = 'active' | 'inactive' | 'expired' | 'error' | 'pending_auth' | 'revoked';

export interface EmailAccount {
  id: string;
  project_id: string;
  provider: EmailProvider;
  account_type: EmailAccountType;
  email_address: string;
  display_name?: string;
  avatar_url?: string;
  granted_scopes?: string[];
  storage_used_bytes?: number;
  storage_total_bytes?: number;
  unread_count?: number;
  status: EmailAccountStatus;
  last_sync_at?: string;
  last_error?: string;
  sync_enabled?: boolean;
  sync_frequency_minutes?: number;
  auto_reply_enabled?: boolean;
  signature?: string;
  created_at: string;
  updated_at: string;
}

export interface CreateEmailAccount {
  project_id: string;
  provider: EmailProvider;
  account_type?: EmailAccountType;
  email_address: string;
  display_name?: string;
  avatar_url?: string;
  access_token?: string;
  refresh_token?: string;
  token_expires_at?: string;
  imap_host?: string;
  imap_port?: number;
  smtp_host?: string;
  smtp_port?: number;
  use_ssl?: boolean;
  granted_scopes?: string[];
  metadata?: Record<string, unknown>;
}

export interface UpdateEmailAccount {
  display_name?: string;
  avatar_url?: string;
  status?: EmailAccountStatus;
  sync_enabled?: boolean;
  sync_frequency_minutes?: number;
  auto_reply_enabled?: boolean;
  signature?: string;
}

export interface OAuthUrlResponse {
  auth_url: string;
  state: string;
}

export interface InitiateOAuthRequest {
  project_id: string;
  provider: EmailProvider;
  redirect_uri: string;
}

// Email message types
export interface EmailMessage {
  id: string;
  email_account_id: string;
  project_id: string;
  provider_message_id: string;
  thread_id?: string;
  from_address: string;
  from_name?: string;
  to_addresses: string[];
  cc_addresses?: string[];
  bcc_addresses?: string[];
  reply_to?: string;
  subject?: string;
  body_text?: string;
  body_html?: string;
  snippet?: string;
  has_attachments: boolean;
  attachments?: EmailAttachment[];
  labels?: string[];
  is_read: boolean;
  is_starred: boolean;
  is_draft: boolean;
  is_sent: boolean;
  is_archived: boolean;
  is_spam: boolean;
  is_trash: boolean;
  crm_contact_id?: string;
  crm_deal_id?: string;
  sentiment?: 'positive' | 'neutral' | 'negative' | 'unknown';
  priority?: 'low' | 'normal' | 'high' | 'urgent';
  needs_response: boolean;
  response_due_at?: string;
  responded_at?: string;
  received_at: string;
  sent_at?: string;
  created_at: string;
  updated_at: string;
}

export interface EmailAttachment {
  name: string;
  size: number;
  content_type: string;
  url?: string;
}

// Provider display info
export const EMAIL_PROVIDER_INFO: Record<EmailProvider, {
  name: string;
  icon: string;
  color: string;
  description: string;
}> = {
  gmail: {
    name: 'Gmail',
    icon: 'M',
    color: '#EA4335',
    description: 'Google Gmail - Use as master credentials for social platforms',
  },
  zoho: {
    name: 'Zoho Mail',
    icon: 'Z',
    color: '#C8202B',
    description: 'Zoho Mail - Team operations and CRM integration',
  },
  outlook: {
    name: 'Outlook',
    icon: 'O',
    color: '#0078D4',
    description: 'Microsoft Outlook / Office 365',
  },
  imap_custom: {
    name: 'Custom IMAP',
    icon: '@',
    color: '#6B7280',
    description: 'Connect any email provider via IMAP/SMTP',
  },
};

// Account type display info
export const ACCOUNT_TYPE_INFO: Record<EmailAccountType, {
  label: string;
  description: string;
  icon: string;
}> = {
  primary: {
    label: 'Primary',
    description: 'Main account for authentication and master credentials',
    icon: 'star',
  },
  team: {
    label: 'Team',
    description: 'Shared team inbox for internal operations',
    icon: 'users',
  },
  notifications: {
    label: 'Notifications',
    description: 'Receive platform notifications and alerts',
    icon: 'bell',
  },
  marketing: {
    label: 'Marketing',
    description: 'Outbound marketing and campaigns',
    icon: 'megaphone',
  },
  support: {
    label: 'Support',
    description: 'Customer support and inquiries',
    icon: 'help-circle',
  },
};
