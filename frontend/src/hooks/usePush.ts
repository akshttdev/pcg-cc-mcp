import { useMutation, useQueryClient } from '@tanstack/react-query';
import { attemptsApi } from '@/lib/api';
import { branchKeys } from '@/lib/query-keys';

export function usePush(
  attemptId?: string,
  onSuccess?: () => void,
  onError?: (err: unknown) => void
) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => {
      if (!attemptId) return Promise.resolve();
      return attemptsApi.push(attemptId);
    },
    onSuccess: () => {
      // A push only affects remote status; invalidate the same branchStatus
      queryClient.invalidateQueries({ queryKey: branchKeys.status(attemptId) });
      onSuccess?.();
    },
    onError: (err) => {
      console.error('Failed to push:', err);
      onError?.(err);
    },
  });
}
