import { useQuery } from '@tanstack/react-query';
import { crmApi } from '@/lib/api';
import type { CrmContactRecord } from '@/lib/api';
import { crmKeys } from '@/lib/query-keys';

const CRM_CONTACTS_STALE_TIME = 30_000; // 30 seconds

// Re-export for backward compatibility — prefer importing from @/lib/query-keys directly
export const crmContactsQueryKey = crmKeys.contacts;

export function useCrmContacts(orgId: string | undefined) {
  return useQuery<CrmContactRecord[]>({
    queryKey: crmKeys.contacts(orgId),
    queryFn: () => crmApi.listContacts(orgId!),
    enabled: !!orgId,
    staleTime: CRM_CONTACTS_STALE_TIME,
  });
}
