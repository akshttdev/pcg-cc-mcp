import { useQuery } from '@tanstack/react-query';
import { organizationsApi } from '@/lib/api';
import type { OrganizationData } from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';

const ORG_STALE_TIME = 5 * 60 * 1000; // 5 minutes

// Re-export for backward compatibility — prefer importing from @/lib/query-keys directly
export const organizationByIdQueryKey = organizationKeys.detail;

export function useOrganizationById(orgId: string | undefined) {
  return useQuery<OrganizationData>({
    queryKey: organizationKeys.detail(orgId),
    queryFn: () => organizationsApi.getById(orgId!),
    enabled: !!orgId,
    staleTime: ORG_STALE_TIME,
  });
}
