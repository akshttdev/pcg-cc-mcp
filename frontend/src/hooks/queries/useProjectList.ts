import { useQuery } from '@tanstack/react-query';
import { projectsApi } from '@/lib/api';

const PROJECTS_STALE_TIME = 5 * 60 * 1000; // 5 minutes

export const PROJECT_LIST_QUERY_KEY = ['projects'] as const;

export function useProjectList() {
  return useQuery({
    queryKey: PROJECT_LIST_QUERY_KEY,
    queryFn: () => projectsApi.getAll(),
    staleTime: PROJECTS_STALE_TIME,
  });
}
