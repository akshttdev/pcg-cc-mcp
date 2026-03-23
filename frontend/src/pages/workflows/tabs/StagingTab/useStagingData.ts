import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { workflowKeys } from '@/lib/query-keys';
import { workflowsApi, stagingApi, schemasApi } from '@/lib/api';
import type { WorkflowStagingRecord, TargetSchema } from '@/lib/api';

export function useStagingData(orgId: string | undefined) {
  const { data: pendingRecords = [], isLoading } = useQuery({
    queryKey: workflowKeys.stagingPendingOrg(orgId),
    queryFn: () => stagingApi.listPending(orgId!),
    enabled: !!orgId,
    refetchInterval: 15000,
  });

  const { data: recentRuns = [] } = useQuery({
    queryKey: workflowKeys.recentRuns(),
    queryFn: () => workflowsApi.listRecentRuns({ limit: 100 }),
    staleTime: 30000,
  });

  const runNameMap = useMemo(() => {
    const m: Record<string, string> = {};
    for (const r of recentRuns) {
      if (r.id && r.workflow_name) m[r.id] = r.workflow_name;
    }
    return m;
  }, [recentRuns]);

  const targetTypes = useMemo(() => [...new Set(pendingRecords.map(r => r.target_type))], [pendingRecords]);

  const { data: schemasMap = {} } = useQuery({
    queryKey: workflowKeys.schemas(targetTypes),
    queryFn: async () => {
      const entries = await Promise.all(
        targetTypes.map(async (tt) => {
          try {
            const schema = await schemasApi.get(tt);
            return [tt, schema] as [string, TargetSchema];
          } catch { return null; }
        })
      );
      return Object.fromEntries(entries.filter(Boolean) as [string, TargetSchema][]);
    },
    enabled: targetTypes.length > 0,
    staleTime: 60000,
  });

  const grouped = useMemo(() => {
    const byRun: Record<string, { runId: string; records: WorkflowStagingRecord[]; workflowName?: string }> = {};
    for (const r of pendingRecords) {
      const rid = r.workflow_run_id;
      if (!byRun[rid]) byRun[rid] = { runId: rid, records: [], workflowName: runNameMap[rid] };
      byRun[rid].records.push(r);
    }
    return Object.values(byRun);
  }, [pendingRecords, runNameMap]);

  const totalPending = pendingRecords.filter(r => r.status === 'pending_review').length;
  const totalApproved = pendingRecords.filter(r => r.status === 'approved').length;
  const totalDuplicates = pendingRecords.filter(r => r.duplicate_of_id != null).length;
  const pendingNonDuplicate = useMemo(
    () => pendingRecords.filter(r => r.status === 'pending_review' && !r.duplicate_of_id),
    [pendingRecords]
  );

  const totalRejected = pendingRecords.filter(r => r.status === 'rejected').length;
  const totalError = pendingRecords.filter(r => r.status === 'error').length;
  const totalWithIssues = useMemo(() => pendingRecords.filter(r => {
    if (!r.validation_errors) return false;
    try { const e = JSON.parse(r.validation_errors); return Array.isArray(e) && e.length > 0; } catch { return false; }
  }).length, [pendingRecords]);

  const uniqueTypes = useMemo(() => {
    const types = new Set(pendingRecords.map(r => r.target_type));
    return Array.from(types);
  }, [pendingRecords]);

  const uniqueWorkflows = useMemo(() => {
    const wfs: Record<string, string> = {};
    for (const r of pendingRecords) {
      if (!wfs[r.workflow_run_id]) wfs[r.workflow_run_id] = runNameMap[r.workflow_run_id] || r.workflow_run_id.slice(0, 8);
    }
    return Object.entries(wfs);
  }, [pendingRecords, runNameMap]);

  const commonWarnings = useMemo(() => {
    const warnCounts: Record<string, number> = {};
    for (const r of pendingRecords) {
      if (!r.validation_errors) continue;
      try {
        const errs = JSON.parse(r.validation_errors);
        if (Array.isArray(errs)) {
          for (const e of errs) warnCounts[e] = (warnCounts[e] || 0) + 1;
        }
      } catch { /* intentionally empty */ }
    }
    const threshold = Math.max(2, Math.floor(pendingRecords.length * 0.5));
    return Object.entries(warnCounts)
      .filter(([, count]) => count >= threshold)
      .map(([msg, count]) => ({ msg, count }));
  }, [pendingRecords]);

  const commonWarningSet = useMemo(() => new Set(commonWarnings.map(w => w.msg)), [commonWarnings]);

  return {
    pendingRecords,
    isLoading,
    runNameMap,
    schemasMap,
    grouped,
    totalPending,
    totalApproved,
    totalDuplicates,
    pendingNonDuplicate,
    totalRejected,
    totalError,
    totalWithIssues,
    uniqueTypes,
    uniqueWorkflows,
    commonWarnings,
    commonWarningSet,
  };
}
