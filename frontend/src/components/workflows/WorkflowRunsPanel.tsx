import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Activity,
  Clock,
  Coins,
  Cpu,
  Database,
  Hash,
  Loader2,
  CheckCircle2,
  XCircle,
  ChevronRight,
  Zap,
  Copy,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { workflowsApi, dataSourcesApi } from '@/lib/api';
import type { WorkflowRun } from '@/lib/api';
import { StagingReviewPanel } from './StagingReviewPanel';

interface WorkflowRunsPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workflowId?: string;
  organizationId?: string;
}

function formatDuration(ms: number | undefined): string {
  if (!ms) return '-';
  if (ms < 1000) return `${ms}ms`;
  if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
}

function formatCost(micros: number | undefined): string {
  if (!micros) return '$0.00';
  return `$${(micros / 1_000_000).toFixed(4)}`;
}

function formatTokens(count: number | undefined): string {
  if (!count) return '0';
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
  return String(count);
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr + 'Z');
  const now = new Date();
  const diffMs = now.getTime() - d.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  if (diffMins < 1) return 'just now';
  if (diffMins < 60) return `${diffMins}m ago`;
  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 7) return `${diffDays}d ago`;
  return d.toLocaleDateString();
}

const STATUS_CONFIG = {
  running: { label: 'Running', icon: Loader2, color: 'text-blue-500', bg: 'bg-blue-50 border-blue-200', iconClass: 'animate-spin' },
  completed: { label: 'Completed', icon: CheckCircle2, color: 'text-green-500', bg: 'bg-green-50 border-green-200', iconClass: '' },
  failed: { label: 'Failed', icon: XCircle, color: 'text-red-500', bg: 'bg-red-50 border-red-200', iconClass: '' },
} as const;

