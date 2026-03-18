import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { workflowTemplatesApi } from '@/lib/api';
import { workflowTemplateKeys, crmKeys, projectKeys } from '@/lib/query-keys';
import type { ConvertDealRequest } from '@/lib/api';

export function useWorkflowTemplates() {
  return useQuery({
    queryKey: workflowTemplateKeys.all(),
    queryFn: () => workflowTemplatesApi.list(),
    staleTime: 10 * 60 * 1000, // Templates rarely change
  });
}

export function useConvertDeal() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ dealId, data }: { dealId: string; data: ConvertDealRequest }) =>
      workflowTemplatesApi.convertDeal(dealId, data),
    onSuccess: () => {
      // Invalidate deals (custom_fields updated) and projects (new project created)
      queryClient.invalidateQueries({ queryKey: crmKeys.all });
      queryClient.invalidateQueries({ queryKey: projectKeys.all });
    },
  });
}
