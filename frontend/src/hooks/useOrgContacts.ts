import { useQuery } from '@tanstack/react-query';
import { crmApi, type CrmContactRecord } from '@/lib/api';

// OrgContact is now a CrmContactRecord (org-scoped CRM contacts)
export type OrgContact = CrmContactRecord & { person_type?: string };

export function useOrgContacts(orgId: string | undefined) {
  const { data: contacts = [], isLoading } = useQuery<OrgContact[]>({
    queryKey: ['org-crm-contacts', orgId],
    queryFn: () => crmApi.listContacts(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  return {
    contacts,
    isLoading,
    projectCount: 0,
    loadedCount: isLoading ? 0 : 1,
  };
}
