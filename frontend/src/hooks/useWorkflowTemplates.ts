import { useQuery } from '@tanstack/react-query';
import { workflowTemplatesApi } from '@/lib/api';
import { workflowTemplateKeys, crmKeys, projectKeys } from '@/lib/query-keys';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import type { ConvertDealRequest } from '@/lib/api';

export function useWorkflowTemplates() {
  return useQuery({
    queryKey: workflowTemplateKeys.all(),
    queryFn: () => workflowTemplatesApi.list(),
    staleTime: 10 * 60 * 1000, // Templates rarely change
  });
}

export function useConvertDeal() {
  return useMutationWithToast({
    mutationFn: ({ dealId, data }: { dealId: string; data: ConvertDealRequest }) =>
      workflowTemplatesApi.convertDeal(dealId, data),
    successMessage: 'Deal converted successfully',
    errorMessage: 'Failed to convert deal',
    invalidateKeys: [crmKeys.all, projectKeys.all],
  });
}
