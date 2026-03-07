import { useQuery } from '@tanstack/react-query';
import { personsApi, type PersonRecord } from '@/lib/api';

// OrgContact is just a PersonRecord — org contacts come from the persons table
// (seeded via meeting ingestion, intelligence pipeline, manual create, etc.)
export type OrgContact = PersonRecord;

export function useOrgContacts(orgId: string | undefined) {
  const { data: contacts = [], isLoading } = useQuery<PersonRecord[]>({
    queryKey: ['org-persons', orgId],
    queryFn: () => personsApi.list({ organization_id: orgId!, limit: 500 }),
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
