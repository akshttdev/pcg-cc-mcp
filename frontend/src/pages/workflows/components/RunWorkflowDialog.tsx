// ─── RunWorkflowDialog ────────────────────────────────────────────────────────
//
// Dialog to select a data source and run a workflow against it.

import { useState, useMemo } from 'react';
import { useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { EmptyState } from '@/components/ui/empty-state';
import { Play, Search, Loader2, FileText, Upload, Database } from 'lucide-react';
import { dataSourcesApi, workflowsApi, DATA_TYPE_OPTIONS } from '@/lib/api';
import { workflowKeys } from '@/lib/query-keys';
import type { WorkflowDefinition } from '@/lib/api';
import { useAuth } from '@/contexts/AuthContext';

interface RunWorkflowDialogProps {
  workflow: WorkflowDefinition | null;
  onClose: () => void;
  /** When provided, called instead of navigating away after a successful run */
  onRunComplete?: (runId: string, stagedRecords: number) => void;
}

export function RunWorkflowDialog({ workflow, onClose, onRunComplete }: RunWorkflowDialogProps) {
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const { user } = useAuth();
  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;
  const [selectedDataSourceId, setSelectedDataSourceId] = useState<string>('');
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [searchFilter, setSearchFilter] = useState('');
  const [dataTypeFilter, setDataTypeFilter] = useState<string>('__all__');
  const { data: dataSources = [] } = useQuery({
    queryKey: workflowKeys.orgDataSources(orgId!),
    queryFn: () => dataSourcesApi.listByOrganization(orgId!),
    enabled: !!workflow && !!orgId,
  });

  const { data: availableModels } = useQuery({
    queryKey: workflowKeys.models(),
    queryFn: () => workflowsApi.listAvailableModels(),
    staleTime: 60 * 60 * 1000,
    enabled: !!workflow,
  });

  const effectiveModel = selectedModel || availableModels?.find((m) => m.is_default)?.id || '';

  const filteredSources = useMemo(() => {
    let result = dataSources.filter((ds) => ds.status === 'ready');
    if (searchFilter) {
      const lower = searchFilter.toLowerCase();
      result = result.filter((ds) =>
        ds.title.toLowerCase().includes(lower) ||
        ds.description?.toLowerCase().includes(lower)
      );
    }
    if (dataTypeFilter && dataTypeFilter !== '__all__') {
      result = result.filter((ds) => ds.data_type === dataTypeFilter);
    }
    return result;
  }, [dataSources, searchFilter, dataTypeFilter]);

  const runMutation = useMutation({
    mutationFn: () => dataSourcesApi.runWorkflow(selectedDataSourceId, workflow!.id, effectiveModel || undefined),
    onSuccess: (data) => {
      if (data.workflow_run_id && data.staged_records > 0) {
        queryClient.invalidateQueries({ queryKey: workflowKeys.staging(data.workflow_run_id) });
        queryClient.invalidateQueries({ queryKey: workflowKeys.stagingPending() });
        if (onRunComplete) {
          handleClose();
          onRunComplete(data.workflow_run_id, data.staged_records);
        } else {
          handleClose();
          navigate('/workflows?tab=staging&run=' + data.workflow_run_id);
        }
      }
    },
  });

  const handleClose = () => {
    setSelectedDataSourceId('');
    setSelectedModel('');
    setSearchFilter('');
    setDataTypeFilter('__all__');
    runMutation.reset();
    onClose();
  };

  const sourceTypeIcon = (st: string) => {
    if (st === 'file') return <Upload className="h-3.5 w-3.5 text-muted-foreground" />;
    if (st === 'text') return <FileText className="h-3.5 w-3.5 text-muted-foreground" />;
    return <Database className="h-3.5 w-3.5 text-muted-foreground" />;
  };

  return (
    <>
      <Dialog open={!!workflow} onOpenChange={(open) => { if (!open) handleClose(); }}>
        <DialogContent className="max-w-3xl max-h-[85vh] flex flex-col overflow-hidden">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Play className="h-4 w-4" />
              Run: {workflow?.name}
            </DialogTitle>
          </DialogHeader>

          <div className="space-y-4 flex-1 min-h-0 flex flex-col">
            {/* Filters */}
            <div className="flex items-center gap-2 shrink-0">
              <div className="relative flex-1">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
                <Input
                  placeholder="Search data sources..."
                  className="pl-8 h-8 text-sm"
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                />
              </div>
              <Select value={dataTypeFilter} onValueChange={setDataTypeFilter}>
                <SelectTrigger className="h-8 w-[140px] text-xs">
                  <SelectValue placeholder="All types" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__all__">All types</SelectItem>
                  {DATA_TYPE_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Data source list */}
            <div className="border rounded-md flex-1 min-h-0 overflow-auto">
              {filteredSources.length === 0 ? (
                <EmptyState
                  icon={Database}
                  title={dataSources.length === 0 ? 'No data sources available' : 'No matching data sources'}
                  description={dataSources.length === 0
                    ? 'Add data sources in the Knowledge tab.'
                    : 'No data sources match your filters.'}
                  className="py-8"
                />
              ) : (
                filteredSources.map((ds) => (
                  <button
                    key={ds.id}
                    className={`flex items-center gap-3 w-full text-left px-3 py-2.5 border-b last:border-b-0 transition-colors ${
                      selectedDataSourceId === ds.id
                        ? 'bg-primary/10 border-l-2 border-l-primary'
                        : 'hover:bg-muted/50'
                    }`}
                    onClick={() => setSelectedDataSourceId(ds.id)}
                  >
                    {sourceTypeIcon(ds.source_type)}
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium truncate">{ds.title}</div>
                      {ds.description && (
                        <div className="text-xs text-muted-foreground truncate">{ds.description}</div>
                      )}
                    </div>
                    <Badge variant="outline" className="text-[9px] shrink-0">
                      {DATA_TYPE_OPTIONS.find((o) => o.value === ds.data_type)?.label ?? ds.data_type}
                    </Badge>
                  </button>
                ))
              )}
            </div>

            {/* Model selector */}
            {Array.isArray(availableModels) && availableModels.length > 0 && (
              <div className="flex items-center gap-2 shrink-0">
                <span className="text-xs text-muted-foreground shrink-0">Model:</span>
                <Select value={effectiveModel} onValueChange={setSelectedModel}>
                  <SelectTrigger className="h-8 text-xs">
                    <SelectValue placeholder="Use workflow default" />
                  </SelectTrigger>
                  <SelectContent>
                    {availableModels.map((m) => (
                      <SelectItem key={m.id} value={m.id}>{m.label}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            {/* Run result */}
            {runMutation.isError && (
              <div className="p-3 rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-sm text-red-700 dark:text-red-300">
                Failed: {(runMutation.error as Error)?.message ?? 'Unknown error'}
              </div>
            )}

            {runMutation.isSuccess && (
              <div className="p-3 rounded-md bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-sm text-green-700 dark:text-green-300">
                Workflow completed. {runMutation.data?.staged_records ?? 0} records staged.
              </div>
            )}

            {/* Actions */}
            <DialogFooter>
              <Button variant="outline" size="sm" onClick={handleClose}>Cancel</Button>
              <Button
                size="sm"
                className="gap-1.5"
                onClick={() => runMutation.mutate()}
                disabled={!selectedDataSourceId || runMutation.isPending}
              >
                {runMutation.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Play className="h-3.5 w-3.5" />
                )}
                Run Workflow
              </Button>
            </DialogFooter>
          </div>
        </DialogContent>
      </Dialog>

    </>
  );
}
