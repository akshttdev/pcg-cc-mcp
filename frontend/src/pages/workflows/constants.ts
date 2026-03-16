// ─── Shared Constants for Workflows Page ──────────────────────────────────────

import { Users, Building2, Handshake, ListTodo } from 'lucide-react';
import type { GlobalFilterType } from './types';

export const VALID_GLOBAL_FILTERS: GlobalFilterType[] = [
  'all', 'valid', 'duplicates', 'approved', 'rejected', 'issues', 'error',
];

export const STAGING_TARGET_CONFIG: Record<string, { label: string; icon: typeof Users; color: string }> = {
  crm_contact: { label: 'Contacts', icon: Users, color: 'text-blue-500' },
  company: { label: 'Companies', icon: Building2, color: 'text-purple-500' },
  crm_deal: { label: 'Deals', icon: Handshake, color: 'text-green-500' },
  task: { label: 'Tasks', icon: ListTodo, color: 'text-orange-500' },
};
