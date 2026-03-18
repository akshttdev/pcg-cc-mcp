import { useQuery } from '@tanstack/react-query';
import { orgOnboardingApi } from '@/lib/api';
import { organizationKeys } from '@/lib/query-keys';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';

export function useOrgOnboarding(orgId: string | undefined) {
  const onboardingKey = organizationKeys.onboarding(orgId!);

  const query = useQuery({
    queryKey: onboardingKey,
    queryFn: () => orgOnboardingApi.getByOrg(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const startOnboarding = useMutationWithToast({
    mutationFn: (contextData?: string) => orgOnboardingApi.startOrg(orgId!, contextData),
    errorMessage: 'Failed to start onboarding',
    invalidateKeys: [onboardingKey],
  });

  const startSegment = useMutationWithToast({
    mutationFn: (segmentId: string) => orgOnboardingApi.startOrgSegment(segmentId),
    errorMessage: 'Failed to start segment',
    invalidateKeys: [onboardingKey],
  });

  const completeSegment = useMutationWithToast({
    mutationFn: ({ segmentId, opts }: { segmentId: string; opts?: { user_decisions?: string; skip?: boolean } }) =>
      orgOnboardingApi.completeOrgSegment(segmentId, opts),
    errorMessage: 'Failed to complete segment',
    invalidateKeys: [onboardingKey],
  });

  const skipSegment = useMutationWithToast({
    mutationFn: (segmentId: string) => orgOnboardingApi.completeOrgSegment(segmentId, { skip: true }),
    errorMessage: 'Failed to skip segment',
    invalidateKeys: [onboardingKey],
  });

  return {
    data: query.data,
    isLoading: query.isLoading,
    startOnboarding,
    startSegment,
    completeSegment,
    skipSegment,
  };
}
