import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { orgOnboardingApi } from '@/lib/api';

const ORG_ONBOARDING_KEY = 'org-onboarding';

export function useOrgOnboarding(orgId: string | undefined) {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: [ORG_ONBOARDING_KEY, orgId],
    queryFn: () => orgOnboardingApi.getByOrg(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const startOnboarding = useMutation({
    mutationFn: (contextData?: string) => orgOnboardingApi.startOrg(orgId!, contextData),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ORG_ONBOARDING_KEY, orgId] });
    },
  });

  const startSegment = useMutation({
    mutationFn: (segmentId: string) => orgOnboardingApi.startOrgSegment(segmentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ORG_ONBOARDING_KEY, orgId] });
    },
  });

  const completeSegment = useMutation({
    mutationFn: ({ segmentId, opts }: { segmentId: string; opts?: { user_decisions?: string; skip?: boolean } }) =>
      orgOnboardingApi.completeOrgSegment(segmentId, opts),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ORG_ONBOARDING_KEY, orgId] });
    },
  });

  const skipSegment = useMutation({
    mutationFn: (segmentId: string) => orgOnboardingApi.completeOrgSegment(segmentId, { skip: true }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: [ORG_ONBOARDING_KEY, orgId] });
    },
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
