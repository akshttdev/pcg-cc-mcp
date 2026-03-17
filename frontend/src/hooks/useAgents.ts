import { useQuery } from '@tanstack/react-query';
import { agentsApi } from '@/lib/api';
import type { CreateAgent, UpdateAgent, AgentStatus } from 'shared/types';
import { agentKeys } from '@/lib/query-keys';
import { useMutationWithToast } from './useMutationWithToast';

export function useAgents() {
  return useQuery({
    queryKey: agentKeys.all,
    queryFn: agentsApi.list,
    staleTime: 1000 * 60,
  });
}

export function useActiveAgents() {
  return useQuery({
    queryKey: agentKeys.active(),
    queryFn: agentsApi.listActive,
    staleTime: 1000 * 60,
  });
}

export function useAgent(agentId: string | undefined) {
  return useQuery({
    queryKey: agentKeys.detail(agentId!),
    queryFn: () => agentsApi.getById(agentId!),
    enabled: !!agentId,
    staleTime: 1000 * 60,
  });
}

export function useAgentByName(name: string | undefined) {
  return useQuery({
    queryKey: agentKeys.byName(name!),
    queryFn: () => agentsApi.getByName(name!),
    enabled: !!name,
    staleTime: 1000 * 60,
  });
}

export function useCreateAgent() {
  return useMutationWithToast({
    mutationFn: (agent: CreateAgent) => agentsApi.create(agent),
    successMessage: 'Agent created',
    errorMessage: 'Failed to create agent',
    invalidateKeys: [agentKeys.all],
  });
}

export function useUpdateAgent() {
  return useMutationWithToast({
    mutationFn: ({ agentId, agent }: { agentId: string; agent: UpdateAgent }) =>
      agentsApi.update(agentId, agent),
    successMessage: 'Agent updated',
    errorMessage: 'Failed to update agent',
    invalidateKeys: [agentKeys.all],
  });
}

export function useDeleteAgent() {
  return useMutationWithToast({
    mutationFn: (agentId: string) => agentsApi.delete(agentId),
    successMessage: 'Agent deleted',
    errorMessage: 'Failed to delete agent',
    invalidateKeys: [agentKeys.all],
  });
}

export function useSeedCoreAgents() {
  return useMutationWithToast({
    mutationFn: () => agentsApi.seedCoreAgents(),
    successMessage: 'Core agents seeded',
    errorMessage: 'Failed to seed core agents',
    invalidateKeys: [agentKeys.all],
  });
}

export function useUpdateAgentStatus() {
  return useMutationWithToast({
    mutationFn: ({ agentId, status }: { agentId: string; status: AgentStatus }) =>
      agentsApi.updateStatus(agentId, status),
    successMessage: 'Agent status updated',
    errorMessage: 'Failed to update agent status',
    invalidateKeys: [agentKeys.all],
  });
}

export function useAssignAgentWallet() {
  return useMutationWithToast({
    mutationFn: ({ agentId, walletAddress }: { agentId: string; walletAddress: string }) =>
      agentsApi.assignWallet(agentId, walletAddress),
    successMessage: 'Wallet assigned',
    errorMessage: 'Failed to assign wallet',
    invalidateKeys: [agentKeys.all],
  });
}
