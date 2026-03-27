import { useQuery } from '@tanstack/react-query';
import { crmApi, type CrmContactRecord } from '@/lib/api';
import { crmKeys } from '@/lib/query-keys';

export type OrgContact = CrmContactRecord;

export function useOrgContacts(orgId: string | undefined) {
  const { data: contacts = [], isLoading } = useQuery<OrgContact[]>({
    queryKey: crmKeys.contacts(orgId),
    queryFn: () => crmApi.listContacts(orgId!, { limit: 500 }),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  return {
    contacts,
    isLoading,
    projectCount: 0,
    loadedCount: isLoading ? 0 : 1,
  };
}
