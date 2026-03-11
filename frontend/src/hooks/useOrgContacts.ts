import { useQuery } from '@tanstack/react-query';
import { personsApi, type PersonRecord } from '@/lib/api';

// OrgContact maps to PersonRecord — intake-derived leads/contacts live in persons table
export type OrgContact = PersonRecord;

export function useOrgContacts(orgId: string | undefined) {
  const { data: contacts = [], isLoading } = useQuery<OrgContact[]>({
    queryKey: ['org-persons', orgId],
    queryFn: () => personsApi.list({ organization_id: orgId!, limit: 500 }),
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
