import { useQuery } from '@tanstack/react-query';
import { crmApi } from '@/lib/api';
import type { CrmContactRecord } from '@/lib/api';

const CRM_CONTACTS_STALE_TIME = 30_000; // 30 seconds

export const crmContactsQueryKey = (orgId: string | undefined) =>
  ['crm-contacts', orgId] as const;

export function useCrmContacts(orgId: string | undefined) {
  return useQuery<CrmContactRecord[]>({
    queryKey: crmContactsQueryKey(orgId),
    queryFn: () => crmApi.listContacts(orgId!),
    enabled: !!orgId,
    staleTime: CRM_CONTACTS_STALE_TIME,
  });
}
