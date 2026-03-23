import type { EmailAccountRecord } from '@/lib/api';

export function mapEmailAccounts(records: EmailAccountRecord[]) {
  return records.map((record) => ({
    id: record.id,
    project_id: record.project_id,
    provider: record.provider as 'gmail' | 'zoho' | 'imap_custom',
    account_type: (record.account_type || 'primary') as
      | 'primary'
      | 'team'
      | 'notifications'
      | 'marketing'
      | 'support',
    email_address: record.email_address,
    display_name: record.display_name ?? undefined,
    avatar_url: record.avatar_url ?? undefined,
    unread_count: record.unread_count ?? undefined,
    status: (record.status || 'active') as
      | 'active'
      | 'inactive'
      | 'expired'
      | 'error'
      | 'pending_auth'
      | 'revoked',
    last_sync_at: record.last_sync_at ?? undefined,
    last_error: record.last_error ?? undefined,
    sync_enabled: record.sync_enabled === 1,
    created_at: record.created_at,
    updated_at: record.updated_at,
  }));
}
