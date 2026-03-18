// ─── RunsTab ──────────────────────────────────────────────────────────────────
//
// Workflow Runs tab: history of workflow executions with token usage, cost, and output metrics.

import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Activity, Database, ClipboardCheck } from 'lucide-react';
import { workflowsApi, dataSourcesApi } from '@/lib/api';
import { workflowKeys } from '@/lib/query-keys';

export function RunsTab() {
  const navigate = useNavigate();

  const { data: recentRuns = [], isLoading } = useQuery({
    queryKey: workflowKeys.recentRuns(),
    queryFn: () => workflowsApi.listRecentRuns({ limit: 50 }),
    refetchInterval: 10000,
  });

  const runDsIds = useMemo(
    () => [...new Set(recentRuns.map((r) => r.data_source_id).filter(Boolean))] as string[],
    [recentRuns],
  );
  const { data: dsNames = {} } = useQuery({
    queryKey: workflowKeys.dataSourceNamesForRunsTab(runDsIds),
    queryFn: async () => {
      const out: Record<string, string> = {};
      await Promise.all(runDsIds.map(async (id) => {
        try { out[id] = (await dataSourcesApi.get(id)).title; } catch { out[id] = '(Deleted source)'; }
      }));
      return out;
    },
    enabled: runDsIds.length > 0,
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading workflow runs...</div>
      </div>
    );
  }

  const formatDurationShort = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
  };

  return (
    <div className="space-y-4 max-w-[1200px] mx-auto">
      <div>
        <h2 className="text-lg font-semibold">Workflow Runs</h2>
        <p className="text-sm text-muted-foreground">
          History of workflow executions with token usage, cost, and output metrics.
        </p>
      </div>

      {recentRuns.length === 0 ? (
        <div className="text-center py-16 text-muted-foreground">
          <Activity className="h-10 w-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm font-medium">No workflow runs yet</p>
          <p className="text-xs mt-1">Run a workflow from a data source to see execution history here.</p>
        </div>
      ) : (
        <div className="space-y-2">
          {recentRuns.map((run) => (
            <Card
              key={run.id}
              className={`border-l-4 ${run.status === 'completed' ? 'border-l-green-500' : run.status === 'failed' ? 'border-l-red-500' : 'border-l-yellow-500'}`}
            >
              <CardContent className="py-3 px-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Badge variant={run.status === 'completed' ? 'default' : run.status === 'failed' ? 'destructive' : 'outline'} className="text-[10px] capitalize">
                      {run.status}
                    </Badge>
                    <span className="text-sm font-medium">{run.workflow_name || run.workflow_id}</span>
                    {run.data_source_id && dsNames[run.data_source_id] && (
                      <span
                        className={`text-xs text-muted-foreground flex items-center gap-1 ${dsNames[run.data_source_id] === '(Deleted source)' ? 'italic opacity-60' : ''}`}
                        title={dsNames[run.data_source_id] === '(Deleted source)' ? run.data_source_id : undefined}
                      >
                        <Database className="h-3 w-3" />
                        {dsNames[run.data_source_id]}
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-4 text-xs text-muted-foreground">
                    {run.duration_ms && <span>{formatDurationShort(run.duration_ms)}</span>}
                    {run.total_estimated_cost_micros != null && <span>${(run.total_estimated_cost_micros / 1_000_000).toFixed(4)}</span>}
                    {run.total_records_staged != null && (
                      <span className={run.total_records_staged > 0 ? 'text-foreground font-medium' : ''}>
                        {run.total_records_staged} records
                      </span>
                    )}
                    <span>{new Date(run.started_at).toLocaleString()}</span>
                    {(run.total_records_staged ?? 0) > 0 && (
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-6 text-[10px] gap-1 ml-1"
                        onClick={() => navigate(`/workflows?tab=staging&run=${run.id}`)}
                      >
                        <ClipboardCheck className="h-3 w-3" />
                        Review Staging
                      </Button>
                    )}
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}
