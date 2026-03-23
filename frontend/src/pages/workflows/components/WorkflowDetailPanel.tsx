// ─── WorkflowDetailPanel ──────────────────────────────────────────────────────
//
// Inline detail panel shown below the workflow grid when a workflow card is clicked.

import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Play, Pencil, Zap, ArrowRight, Database } from 'lucide-react';
import { workflowsApi, dataSourcesApi, stagingApi } from '@/lib/api';
import { workflowKeys } from '@/lib/query-keys';
import type { WorkflowDefinition, WorkflowStagingRecord, WorkflowRun, WorkflowNode } from '@/lib/api';
import { useAuth } from '@/contexts/AuthContext';

interface WorkflowDetailPanelProps {
  workflow: WorkflowDefinition;
  onEdit: () => void;
  onRun: () => void;
  onViewTriggers: () => void;
  onClose: () => void;
}

export function WorkflowDetailPanel({
  workflow,
  onEdit,
  onRun,
  onViewTriggers,
  onClose,
}: WorkflowDetailPanelProps) {
  const navigate = useNavigate();
  const [detailTab, setDetailTab] = useState<'overview' | 'runs' | 'staging'>('overview');

  const { data: recentRuns = [] } = useQuery({
    queryKey: workflowKeys.runsByWorkflow(workflow.id),
    queryFn: () => workflowsApi.listRecentRuns({ workflow_id: workflow.id, limit: 10 }),
    refetchInterval: 15000,
  });

  // Resolve data source names for runs that have data_source_id
  const dsIds = useMemo(
    () => [...new Set(recentRuns.map((r: WorkflowRun) => r.data_source_id).filter(Boolean))] as string[],
    [recentRuns],
  );
  const { data: dsNames = {} } = useQuery({
    queryKey: workflowKeys.dataSourceNamesForRuns(dsIds),
    queryFn: async () => {
      const out: Record<string, string> = {};
      await Promise.all(dsIds.map(async (id) => {
        try { out[id] = (await dataSourcesApi.get(id)).title; } catch { out[id] = id.slice(0, 8) + '\u2026'; }
      }));
      return out;
    },
    enabled: dsIds.length > 0,
  });

  const { user } = useAuth();
  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;
  const { data: pendingRecords = [] } = useQuery({
    queryKey: workflowKeys.stagingPendingByWorkflow(workflow.id),
    queryFn: () => stagingApi.listPending(orgId!),
    enabled: !!orgId,
  });

  // Filter pending records to this workflow's runs
  const workflowPending = useMemo(() => {
    const runIds = new Set(recentRuns.map((r: WorkflowRun) => r.id));
    return pendingRecords.filter((r) => runIds.has(r.workflow_run_id));
  }, [pendingRecords, recentRuns]);

  const pendingByRun = useMemo(() => {
    const byRun: Record<string, WorkflowStagingRecord[]> = {};
    for (const r of workflowPending) {
      if (!byRun[r.workflow_run_id]) byRun[r.workflow_run_id] = [];
      byRun[r.workflow_run_id].push(r);
    }
    return byRun;
  }, [workflowPending]);

  const formatDurationShort = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
  };

  const nodeCount = workflow.nodes?.length ?? 0;
  const outputNodes = (workflow.nodes ?? []).filter((n: WorkflowNode) =>
    ['output_crm_contacts', 'output_crm_companies', 'output_crm_deals', 'output_tasks'].includes(n.type)
  );

  const totalRuns = recentRuns.length;
  const successRuns = recentRuns.filter((r: WorkflowRun) => r.status === 'completed').length;
  const totalRecordsStaged = recentRuns.reduce((sum: number, r: WorkflowRun) => sum + (r.total_records_staged || 0), 0);

  return (
    <Card className="border-primary/30 bg-card/80">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <CardTitle className="text-base">{workflow.name}</CardTitle>
            {workflow.is_system && <Badge variant="secondary" className="text-xs">System</Badge>}
          </div>
          <div className="flex items-center gap-1.5">
            <Button size="sm" variant="outline" className="gap-1.5 h-7 text-xs" onClick={onRun}>
              <Play className="h-3 w-3" />
              Run
            </Button>
            <Button size="sm" variant="outline" className="gap-1.5 h-7 text-xs" onClick={onEdit}>
              <Pencil className="h-3 w-3" />
              Edit
            </Button>
            <Button size="sm" variant="outline" className="gap-1.5 h-7 text-xs" onClick={onViewTriggers}>
              <Zap className="h-3 w-3" />
              Triggers
            </Button>
            <Button size="sm" variant="ghost" className="h-7 w-7 p-0" onClick={onClose}>
              <span className="sr-only">Close</span>
              &times;
            </Button>
          </div>
        </div>
        {workflow.description && (
          <p className="text-sm text-muted-foreground mt-1">{workflow.description}</p>
        )}
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Quick stats */}
        <div className="grid grid-cols-4 gap-3">
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold">{nodeCount}</div>
            <div className="text-xs text-muted-foreground uppercase tracking-wide">Nodes</div>
          </div>
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold">{totalRuns}</div>
            <div className="text-xs text-muted-foreground uppercase tracking-wide">Runs</div>
          </div>
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold text-green-600">{successRuns}</div>
            <div className="text-xs text-muted-foreground uppercase tracking-wide">Successful</div>
          </div>
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold">{totalRecordsStaged}</div>
            <div className="text-xs text-muted-foreground uppercase tracking-wide">Records</div>
          </div>
        </div>

        {/* Output types */}
        {outputNodes.length > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Outputs:</span>
            {outputNodes.map((node: WorkflowNode) => {
              const outputLabels: Record<string, string> = {
                output_crm_contacts: 'Contacts',
                output_crm_companies: 'Companies',
                output_crm_deals: 'Deals',
                output_tasks: 'Tasks',
              };
              return (
                <Badge key={node.id} variant="secondary" className="text-xs">
                  {outputLabels[node.type] ?? node.type}
                </Badge>
              );
            })}
          </div>
        )}

        {/* Sub-tabs: Overview / Runs / Staging */}
        <div className="border-t pt-3">
          <div className="flex gap-1 mb-3">
            {(['overview', 'runs', 'staging'] as const).map((tab) => (
              <Button
                key={tab}
                size="sm"
                variant={detailTab === tab ? 'default' : 'ghost'}
                className="h-7 text-xs capitalize"
                onClick={() => setDetailTab(tab)}
              >
                {tab}
                {tab === 'staging' && workflowPending.length > 0 && (
                  <Badge variant="secondary" className="ml-1 text-[9px] h-4 px-1">{workflowPending.length}</Badge>
                )}
                {tab === 'runs' && totalRuns > 0 && (
                  <Badge variant="secondary" className="ml-1 text-[9px] h-4 px-1">{totalRuns}</Badge>
                )}
              </Button>
            ))}
          </div>

          {detailTab === 'overview' && (
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Pipeline</p>
              <div className="flex flex-wrap items-center gap-1.5">
                {(workflow.nodes ?? []).map((node: WorkflowNode, idx: number) => (
                    <div key={node.id} className="flex items-center gap-1.5">
                      {idx > 0 && <ArrowRight className="h-3 w-3 text-muted-foreground" />}
                      <Badge variant="outline" className="text-xs">
                        {node.name}
                      </Badge>
                    </div>
                  ))}
              </div>
            </div>
          )}

          {detailTab === 'runs' && (
            <div className="space-y-1.5 max-h-48 overflow-auto">
              {recentRuns.length === 0 ? (
                <p className="text-xs text-muted-foreground py-4 text-center">No runs yet. Click "Run" to execute this workflow.</p>
              ) : (
                recentRuns.map((run: WorkflowRun) => (
                  <div
                    key={run.id}
                    className={`flex items-center justify-between p-2 rounded-md border text-xs cursor-pointer hover:bg-muted/50 ${
                      run.status === 'completed' ? 'border-l-2 border-l-green-500' : run.status === 'failed' ? 'border-l-2 border-l-red-500' : 'border-l-2 border-l-yellow-500'
                    }`}
                    onClick={() => {
                      // API returns `total_records_staged` (not `records_staged`) per WorkflowRun type in shared/types.ts
                      if ((run.total_records_staged ?? 0) > 0) navigate(`/workflows?tab=staging&run=${run.id}`);
                    }}
                  >
                    <div className="flex items-center gap-2">
                      <Badge variant={run.status === 'completed' ? 'default' : run.status === 'failed' ? 'destructive' : 'outline'} className="text-[9px] capitalize">
                        {run.status}
                      </Badge>
                      <span className="text-muted-foreground">{formatDurationShort(run.duration_ms)}</span>
                      {(run.total_records_staged ?? 0) > 0 && (
                        <span>{run.total_records_staged} records</span>
                      )}
                      {run.data_source_id && dsNames[run.data_source_id] && (
                        <span className="text-muted-foreground flex items-center gap-0.5">
                          <Database className="h-2.5 w-2.5" />
                          {dsNames[run.data_source_id]}
                        </span>
                      )}
                    </div>
                    <span className="text-muted-foreground">
                      {new Date(run.started_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                    </span>
                  </div>
                ))
              )}
            </div>
          )}

          {detailTab === 'staging' && (
            <div className="space-y-1.5 max-h-48 overflow-auto">
              {workflowPending.length === 0 ? (
                <p className="text-xs text-muted-foreground py-4 text-center">No pending records for this workflow.</p>
              ) : (
                Object.entries(pendingByRun).map(([runId, records]) => (
                  <div
                    key={runId}
                    className="flex items-center justify-between p-2 rounded-md border text-xs cursor-pointer hover:bg-muted/50"
                    onClick={() => navigate(`/workflows?tab=staging&run=${runId}`)}
                  >
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-[9px]">Pending</Badge>
                      <span>{records.length} record{records.length !== 1 ? 's' : ''}</span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      {(() => {
                        const types: Record<string, number> = {};
                        for (const r of records) types[r.target_type] = (types[r.target_type] || 0) + 1;
                        return Object.entries(types).map(([t, c]) => (
                          <Badge key={t} variant="secondary" className="text-[9px]">{c} {t.replace('crm_', '')}</Badge>
                        ));
                      })()}
                    </div>
                  </div>
                ))
              )}
            </div>
          )}
        </div>
      </CardContent>

    </Card>
  );
}
