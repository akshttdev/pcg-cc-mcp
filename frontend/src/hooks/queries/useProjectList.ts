import { useQuery } from '@tanstack/react-query';
import { projectsApi } from '@/lib/api';
import { projectKeys } from '@/lib/query-keys';

const PROJECTS_STALE_TIME = 5 * 60 * 1000; // 5 minutes

// Re-export for backward compatibility — prefer importing from @/lib/query-keys directly
export const PROJECT_LIST_QUERY_KEY = projectKeys.all;

export function useProjectList() {
  return useQuery({
    queryKey: projectKeys.all,
    queryFn: () => projectsApi.getAll(),
    staleTime: PROJECTS_STALE_TIME,
  });
}
