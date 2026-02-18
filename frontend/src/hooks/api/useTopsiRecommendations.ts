import { useQuery } from '@tanstack/react-query';

export interface Recommendation {
  taskId: string;
  taskName: string;
  reason: string;
  vibeEstimate: number | null;
  timeContext: string | null;
  followUps: string[];
}

export interface RecommendationBatch {
  recommendations: Recommendation[];
  summary: string;
  generatedAt: string;
}

async function fetchRecommendations(projectId?: string): Promise<RecommendationBatch> {
  const url = projectId
    ? `/api/topsi/recommendations/${projectId}`
    : '/api/topsi/recommendations';
  const response = await fetch(url, { credentials: 'include' });
  if (!response.ok) {
    // Return empty batch on error (Topsi may not be initialized)
    return { recommendations: [], summary: '', generatedAt: new Date().toISOString() };
  }
  return response.json();
}

export function useTopsiRecommendations(projectId?: string) {
  return useQuery({
    queryKey: ['topsi-recommendations', projectId ?? 'all'],
    queryFn: () => fetchRecommendations(projectId),
    staleTime: 60 * 1000, // 1 minute
    refetchInterval: 2 * 60 * 1000, // Refetch every 2 minutes
    retry: 1,
  });
}
