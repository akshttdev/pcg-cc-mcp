import { useQuery } from '@tanstack/react-query';

export interface SubAgentBrief {
  id: string;
  shortName: string;
  designation: string;
  status: string;
}

export interface OrchaStatus {
  orchestratorName: string;
  agentId: string;
  device: string;
  isAdmin: boolean;
  subAgents: SubAgentBrief[];
}

async function fetchOrchaStatus(): Promise<OrchaStatus | null> {
  const response = await fetch('/api/orcha/status', { credentials: 'include' });
  if (!response.ok) return null;
  return response.json();
}

export function useOrchaStatus() {
  return useQuery({
    queryKey: ['orcha-status'],
    queryFn: fetchOrchaStatus,
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 1,
  });
}
