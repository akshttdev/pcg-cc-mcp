import { useQuery } from '@tanstack/react-query';
import { systemApi } from '@/lib/api';
import { orchaKeys } from '@/lib/query-keys';

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

export function useOrchaStatus() {
  return useQuery({
    queryKey: orchaKeys.status(),
    queryFn: () => systemApi.getOrchaStatus(),
    staleTime: 5 * 60 * 1000, // 5 minutes
    retry: 1,
  });
}
