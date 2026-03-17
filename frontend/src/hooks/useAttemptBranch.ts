import { useQuery } from '@tanstack/react-query';
import { attemptsApi } from '@/lib/api';
import { branchKeys } from '@/lib/query-keys';

export function useAttemptBranch(attemptId?: string) {
  const query = useQuery({
    queryKey: branchKeys.attemptBranch(attemptId),
    queryFn: async () => {
      const attempt = await attemptsApi.get(attemptId!);
      return attempt.branch ?? null;
    },
    enabled: !!attemptId,
  });

  return {
    branch: query.data ?? null,
    isLoading: query.isLoading,
    refetch: query.refetch,
  } as const;
}
