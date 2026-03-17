import { useMutation, useQueryClient } from '@tanstack/react-query';
import { attemptsApi } from '@/lib/api';
import { branchKeys } from '@/lib/query-keys';

export function useMerge(
  attemptId?: string,
  onSuccess?: () => void,
  onError?: (err: unknown) => void
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => {
      if (!attemptId) return Promise.resolve();
      return attemptsApi.merge(attemptId);
    },
    onSuccess: () => {
      // Refresh attempt-specific branch information
      queryClient.invalidateQueries({ queryKey: branchKeys.status(attemptId) });

      // If a merge can change the list of branches shown elsewhere
      queryClient.invalidateQueries({ queryKey: branchKeys.projectBranches() });

      onSuccess?.();
    },
    onError: (err) => {
      console.error('Failed to merge:', err);
      onError?.(err);
    },
  });
}
