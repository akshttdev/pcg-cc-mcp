import { useQuery } from '@tanstack/react-query';
import { tasksApi } from '@/lib/api';
import { taskKeys } from '@/lib/query-keys';

export function useTaskList(projectId: string) {
  return useQuery({
    queryKey: taskKeys.list(projectId),
    queryFn: () => tasksApi.getAll(projectId),
    enabled: !!projectId,
    staleTime: 2 * 60 * 1000, // 2 minutes
  });
}
