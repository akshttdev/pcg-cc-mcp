import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { agentsApi } from '@/lib/api';
import type { CreateAgent, UpdateAgent, AgentStatus } from 'shared/types';

const AGENTS_QUERY_KEY = ['agents'];

/**
 * Hook to fetch all agents
 */
export function useAgents() {
  return useQuery({
    queryKey: AGENTS_QUERY_KEY,
    queryFn: agentsApi.list,
    staleTime: 1000 * 60, // 1 minute
  });
}

/**
 * Hook to fetch only active agents
 */
export function useActiveAgents() {
  return useQuery({
    queryKey: [...AGENTS_QUERY_KEY, 'active'],
    queryFn: agentsApi.listActive,
    staleTime: 1000 * 60, // 1 minute
  });
}

/**
 * Hook to fetch a single agent by ID
 */
export function useAgent(agentId: string | undefined) {
  return useQuery({
    queryKey: [...AGENTS_QUERY_KEY, agentId],
    queryFn: () => agentsApi.getById(agentId!),
    enabled: !!agentId,
    staleTime: 1000 * 60, // 1 minute
  });
}

/**
 * Hook to fetch a single agent by name
 */
export function useAgentByName(name: string | undefined) {
  return useQuery({
    queryKey: [...AGENTS_QUERY_KEY, 'by-name', name],
    queryFn: () => agentsApi.getByName(name!),
    enabled: !!name,
    staleTime: 1000 * 60, // 1 minute
  });
}

/**
 * Hook to create a new agent
 */
export function useCreateAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (agent: CreateAgent) => agentsApi.create(agent),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AGENTS_QUERY_KEY });
    },
  });
}

/**
 * Hook to update an agent
 */
export function useUpdateAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ agentId, agent }: { agentId: string; agent: UpdateAgent }) =>
      agentsApi.update(agentId, agent),
    onSuccess: (updatedAgent) => {
      queryClient.invalidateQueries({ queryKey: AGENTS_QUERY_KEY });
      queryClient.setQueryData([...AGENTS_QUERY_KEY, updatedAgent.id], updatedAgent);
    },
  });
}

/**
 * Hook to delete an agent
 */
export function useDeleteAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (agentId: string) => agentsApi.delete(agentId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AGENTS_QUERY_KEY });
    },
  });
}

/**
 * Hook to seed core agents
 */
export function useSeedCoreAgents() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => agentsApi.seedCoreAgents(),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: AGENTS_QUERY_KEY });
    },
  });
}

/**
 * Hook to update agent status
 */
export function useUpdateAgentStatus() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ agentId, status }: { agentId: string; status: AgentStatus }) =>
      agentsApi.updateStatus(agentId, status),
    onSuccess: (updatedAgent) => {
      queryClient.invalidateQueries({ queryKey: AGENTS_QUERY_KEY });
      queryClient.setQueryData([...AGENTS_QUERY_KEY, updatedAgent.id], updatedAgent);
    },
  });
}

/**
 * Hook to assign wallet to agent
 */
export function useAssignAgentWallet() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ agentId, walletAddress }: { agentId: string; walletAddress: string }) =>
      agentsApi.assignWallet(agentId, walletAddress),
    onSuccess: (updatedAgent) => {
      queryClient.invalidateQueries({ queryKey: AGENTS_QUERY_KEY });
      queryClient.setQueryData([...AGENTS_QUERY_KEY, updatedAgent.id], updatedAgent);
    },
  });
}
