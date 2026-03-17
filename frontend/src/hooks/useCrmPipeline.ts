import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';
import { crmPipelinesApi, crmDealsApi, projectsApi, organizationsApi } from '@/lib/api';
import type {
  PipelineType,
  CreateCrmDeal,
  UpdateCrmDeal,
  MoveDealRequest,
  CrmDealWithContact,
} from '@/types/crm';

import { crmKeys, projectKeys, organizationKeys } from '@/lib/query-keys';

// Re-export for backward compatibility — prefer importing from @/lib/query-keys directly
export const crmQueryKeys = {
  pipelines: crmKeys.pipelines,
  pipelinesByType: crmKeys.pipelinesByType,
  pipeline: crmKeys.pipeline,
  kanban: crmKeys.kanban,
  deals: crmKeys.deals,
  deal: crmKeys.deal,
};

// Hook to list all pipelines for an organization
export function useCrmPipelines(organizationId: string, pipelineType?: PipelineType) {
  return useQuery({
    queryKey: pipelineType
      ? crmQueryKeys.pipelinesByType(organizationId, pipelineType)
      : crmQueryKeys.pipelines(organizationId),
    queryFn: () =>
      crmPipelinesApi.listPipelines(organizationId, pipelineType ? { pipelineType } : undefined),
    enabled: !!organizationId,
    staleTime: 5 * 60 * 1000, // 5 minutes
    placeholderData: keepPreviousData,
  });
}

// Hook to get a single pipeline by type (for board pages)
export function useCrmPipelineByType(organizationId: string, pipelineType: PipelineType) {
  const { data: pipelines, ...rest } = useCrmPipelines(organizationId, pipelineType);
  return {
    ...rest,
    data: pipelines?.[0],
  };
}

// Hook to get pipeline with stages
export function useCrmPipeline(pipelineId: string | undefined) {
  return useQuery({
    queryKey: crmQueryKeys.pipeline(pipelineId || ''),
    queryFn: () => crmPipelinesApi.getPipeline(pipelineId!),
    enabled: !!pipelineId,
    staleTime: 5 * 60 * 1000,
    placeholderData: keepPreviousData,
  });
}

// Hook to get Kanban board data
export function useCrmKanban(pipelineId: string | undefined) {
  return useQuery({
    queryKey: crmQueryKeys.kanban(pipelineId || ''),
    queryFn: () => crmDealsApi.getKanbanData(pipelineId!),
    enabled: !!pipelineId,
    staleTime: 30 * 1000, // 30 seconds - more frequent updates for kanban
    placeholderData: keepPreviousData,
  });
}

// ── Org-scoped hooks ──

// Re-export for backward compatibility — prefer importing from @/lib/query-keys directly
export const orgCrmQueryKeys = {
  pipelines: crmKeys.orgPipelines,
  pipelinesByType: crmKeys.orgPipelinesByType,
  kanban: crmKeys.orgKanban,
};

// Hook to list all pipelines for an organization
export function useOrgCrmPipelines(orgId: string, pipelineType?: PipelineType) {
  return useQuery({
    queryKey: pipelineType
      ? orgCrmQueryKeys.pipelinesByType(orgId, pipelineType)
      : orgCrmQueryKeys.pipelines(orgId),
    queryFn: () => crmPipelinesApi.listOrgPipelines(orgId),
    enabled: !!orgId,
    staleTime: 5 * 60 * 1000,
    placeholderData: keepPreviousData,
    select: pipelineType
      ? (data) => data.filter((p) => p.pipeline_type === pipelineType)
      : undefined,
  });
}

// Hook to get first pipeline of a given type across the org
export function useOrgCrmPipelineByType(orgId: string, pipelineType: PipelineType) {
  const { data: pipelines, ...rest } = useOrgCrmPipelines(orgId, pipelineType);
  return {
    ...rest,
    data: pipelines?.[0],
  };
}

// Hook to get org-aggregated Kanban board data
export function useOrgCrmKanban(orgId: string, pipelineId: string | undefined) {
  return useQuery({
    queryKey: orgCrmQueryKeys.kanban(orgId, pipelineId || ''),
    queryFn: () => crmDealsApi.getOrgKanbanData(orgId, pipelineId!),
    enabled: !!orgId && !!pipelineId,
    staleTime: 30 * 1000,
    placeholderData: keepPreviousData,
  });
}

// Hook to create a deal
export function useCreateDeal() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (data: CreateCrmDeal) => crmDealsApi.createDeal(data),
    onSuccess: (deal) => {
      // Invalidate kanban data for both project-scoped and org-scoped queries
      if (deal.crm_pipeline_id) {
        queryClient.invalidateQueries({
          queryKey: crmQueryKeys.kanban(deal.crm_pipeline_id),
        });
        if (deal.organization_id) {
          queryClient.invalidateQueries({
            queryKey: orgCrmQueryKeys.kanban(deal.organization_id, deal.crm_pipeline_id),
          });
        }
      }
      // Invalidate deals list
      queryClient.invalidateQueries({
        queryKey: crmQueryKeys.deals(deal.organization_id),
      });
    },
  });
}

