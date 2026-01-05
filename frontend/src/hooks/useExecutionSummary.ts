import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { executionSummaryApi } from '@/lib/api';

interface UpdateFeedbackRequest {
  human_rating?: number | null;
  human_notes?: string | null;
  is_reference_example?: boolean | null;
}

export function useExecutionSummary(attemptId: string | null | undefined) {
  return useQuery({
    queryKey: ['execution-summary', attemptId],
    queryFn: async () => {
      if (!attemptId) return null;
      return executionSummaryApi.getByAttemptId(attemptId);
    },
    enabled: !!attemptId,
    staleTime: 30000, // Consider data fresh for 30 seconds
  });
}

export function useUpdateExecutionSummaryFeedback(summaryId: string | undefined) {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (feedback: UpdateFeedbackRequest) => {
      if (!summaryId) throw new Error('Summary ID required');
      return executionSummaryApi.updateFeedback(summaryId, feedback);
    },
    onSuccess: (data) => {
      // Update the cache with the new data
      queryClient.setQueryData(['execution-summary', data.task_attempt_id], data);
    },
  });
}
