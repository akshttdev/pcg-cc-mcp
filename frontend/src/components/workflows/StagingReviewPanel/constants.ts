import { Users, Building2, Handshake, ListTodo } from 'lucide-react';

export const TARGET_TYPE_CONFIG = {
  crm_contact: { label: 'Contacts', icon: Users, color: 'text-blue-500' },
  company: { label: 'Companies', icon: Building2, color: 'text-purple-500' },
  crm_deal: { label: 'Deals', icon: Handshake, color: 'text-green-500' },
  task: { label: 'Tasks', icon: ListTodo, color: 'text-orange-500' },
} as const;
