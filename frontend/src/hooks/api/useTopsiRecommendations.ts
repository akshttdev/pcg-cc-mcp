import { useQuery } from '@tanstack/react-query';
import { topsiApi } from '@/lib/api';
import { topsiKeys } from '@/lib/query-keys';
import type { RecommendationBatch } from '@/lib/api/topsi';

export type { Recommendation, RecommendationBatch } from '@/lib/api/topsi';

export function useTopsiRecommendations(projectId?: string) {
  return useQuery({
    queryKey: topsiKeys.recommendations(projectId),
    queryFn: async (): Promise<RecommendationBatch> => {
      try {
        return await topsiApi.getRecommendations(projectId);
      } catch {
        // Return empty batch on error (Topsi may not be initialized)
        return { recommendations: [], summary: '', generatedAt: new Date().toISOString() };
      }
    },
    staleTime: 60 * 1000, // 1 minute
    refetchInterval: 2 * 60 * 1000, // Refetch every 2 minutes
    retry: 1,
  });
}
