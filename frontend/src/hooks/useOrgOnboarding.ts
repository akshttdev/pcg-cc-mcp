import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { orgOnboardingApi } from '@/lib/api';
import { toast } from 'sonner';

const ORG_ONBOARDING_KEY = 'org-onboarding';

export function useOrgOnboarding(orgId: string | undefined) {
  const queryClient = useQueryClient();

  const invalidate = () => queryClient.invalidateQueries({ queryKey: [ORG_ONBOARDING_KEY, orgId] });

  const query = useQuery({
    queryKey: [ORG_ONBOARDING_KEY, orgId],
    queryFn: () => orgOnboardingApi.getByOrg(orgId!),
    enabled: !!orgId,
    staleTime: 60_000,
  });

  const startOnboarding = useMutation({
    mutationFn: (contextData?: string) => orgOnboardingApi.startOrg(orgId!, contextData),
    onSuccess: invalidate,
    onError: () => toast.error('Failed to start onboarding'),
  });

  const startSegment = useMutation({
    mutationFn: (segmentId: string) => orgOnboardingApi.startOrgSegment(segmentId),
    onSuccess: invalidate,
    onError: () => toast.error('Failed to start segment'),
  });

  const completeSegment = useMutation({
    mutationFn: ({ segmentId, opts }: { segmentId: string; opts?: { user_decisions?: string; skip?: boolean } }) =>
      orgOnboardingApi.completeOrgSegment(segmentId, opts),
    onSuccess: invalidate,
    onError: () => toast.error('Failed to complete segment'),
  });

  const skipSegment = useMutation({
    mutationFn: (segmentId: string) => orgOnboardingApi.completeOrgSegment(segmentId, { skip: true }),
    onSuccess: invalidate,
    onError: () => toast.error('Failed to skip segment'),
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
