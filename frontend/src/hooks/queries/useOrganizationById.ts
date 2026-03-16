import { useQuery } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api';
import type { OrganizationData } from '@/lib/api';

const ORG_STALE_TIME = 5 * 60 * 1000; // 5 minutes

export const organizationByIdQueryKey = (orgId: string | undefined) =>
  ['organization', orgId] as const;

export function useOrganizationById(orgId: string | undefined) {
  return useQuery<OrganizationData>({
    queryKey: organizationByIdQueryKey(orgId),
    queryFn: () => organizationsApi.getById(orgId!),
    enabled: !!orgId,
    staleTime: ORG_STALE_TIME,
  });
}
