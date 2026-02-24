import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { crmActivitiesApi } from '@/lib/api';

export const crmActivityQueryKeys = {
  activities: (projectId: string) => ['crm', 'activities', projectId] as const,
  contactActivities: (contactId: string) => ['crm', 'activities', 'contact', contactId] as const,
  dealActivities: (dealId: string) => ['crm', 'activities', 'deal', dealId] as const,
};

export function useCrmActivities(options: {
  projectId?: string;
  contactId?: string;
  dealId?: string;
  limit?: number;
}) {
  const queryKey = options.contactId
    ? crmActivityQueryKeys.contactActivities(options.contactId)
    : options.dealId
      ? crmActivityQueryKeys.dealActivities(options.dealId)
      : crmActivityQueryKeys.activities(options.projectId || '');

  return useQuery({
    queryKey,
    queryFn: () =>
      crmActivitiesApi.listActivities({
        project_id: options.projectId,
        contact_id: options.contactId,
        deal_id: options.dealId,
        limit: options.limit,
      }),
    enabled: !!(options.projectId || options.contactId || options.dealId),
    staleTime: 30 * 1000,
  });
}

export function useCreateActivity() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: crmActivitiesApi.createActivity,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm', 'activities'] });
    },
  });
}

export function useDeleteActivity() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: crmActivitiesApi.deleteActivity,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['crm', 'activities'] });
    },
  });
}