export function WorkflowRunsPanel({
  open,
  onOpenChange,
  workflowId,
  organizationId,
}: WorkflowRunsPanelProps) {
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const [selectedRunName, setSelectedRunName] = useState<string>('');

  const { data: runs = [], isLoading } = useQuery({
    queryKey: ['workflow-runs', workflowId, organizationId],
    queryFn: () => workflowsApi.listRecentRuns({ workflow_id: workflowId, organization_id: organizationId }),
    enabled: open,
    refetchInterval: 10000,
  });

  // Collect unique data_source_ids from runs to batch-resolve names
  const dataSourceIds = useMemo(
    () => [...new Set(runs.map(r => r.data_source_id).filter(Boolean))] as string[],
    [runs],
  );

  const { data: dataSourceNames = {} } = useQuery({
    queryKey: ['data-source-names', dataSourceIds],
    queryFn: async () => {
      const results: Record<string, string> = {};
      await Promise.all(
        dataSourceIds.map(async (id) => {
          try {
            const ds = await dataSourcesApi.get(id);
            results[id] = ds.title;
          } catch {
            results[id] = id.slice(0, 8) + '…';
          }
        }),
      );
      return results;
    },
    enabled: open && dataSourceIds.length > 0,
  });

  // Aggregate stats
  const totalRuns = runs.length;
  const totalCostMicros = runs.reduce((sum, r) => sum + (r.total_estimated_cost_micros || 0), 0);
  const totalRecords = runs.reduce((sum, r) => sum + (r.total_records_staged || 0), 0);
  const totalDuplicates = runs.reduce((sum, r) => sum + (r.total_duplicates_found || 0), 0);
  const avgDuration = totalRuns > 0
    ? runs.reduce((sum, r) => sum + (r.duration_ms || 0), 0) / totalRuns
    : 0;
  const duplicateRate = totalRecords > 0 ? totalDuplicates / totalRecords : 0;

  const handleRunClick = (run: WorkflowRun) => {
    setSelectedRunId(run.id);
    setSelectedRunName(run.workflow_name);
  };

  return (
    <>
      <Dialog open={open} onOpenChange={onOpenChange}>
        <DialogContent className="max-w-[1000px] w-full max-h-[85vh] p-0 flex flex-col overflow-hidden">
          {/* Header */}
          <div className="px-6 py-4 border-b">
            <DialogTitle className="text-lg">Workflow Run History</DialogTitle>
            <p className="text-sm text-muted-foreground mt-0.5">
              {workflowId ? 'Runs for this workflow' : 'All recent workflow runs'}
            </p>
          </div>

          {/* Aggregate stats bar */}
          {runs.length > 0 && (
            <div className="grid grid-cols-5 gap-3 px-6 py-3 border-b bg-muted/30">
              <div className="text-center">
                <div className="text-lg font-semibold">{totalRuns}</div>
                <div className="text-[10px] text-muted-foreground uppercase tracking-wider">Total Runs</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-semibold">{formatCost(totalCostMicros)}</div>
                <div className="text-[10px] text-muted-foreground uppercase tracking-wider">Total Cost</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-semibold">{totalRecords}</div>
                <div className="text-[10px] text-muted-foreground uppercase tracking-wider">Records Staged</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-semibold">{(duplicateRate * 100).toFixed(0)}%</div>
                <div className="text-[10px] text-muted-foreground uppercase tracking-wider">Duplicate Rate</div>
              </div>
              <div className="text-center">
                <div className="text-lg font-semibold">{formatDuration(Math.round(avgDuration))}</div>
                <div className="text-[10px] text-muted-foreground uppercase tracking-wider">Avg Duration</div>
              </div>
            </div>
          )}

          {/* Runs list */}
          <ScrollArea className="flex-1">
            <div className="p-4 space-y-2">
              {isLoading && (
                <div className="text-center py-12 text-muted-foreground">
                  <Loader2 className="h-6 w-6 mx-auto mb-2 animate-spin" />
                  <p className="text-sm">Loading workflow runs...</p>
                </div>
              )}

              {!isLoading && runs.length === 0 && (
                <div className="text-center py-12 text-muted-foreground">
                  <Activity className="h-8 w-8 mx-auto mb-2 opacity-40" />
                  <p className="text-sm">No workflow runs yet.</p>
                  <p className="text-xs mt-1">Run a workflow to see metrics here.</p>
                </div>
              )}

              {runs.map((run) => {
                const statusConfig = STATUS_CONFIG[run.status as keyof typeof STATUS_CONFIG] || STATUS_CONFIG.completed;
                const StatusIcon = statusConfig.icon;
                const totalTokens = (run.total_input_tokens || 0) + (run.total_output_tokens || 0);

                return (
                  <button
                    key={run.id}
                    onClick={() => handleRunClick(run)}
                    className={cn(
                      'w-full text-left rounded-lg border p-3 transition-colors hover:bg-muted/50',
                      'focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-1',
                    )}
                  >
                    <div className="flex items-center gap-3">
                      {/* Status icon */}
                      <StatusIcon className={cn('h-4 w-4 shrink-0', statusConfig.color, statusConfig.iconClass)} />

                      {/* Name and date */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium truncate">{run.workflow_name}</span>
                          <Badge
                            variant="outline"
                            className={cn('text-[10px] shrink-0', statusConfig.bg, statusConfig.color)}
                          >
                            {statusConfig.label}
                          </Badge>
                        </div>
                        <div className="flex items-center gap-3 mt-1 text-[11px] text-muted-foreground">
                          <span className="flex items-center gap-1">
                            <Clock className="h-3 w-3" />
                            {formatDate(run.started_at)}
                          </span>
                          {run.data_source_id && dataSourceNames[run.data_source_id] && (
                            <span className="flex items-center gap-1 truncate max-w-[180px]" title={dataSourceNames[run.data_source_id]}>
                              <Database className="h-3 w-3" />
                              {dataSourceNames[run.data_source_id]}
                            </span>
                          )}
                          {run.model_used && (
                            <span className="flex items-center gap-1">
                              <Cpu className="h-3 w-3" />
                              {run.model_used.split('/').pop()}
                            </span>
                          )}
                        </div>
                      </div>

                      {/* Metrics */}
                      <div className="flex items-center gap-4 text-xs text-muted-foreground shrink-0">
                        {totalTokens > 0 && (
                          <span className="flex items-center gap-1" title="Total tokens">
                            <Zap className="h-3 w-3" />
                            {formatTokens(totalTokens)}
                          </span>
                        )}
                        {(run.total_estimated_cost_micros || 0) > 0 && (
                          <span className="flex items-center gap-1" title="Cost">
                            <Coins className="h-3 w-3" />
                            {formatCost(run.total_estimated_cost_micros)}
                          </span>
                        )}
                        {(run.total_records_staged || 0) > 0 && (
                          <span className="flex items-center gap-1" title="Records staged">
                            <Hash className="h-3 w-3" />
                            {run.total_records_staged}
                            {(run.total_duplicates_found || 0) > 0 && (
                              <span className="text-amber-500 flex items-center gap-0.5" title="Duplicates found">
                                <Copy className="h-2.5 w-2.5" />
                                {run.total_duplicates_found}
                              </span>
                            )}
                          </span>
                        )}
                        {run.duration_ms != null && (
                          <span className="flex items-center gap-1 w-14 text-right" title="Duration">
                            <Clock className="h-3 w-3" />
                            {formatDuration(run.duration_ms)}
                          </span>
                        )}
                        <ChevronRight className="h-4 w-4 text-muted-foreground/50" />
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </ScrollArea>
        </DialogContent>
      </Dialog>

      {/* Staging review panel for selected run */}
      {selectedRunId && (
        <StagingReviewPanel
          open={!!selectedRunId}
          onOpenChange={(open) => { if (!open) setSelectedRunId(null); }}
          workflowRunId={selectedRunId}
          workflowName={selectedRunName}
        />
      )}
    </>
  );
}