// Hook to update a deal
export function useUpdateDeal() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateCrmDeal }) =>
      crmDealsApi.updateDeal(id, data),
    onSuccess: (deal) => {
      // Invalidate deal cache
      queryClient.invalidateQueries({
        queryKey: crmQueryKeys.deal(deal.id),
      });
      // Invalidate kanban data for both project-scoped and org-scoped queries
      if (deal.crm_pipeline_id) {
        queryClient.invalidateQueries({
          queryKey: crmQueryKeys.kanban(deal.crm_pipeline_id),
        });
        if (deal.organization_id) {
          queryClient.invalidateQueries({
            queryKey: orgCrmQueryKeys.kanban(deal.organization_id, deal.crm_pipeline_id),
          });
        }
      }
    },
  });
}

// Hook to move a deal (drag-drop)
export function useMoveDeal() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ dealId, data }: { dealId: string; data: MoveDealRequest }) =>
      crmDealsApi.moveDeal(dealId, data),
    onMutate: async ({ dealId, data }) => {
      // Get the pipeline ID from the current kanban cache
      const cacheEntries = queryClient.getQueriesData({
        queryKey: crmKeys.kanbanAll(),
      });

      // Find which pipeline this deal belongs to and optimistically update
      for (const [queryKey, kanbanData] of cacheEntries) {
        if (!kanbanData) continue;
        const typed = kanbanData as import('@/types/crm').KanbanBoardData;

        // Find the deal in any stage
        let foundDeal: import('@/types/crm').CrmDealWithContact | undefined;
        let sourceStageId: string | undefined;

        for (const stageData of typed.stages) {
          const dealIndex = stageData.deals.findIndex((d) => d.id === dealId);
          if (dealIndex >= 0) {
            foundDeal = stageData.deals[dealIndex];
            sourceStageId = stageData.stage.id;
            break;
          }
        }

        if (foundDeal && sourceStageId) {
          // Cancel any outgoing refetches
          await queryClient.cancelQueries({ queryKey });

          // Snapshot the previous value
          const previousData = queryClient.getQueryData(queryKey);

          // Optimistically update
          queryClient.setQueryData(queryKey, (old: import('@/types/crm').KanbanBoardData | undefined) => {
            if (!old) return old;

            const newStages = old.stages.map((stageData) => {
              const deals = [...stageData.deals];

              // Remove from source stage
              if (stageData.stage.id === sourceStageId) {
                const idx = deals.findIndex((d) => d.id === dealId);
                if (idx >= 0) deals.splice(idx, 1);
              }

              // Add to target stage
              if (stageData.stage.id === data.stage_id) {
                const updatedDeal = {
                  ...foundDeal!,
                  crm_stage_id: data.stage_id,
                  position: data.position,
                };
                deals.splice(data.position, 0, updatedDeal);
                // Update positions for other deals
                deals.forEach((d, i) => {
                  d.position = i;
                });
              }

              // Recalculate total amount
              const total_amount = deals.reduce((sum, d) => sum + (d.amount ?? 0), 0);

              return { ...stageData, deals, total_amount };
            });

            return { ...old, stages: newStages };
          });

          return { previousData, queryKey };
        }
      }
    },
    onError: (_err, _vars, context) => {
      // Rollback on error
      if (context?.previousData) {
        queryClient.setQueryData(context.queryKey, context.previousData);
      }
    },
    onSettled: (_data, _err, _vars, context) => {
      // Always refetch after mutation settles
      if (context?.queryKey) {
        queryClient.invalidateQueries({ queryKey: context.queryKey });
      }
    },
  });
}

// Hook to delete a deal
export function useDeleteDeal() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => crmDealsApi.deleteDeal(id),
    onSuccess: () => {
      // Invalidate all kanban queries (both project-scoped and org-scoped)
      queryClient.invalidateQueries({ queryKey: crmKeys.kanbanAll() });
      queryClient.invalidateQueries({ queryKey: crmKeys.orgKanbanAll() });
      queryClient.invalidateQueries({ queryKey: crmKeys.dealsAll() });
    },
  });
}

// Hook to fetch projects by client ID
export function useClientProjects(clientId?: string) {
  return useQuery({
    queryKey: projectKeys.byClient(clientId),
    queryFn: () => projectsApi.getByClientId(clientId!),
    enabled: !!clientId,
    staleTime: 5 * 60 * 1000,
  });
}

// Hook to resolve a deal's client and fetch their projects
export function useDealClient(orgId?: string, deal?: CrmDealWithContact | null) {
  // Fetch org clients
  const { data: clients } = useQuery({
    queryKey: organizationKeys.orgClients(orgId),
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!orgId && !!deal,
    staleTime: 5 * 60 * 1000,
  });

  // Match client by company name
  const matchedClient = clients?.find(
    (c) => deal?.contact_company && c.name.toLowerCase() === deal.contact_company.toLowerCase()
  );

  // Fetch projects for the matched client
  const { data: projects, isLoading: isProjectsLoading } = useClientProjects(matchedClient?.id);

  return {
    client: matchedClient ?? null,
    projects: projects ?? [],
    isLoading: isProjectsLoading,
  };
}
