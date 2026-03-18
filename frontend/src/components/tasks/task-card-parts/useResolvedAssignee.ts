import { useMemo } from 'react';
import type { UserListItem } from '@/lib/api';

interface ResolvedAssignee {
  displayName: string;
  initials: string;
}

export function useResolvedAssignee(
  assigneeId: string | null | undefined,
  usersMap?: Map<string, UserListItem>,
): ResolvedAssignee | null {
  return useMemo(() => {
    if (!assigneeId) return null;
    const assignee = usersMap?.get(assigneeId);
    const displayName = assignee?.full_name || assignee?.username || assignee?.email || assigneeId;
    const initials = assignee?.full_name
      ? assignee.full_name.split(' ').map(n => n[0]).join('').toUpperCase().slice(0, 2)
      : (assignee?.username?.[0] || displayName[0] || '?').toUpperCase();
    return { displayName, initials };
  }, [assigneeId, usersMap]);
}
