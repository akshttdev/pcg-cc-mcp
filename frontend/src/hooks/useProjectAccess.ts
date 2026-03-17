import { useQuery } from '@tanstack/react-query';
import { resolveApiUrl } from '@/lib/api';
import { projectAccessKeys } from '@/lib/query-keys';

export interface ProjectAccess {
  has_access: boolean;
  role: string | null;
  can_read: boolean;
  can_write: boolean;
  can_manage_members: boolean;
  can_delete: boolean;
  access_scope: 'full' | 'assigned_only' | 'none';
  platform_roles: string[];
}

async function fetchProjectAccess(projectId: string): Promise<ProjectAccess> {
  const res = await fetch(resolveApiUrl(`/api/projects/${projectId}/access`), {
    credentials: 'include',
  });
  if (!res.ok) {
    return {
      has_access: false,
      role: null,
      can_read: false,
      can_write: false,
      can_manage_members: false,
      can_delete: false,
      access_scope: 'none',
      platform_roles: [],
    };
  }
  const json = await res.json();
  return json.data;
}

export function useProjectAccess(projectId: string | undefined) {
  return useQuery({
    queryKey: projectAccessKeys.access(projectId),
    queryFn: () => fetchProjectAccess(projectId!),
    enabled: !!projectId,
    staleTime: 60_000,
  });
}
