// ─── StagingTab ───────────────────────────────────────────────────────────────
//
// Staging tab: review, approve, reject, and commit records extracted by workflows.

import { useState, useMemo, useCallback } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { workflowKeys } from '@/lib/query-keys';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  ClipboardCheck,
  CheckCheck,
  XCircle,
  LayoutGrid,
  TableProperties,
  ChevronRight,
  AlertTriangle,
  Users,
  Plus,
  Check,
  RotateCcw,
  X,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  ArrowRight,
  Info,
  Hammer,
} from 'lucide-react';
import { workflowsApi, stagingApi, schemasApi } from '@/lib/api';
import type { WorkflowStagingRecord, TargetSchema } from '@/lib/api';
import { StagingReviewContent } from '@/components/workflows/StagingReviewPanel';
import { useAuth } from '@/contexts/AuthContext';
import { VALID_GLOBAL_FILTERS, STAGING_TARGET_CONFIG } from '../constants';
import type { GlobalFilterType } from '../types';
import { InlineEditField } from '../components/InlineEditField';

export function StagingTab() {
  const [searchParams, setSearchParams] = useSearchParams();
  const navigate = useNavigate();
  const { user } = useAuth();
  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Persist view/filter state in URL params
  const activeRunId = searchParams.get('run');
  const viewMode = (searchParams.get('view') === 'table' ? 'table' : 'cards') as 'cards' | 'table';
  const globalFilter = (VALID_GLOBAL_FILTERS.includes(searchParams.get('filter') as GlobalFilterType)
    ? searchParams.get('filter') as GlobalFilterType : 'all');
  const typeFilter = searchParams.get('type') || 'all';
  const workflowFilter = searchParams.get('wf') || 'all';

  // Helper to update search params preserving existing ones
  const updateParams = useCallback((updates: Record<string, string | null>) => {
    setSearchParams(prev => {
      const next = new URLSearchParams(prev);
      for (const [key, value] of Object.entries(updates)) {
        if (value === null || value === 'all' || value === 'cards') {
          next.delete(key);
        } else {
          next.set(key, value);
        }
      }
      // Always keep tab=staging
      next.set('tab', 'staging');
      return next;
    }, { replace: true });
  }, [setSearchParams]);

  const { data: pendingRecords = [], isLoading } = useQuery({
    queryKey: ['stagingPending', orgId],
    queryFn: () => stagingApi.listPending(orgId!),
    enabled: !!orgId,
    refetchInterval: 15000,
  });

  // Fetch recent runs to map run IDs to workflow names
  const { data: recentRuns = [] } = useQuery({
    queryKey: ['workflowRuns'],
    queryFn: () => workflowsApi.listRecentRuns({ limit: 100 }),
    staleTime: 30000,
  });

  const runNameMap = useMemo(() => {
    const m: Record<string, string> = {};
    for (const r of recentRuns as any[]) {
      if (r.id && r.workflow_name) m[r.id] = r.workflow_name;
    }
    return m;
  }, [recentRuns]);

  // Fetch schemas for all target types present in staging records
  const targetTypes = useMemo(() => [...new Set(pendingRecords.map(r => r.target_type))], [pendingRecords]);

  const { data: schemasMap = {} } = useQuery({
    queryKey: ['schemas', targetTypes],
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

  const queryClient = useQueryClient();

  // Global counts (always computed, even if not rendered)
  const totalPending = pendingRecords.filter(r => r.status === 'pending_review').length;
  const totalApproved = pendingRecords.filter(r => r.status === 'approved').length;
  const totalDuplicates = pendingRecords.filter(r => r.duplicate_of_id != null).length;
  const pendingNonDuplicate = useMemo(
    () => pendingRecords.filter(r => r.status === 'pending_review' && !r.duplicate_of_id),
    [pendingRecords]
  );

  // Global batch mutations -- must be declared before early returns
  const batchApproveMutation = useMutationWithToast({
    mutationFn: async () => {
      await stagingApi.batchAction(pendingNonDuplicate.map(r => r.id), 'approve');
      // Auto-commit after approving: commit all runs that have approved records
      const runIds = [...new Set(pendingNonDuplicate.map(r => r.workflow_run_id))];
      return Promise.all(runIds.map(rid => stagingApi.batchCommit(rid)));
    },
    successMessage: 'Batch approved and committed',
    errorMessage: 'Failed to batch approve',
    invalidateKeys: [workflowKeys.stagingPending()],
  });

  const batchRejectDupsMutation = useMutationWithToast({
    mutationFn: () => {
      const dupIds = pendingRecords.filter(r => r.status === 'pending_review' && r.duplicate_of_id != null).map(r => r.id);
      return stagingApi.batchAction(dupIds, 'reject');
    },
    successMessage: 'Duplicates rejected',
    errorMessage: 'Failed to reject duplicates',
    invalidateKeys: [workflowKeys.stagingPending()],
  });

  // Per-record mutations for table view actions
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { status?: string; record_data?: any } }) =>
      stagingApi.update(id, data),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: workflowKeys.stagingPending() }),
  });

  const commitMutation = useMutation({
    mutationFn: (id: string) => stagingApi.commit(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: workflowKeys.stagingPending() }),
  });

  const retryMutation = useMutationWithToast({
    mutationFn: async (id: string) => {
      await stagingApi.update(id, { status: 'approved' });
      return stagingApi.commit(id);
    },
    errorMessage: 'Failed to retry commit',
    invalidateKeys: [workflowKeys.stagingPending()],
  });

  const handleApproveRecord = useCallback((id: string) => {
    // Auto-commit: approve then immediately commit to CRM
    updateMutation.mutate({ id, data: { status: 'approved' } }, {
      onSuccess: () => {
        commitMutation.mutate(id);
      },
    });
  }, [updateMutation, commitMutation]);

  const handleRejectRecord = useCallback((id: string) => {
    updateMutation.mutate({ id, data: { status: 'rejected' } });
  }, [updateMutation]);

  const handleRetryRecord = useCallback((id: string) => {
    retryMutation.mutate(id);
  }, [retryMutation]);

  const handleToggleExpand = useCallback((id: string) => {
    setExpandedId(prev => prev === id ? null : id);
  }, []);

  const handleStopPropagation = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  // Inline field edit: update a single field in record_data
  const handleInlineFieldSave = useCallback((recordId: string, fieldKey: string, newValue: string) => {
    const record = pendingRecords.find(r => r.id === recordId);
    if (!record) return;
    try {
      const data = JSON.parse(record.record_data);
      data[fieldKey] = newValue;
      updateMutation.mutate({ id: recordId, data: { record_data: data } });
    } catch {}
  }, [pendingRecords, updateMutation]);

  const handleRejectDups = useCallback(() => {
    batchRejectDupsMutation.mutate();
  }, [batchRejectDupsMutation]);

  const handleApproveAll = useCallback(() => {
    batchApproveMutation.mutate();
  }, [batchApproveMutation]);

  const selectRun = useCallback((runId: string) => {
    updateParams({ run: runId });
  }, [updateParams]);

  const clearRun = useCallback(() => {
    updateParams({ run: null });
  }, [updateParams]);

  const setCardsView = useCallback(() => updateParams({ view: null }), [updateParams]);
  const setTableView = useCallback(() => updateParams({ view: 'table' }), [updateParams]);

  // Sorting state for global table
  const [sortField, setSortField] = useState<'name' | 'type' | 'workflow' | 'status' | 'confidence'>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  const handleSort = useCallback((field: typeof sortField) => {
    setSortDir(prev => sortField === field ? (prev === 'asc' ? 'desc' : 'asc') : 'asc');
    setSortField(field);
  }, [sortField]);

  // Derived filter counts
  const totalRejected = pendingRecords.filter(r => r.status === 'rejected').length;
  const totalError = pendingRecords.filter(r => r.status === 'error').length;
  const totalWithIssues = useMemo(() => pendingRecords.filter(r => {
    if (!r.validation_errors) return false;
    try { const e = JSON.parse(r.validation_errors); return Array.isArray(e) && e.length > 0; } catch { return false; }
  }).length, [pendingRecords]);

  // Unique target types and workflows for filter dropdowns
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

  // Detect common warnings that appear on many records -- collapse into batch banner
  const commonWarnings = useMemo(() => {
    const warnCounts: Record<string, number> = {};
    for (const r of pendingRecords) {
      if (!r.validation_errors) continue;
      try {
        const errs = JSON.parse(r.validation_errors);
        if (Array.isArray(errs)) {
          for (const e of errs) warnCounts[e] = (warnCounts[e] || 0) + 1;
        }
      } catch {}
    }
    // Warnings appearing on more than half of records are "common"
    const threshold = Math.max(2, Math.floor(pendingRecords.length * 0.5));
    return Object.entries(warnCounts)
      .filter(([, count]) => count >= threshold)
      .map(([msg, count]) => ({ msg, count }));
  }, [pendingRecords]);

  const commonWarningSet = useMemo(() => new Set(commonWarnings.map(w => w.msg)), [commonWarnings]);

  // Filtered records for global table view
  const filteredRecords = useMemo(() => {
    let result = pendingRecords;

    // Status/category filter
    switch (globalFilter) {
      case 'valid':
        result = result.filter(r => r.status === 'pending_review' && !r.duplicate_of_id);
        break;
      case 'duplicates':
        result = result.filter(r => r.duplicate_of_id != null);
        break;
      case 'approved':
        result = result.filter(r => r.status === 'approved');
        break;
      case 'rejected':
        result = result.filter(r => r.status === 'rejected');
        break;
      case 'error':
        result = result.filter(r => r.status === 'error');
        break;
      case 'issues':
        result = result.filter(r => {
          if (!r.validation_errors) return false;
          try { const e = JSON.parse(r.validation_errors); return Array.isArray(e) && e.length > 0; } catch { return false; }
        });
        break;
    }

    // Type filter
    if (typeFilter !== 'all') {
      result = result.filter(r => r.target_type === typeFilter);
    }

    // Workflow filter
    if (workflowFilter !== 'all') {
      result = result.filter(r => r.workflow_run_id === workflowFilter);
    }

    return result;
  }, [pendingRecords, globalFilter, typeFilter, workflowFilter]);

  // Parsed data for table rows with sorting
  const tableRows = useMemo(() => {
    const rows = filteredRecords.map(record => {
      let data: Record<string, any> = {};
      try { data = JSON.parse(record.record_data); } catch {}
      const displayName = data.first_name
        ? `${data.first_name} ${data.last_name || ''}`
        : data.name || data.title || 'Untitled';
      return { record, displayName, data, workflowName: runNameMap[record.workflow_run_id] || '' };
    });

    // Apply sorting
    rows.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'name': cmp = a.displayName.localeCompare(b.displayName); break;
        case 'type': cmp = a.record.target_type.localeCompare(b.record.target_type); break;
        case 'workflow': cmp = a.workflowName.localeCompare(b.workflowName); break;
        case 'status': cmp = a.record.status.localeCompare(b.record.status); break;
        case 'confidence': cmp = (a.record.confidence ?? 0) - (b.record.confidence ?? 0); break;
      }
      return sortDir === 'desc' ? -cmp : cmp;
    });

    return rows;
  }, [filteredRecords, runNameMap, sortField, sortDir]);

  // If a run is selected, show full-page inline review
  if (activeRunId) {
    return (
      <div className="h-full flex flex-col -m-4 sm:-m-6 lg:-m-8">
        <StagingReviewContent
          workflowRunId={activeRunId}
          workflowName={runNameMap[activeRunId]}
          onBack={clearRun}
          alwaysEnabled
        />
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading staging records...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4 max-w-[1400px] mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Staging Review</h2>
          <p className="text-sm text-muted-foreground">
            Review and approve records extracted by workflows before they are committed to the CRM.
          </p>
        </div>
        <div className="flex items-center gap-3">
          {pendingRecords.length > 0 && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Badge variant="outline">{totalPending} pending</Badge>
              {totalApproved > 0 && <Badge variant="outline" className="text-green-600 border-green-200">{totalApproved} approved</Badge>}
              {totalDuplicates > 0 && <Badge variant="outline" className="text-amber-600 border-amber-200">{totalDuplicates} duplicates</Badge>}
              <span className="text-muted-foreground/50">|</span>
              <span>{grouped.length} run{grouped.length !== 1 ? 's' : ''}</span>
            </div>
          )}
          {/* View toggle */}
          {pendingRecords.length > 0 && (
            <div className="flex items-center border rounded-md">
              <button
                onClick={setCardsView}
                className={`p-1.5 rounded-l-md transition-colors ${viewMode === 'cards' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
                title="Card view"
              >
                <LayoutGrid className="h-3.5 w-3.5" />
              </button>
              <button
                onClick={setTableView}
                className={`p-1.5 rounded-r-md transition-colors ${viewMode === 'table' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
                title="Table view"
              >
                <TableProperties className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Global batch action bar */}
      {pendingRecords.length > 0 && (
        <div className="flex items-center gap-2 p-3 rounded-lg border bg-muted/30">
          <span className="text-xs text-muted-foreground mr-auto">Batch actions across all runs:</span>
          {totalDuplicates > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1 text-amber-600 border-amber-200 hover:bg-amber-50"
              onClick={handleRejectDups}
              disabled={batchRejectDupsMutation.isPending}
            >
              <XCircle className="h-3 w-3" />
              {batchRejectDupsMutation.isPending ? 'Removing...' : `Reject ${totalDuplicates} duplicates`}
            </Button>
          )}
          {pendingNonDuplicate.length > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1 text-green-600 border-green-200 hover:bg-green-50"
              onClick={handleApproveAll}
              disabled={batchApproveMutation.isPending}
            >
              <CheckCheck className="h-3 w-3" />
              {batchApproveMutation.isPending ? 'Approving & committing...' : `Approve & commit ${pendingNonDuplicate.length} valid`}
            </Button>
          )}
        </div>
      )}

      {grouped.length === 0 ? (
        <div className="text-center py-16 text-muted-foreground">
          <ClipboardCheck className="h-10 w-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm font-medium">No pending records</p>
          <p className="text-xs mt-1 max-w-sm mx-auto">
            When you run a workflow against a data source, extracted records appear here for review before being committed to your CRM.
          </p>
          <div className="flex items-center justify-center gap-2 mt-4">
            <Button
              size="sm"
              className="gap-1.5"
              onClick={() => navigate('/workflows?tab=builder')}
            >
              <Hammer className="h-3.5 w-3.5" />
              Go to Builder
            </Button>
          </div>
        </div>
      ) : viewMode === 'cards' ? (
        <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
          {grouped.map((group) => {
            const statuses = group.records.reduce<Record<string, number>>((acc, r) => {
              acc[r.status] = (acc[r.status] || 0) + 1;
              return acc;
            }, {});
            const groupTargetTypes = group.records.reduce<Record<string, number>>((acc, r) => {
              acc[r.target_type] = (acc[r.target_type] || 0) + 1;
              return acc;
            }, {});
            const typeLabels: Record<string, string> = {
              crm_contact: 'contacts',
              company: 'companies',
              crm_deal: 'deals',
              task: 'tasks',
            };
            return (
              <Card
                key={group.runId}
                className="card-interactive cursor-pointer"
                onClick={() => selectRun(group.runId)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm">
                      {group.workflowName || 'Workflow Run'}
                    </CardTitle>
                    <Badge variant="outline" className="text-[10px]">
                      {group.records.length} pending
                    </Badge>
                  </div>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {Object.entries(groupTargetTypes).map(([type, count]) => (
                      <Badge key={type} variant="secondary" className="text-[9px]">
                        {count} {typeLabels[type] || type}
                      </Badge>
                    ))}
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="flex flex-wrap gap-1">
                      {Object.entries(statuses).map(([status, count]) => {
                        const statusColors: Record<string, string> = {
                          pending: 'text-amber-600',
                          approved: 'text-green-600',
                          rejected: 'text-red-600',
                        };
                        return (
                          <span key={status} className={`text-[10px] ${statusColors[status] || 'text-muted-foreground'}`}>
                            {count} {status}
                          </span>
                        );
                      })}
                    </div>
                    <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      ) : (
        /* Global table view -- all records across all runs */
        <div className="border rounded-lg overflow-hidden">
          {/* Filter toolbar */}
          <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/20 flex-wrap">
            {/* Status filter tabs */}
            <div className="flex items-center gap-0.5 mr-2">
              {([
                { key: 'all' as const, label: 'All', count: pendingRecords.length, color: '' },
                { key: 'valid' as const, label: 'Valid', count: pendingNonDuplicate.length, color: '' },
                { key: 'duplicates' as const, label: 'Dups', count: totalDuplicates, color: 'text-amber-600' },
                { key: 'issues' as const, label: 'Issues', count: totalWithIssues, color: 'text-amber-600' },
                { key: 'approved' as const, label: 'Approved', count: totalApproved, color: 'text-green-600' },
                { key: 'rejected' as const, label: 'Rejected', count: totalRejected, color: 'text-red-600' },
                { key: 'error' as const, label: 'Errors', count: totalError, color: 'text-red-600' },
              ]).map(({ key, label, count, color }) => {
                if (count === 0 && key !== 'all') return null;
                return (
                  <button
                    key={key}
                    onClick={() => updateParams({ filter: key })}
                    className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                      globalFilter === key
                        ? 'bg-primary text-primary-foreground'
                        : `hover:bg-muted text-muted-foreground ${color && globalFilter !== key ? color : ''}`
                    }`}
                  >
                    {label} ({count})
                  </button>
                );
              })}
            </div>

            <div className="h-4 w-px bg-border" />

            {/* Type filter */}
            {uniqueTypes.length > 1 && (
              <select
                value={typeFilter}
                onChange={(e) => updateParams({ type: e.target.value })}
                className="h-6 text-[11px] rounded border bg-background px-1.5 text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="all">All types</option>
                {uniqueTypes.map(type => (
                  <option key={type} value={type}>
                    {STAGING_TARGET_CONFIG[type]?.label || type}
                  </option>
                ))}
              </select>
            )}

            {/* Workflow filter */}
            {uniqueWorkflows.length > 1 && (
              <select
                value={workflowFilter}
                onChange={(e) => updateParams({ wf: e.target.value })}
                className="h-6 text-[11px] rounded border bg-background px-1.5 text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring max-w-[200px]"
              >
                <option value="all">All workflows</option>
                {uniqueWorkflows.map(([id, name]) => (
                  <option key={id} value={id}>{name}</option>
                ))}
              </select>
            )}

            <div className="flex-1" />
            <span className="text-[10px] text-muted-foreground">{filteredRecords.length} record{filteredRecords.length !== 1 ? 's' : ''}</span>
          </div>

          {/* Common warnings banner -- collapse identical warnings */}
          {viewMode === 'table' && commonWarnings.length > 0 && (
            <div className="px-4 py-2 border-b bg-blue-50/60 dark:bg-blue-950/15 flex items-start gap-2">
              <Info className="h-3.5 w-3.5 text-blue-400 shrink-0 mt-0.5" />
              <div className="text-[11px] text-blue-700 dark:text-blue-400">
                {commonWarnings.map(w => (
                  <div key={w.msg}>{w.msg} <span className="text-blue-500">({w.count} records)</span></div>
                ))}
              </div>
            </div>
          )}

          {/* Sortable table header */}
          <div className="grid grid-cols-[20px_28px_minmax(120px,1fr)_minmax(100px,0.7fr)_80px_60px_60px_minmax(160px,1.5fr)_100px] gap-2 px-4 py-1.5 text-[10px] font-medium text-muted-foreground uppercase tracking-wider border-b bg-muted/10">
            <div />
            <div />
            <button onClick={() => handleSort('name')} className="flex items-center gap-1 hover:text-foreground text-left">
              Name {sortField === 'name' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('workflow')} className="flex items-center gap-1 hover:text-foreground text-left">
              Workflow {sortField === 'workflow' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('type')} className="flex items-center gap-1 hover:text-foreground text-left">
              Type {sortField === 'type' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('status')} className="flex items-center gap-1 hover:text-foreground text-left">
              Status {sortField === 'status' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('confidence')} className="flex items-center gap-1 hover:text-foreground text-left">
              Conf. {sortField === 'confidence' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <div>Details</div>
            <div className="text-right">Actions</div>
          </div>

          {/* Table rows */}
          {tableRows.length === 0 && (
            <div className="text-center py-8 text-muted-foreground text-sm">
              No records match the selected filter.
            </div>
          )}
          {tableRows.map(({ record, displayName, data, workflowName }) => {
            const config = STAGING_TARGET_CONFIG[record.target_type];
            const Icon = config?.icon || Users;
            const isExpanded = expandedId === record.id;

            const detailFields = Object.entries(data)
              .filter(([k, v]) => v != null && !['first_name', 'last_name', 'name', 'title'].includes(k))
              .slice(0, 3);

            const allFields = Object.entries(data).filter(([, v]) => v != null && v !== '');

            let validationErrs: string[] = [];
            if (record.validation_errors) {
              try { validationErrs = JSON.parse(record.validation_errors); } catch {}
            }

            // Check if this record has any of the common/global warnings
            const hasCommonWarning = validationErrs.some(e => commonWarningSet.has(e));
            // Row-specific (non-common) warnings only
            const rowSpecificErrs = validationErrs.filter(e => !commonWarningSet.has(e));

            return (
              <div key={record.id}>
                <div
                  role="row"
                  tabIndex={0}
                  onClick={() => handleToggleExpand(record.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      handleToggleExpand(record.id);
                    }
                  }}
                  className={`grid grid-cols-[20px_28px_minmax(120px,1fr)_minmax(100px,0.7fr)_80px_60px_60px_minmax(160px,1.5fr)_100px] gap-2 px-4 py-1.5 items-center border-b text-xs hover:bg-muted/30 transition-colors cursor-pointer select-none focus:outline-none focus:ring-1 focus:ring-ring focus:ring-inset ${
                    record.status === 'approved' ? 'bg-green-50/30 dark:bg-green-950/10' : ''
                  } ${record.status === 'rejected' ? 'bg-red-50/30 dark:bg-red-950/10 opacity-50' : ''
                  } ${record.duplicate_of_id ? 'bg-amber-50/20 dark:bg-amber-950/5' : ''
                  } ${isExpanded ? 'bg-muted/40 border-b-0' : ''}`}
                >
                  <ChevronRight className={`h-3.5 w-3.5 text-muted-foreground transition-transform ${isExpanded ? 'rotate-90' : ''}`} />
                  <Icon className={`h-3.5 w-3.5 ${config?.color || 'text-muted-foreground'}`} />

                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="font-medium truncate">{displayName}</span>
                    {record.duplicate_of_id && (
                      <span title="Potential duplicate"><AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" /></span>
                    )}
                    {rowSpecificErrs.length > 0 && !record.duplicate_of_id && (
                      <span title={`${rowSpecificErrs.length} issue(s)`}><AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" /></span>
                    )}
                    {hasCommonWarning && (
                      <span title="Affected by global warning (see banner above)"><Info className="h-3 w-3 text-blue-400 shrink-0" /></span>
                    )}
                  </div>

                  <span className="text-[10px] text-muted-foreground truncate">{workflowName || record.workflow_run_id.slice(0, 8)}</span>
                  <span className="text-[10px] text-muted-foreground">{config?.label || record.target_type}</span>

                  <Badge
                    variant="outline"
                    className={`text-[9px] h-5 px-1.5 justify-center ${
                      record.status === 'pending_review' ? 'text-amber-600 border-amber-200'
                        : record.status === 'approved' ? 'text-green-600 border-green-200'
                        : record.status === 'rejected' ? 'text-red-600 border-red-200'
                        : 'text-blue-600 border-blue-200'
                    }`}
                  >
                    {record.status === 'pending_review' ? 'pending' : record.status}
                  </Badge>

                  {record.confidence != null ? (
                    <Badge variant="outline" className={`text-[10px] ${
                      record.confidence >= 0.8 ? 'text-green-600 border-green-200'
                        : record.confidence >= 0.5 ? 'text-amber-600 border-amber-200'
                        : 'text-red-600 border-red-200'
                    }`}>
                      {Math.round(record.confidence * 100)}%
                    </Badge>
                  ) : <div />}

                  <div className="flex items-center gap-3 min-w-0 overflow-hidden">
                    {detailFields.map(([key, value]) => {
                      const dv = value == null ? '' : typeof value === 'object' ? JSON.stringify(value) : String(value);
                      return (
                        <span key={key} className="text-[10px] text-muted-foreground truncate">
                          <span className="opacity-60">{key}:</span> {dv}
                        </span>
                      );
                    })}
                  </div>

                  <div className="flex items-center gap-0.5 justify-end" onClick={handleStopPropagation}>
                    {record.status === 'pending_review' && (
                      <>
                        <button onClick={() => handleApproveRecord(record.id)} className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900" title="Approve & Commit">
                          <Check className="h-3 w-3 text-green-600" />
                        </button>
                        <button onClick={() => handleRejectRecord(record.id)} className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900" title="Reject">
                          <X className="h-3 w-3 text-red-500" />
                        </button>
                      </>
                    )}
                    {record.status === 'error' && (
                      <button onClick={() => handleRetryRecord(record.id)} className="p-1 rounded hover:bg-amber-100 dark:hover:bg-amber-900" title="Retry" disabled={retryMutation.isPending}>
                        <RotateCcw className="h-3 w-3 text-amber-600" />
                      </button>
                    )}
                  </div>
                </div>

                {/* Expanded detail panel with inline editing */}
                {isExpanded && (() => {
                  const schema = schemasMap[record.target_type];
                  const schemaFields = schema?.fields || {};
                  const existingKeys = new Set(allFields.map(([k]) => k));
                  const missingFields = Object.entries(schemaFields).filter(([k]) => !existingKeys.has(k));
                  const canEdit = record.status === 'pending_review';

                  return (
                  <div className="border-b bg-muted/20 px-4 py-3" onClick={handleStopPropagation}>
                    <div className="grid grid-cols-[1fr_1fr] lg:grid-cols-[1fr_1fr_1fr] gap-x-6 gap-y-2">
                      {allFields.map(([key, value]) => {
                        // Treat null/undefined as empty string to avoid showing literal "null"
                        const dv = value == null ? '' : typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);
                        const isLong = dv.length > 80;
                        return (
                          <div key={key} className={isLong ? 'col-span-2 lg:col-span-3' : ''}>
                            <dt className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide">
                              {key.replace(/_/g, ' ')}
                              {schemaFields[key]?.required && <span className="text-red-400 ml-0.5">*</span>}
                            </dt>
                            <InlineEditField
                              fieldKey={key}
                              value={dv}
                              recordId={record.id}
                              onSave={handleInlineFieldSave}
                              editable={canEdit}
                              fieldDef={schemaFields[key]}
                            />
                          </div>
                        );
                      })}
                    </div>

                    {/* Add field dropdown for missing schema fields */}
                    {canEdit && missingFields.length > 0 && (
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <button className="mt-2 flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors">
                            <Plus className="h-3 w-3" /> Add field
                          </button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="start" className="max-h-64 overflow-y-auto">
                          {missingFields.map(([fieldName, fieldDef]) => (
                            <DropdownMenuItem
                              key={fieldName}
                              onClick={() => handleInlineFieldSave(record.id, fieldName, '')}
                              className="text-xs"
                            >
                              <span>{fieldName.replace(/_/g, ' ')}</span>
                              {fieldDef.required && <span className="text-red-400 ml-1">*</span>}
                              <span className="ml-auto text-[10px] text-muted-foreground pl-4">{fieldDef.type}</span>
                            </DropdownMenuItem>
                          ))}
                        </DropdownMenuContent>
                      </DropdownMenu>
                    )}

                    <div className="flex items-center gap-4 mt-3 pt-2 border-t border-muted text-[10px] text-muted-foreground">
                      <span>ID: <code className="text-[9px]">{record.id.slice(0, 8)}</code></span>
                      <span>Run: <code className="text-[9px]">{record.workflow_run_id.slice(0, 8)}</code></span>
                      {workflowName && <span>Workflow: {workflowName}</span>}
                      {record.duplicate_of_id && (
                        <span className="text-amber-600">Duplicate of: <code className="text-[9px]">{record.duplicate_of_id.slice(0, 8)}</code></span>
                      )}
                      {record.created_at && <span>Created: {new Date(record.created_at).toLocaleString()}</span>}
                    </div>

                    {validationErrs.length > 0 && (
                      <div className="mt-2 pt-2 border-t border-amber-200 dark:border-amber-800">
                        <p className="text-[10px] font-medium text-amber-600 mb-1">Validation Issues</p>
                        <ul className="space-y-0.5">
                          {validationErrs.map((err, i) => (
                            <li key={i} className="text-[10px] text-amber-600 flex items-start gap-1.5">
                              <AlertTriangle className="h-3 w-3 shrink-0 mt-0.5" />
                              {err}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                  );
                })()}

                {/* Compact inline warnings when NOT expanded -- skip common warnings shown in banner */}
                {!isExpanded && rowSpecificErrs.length > 0 && record.status !== 'rejected' && (
                  <div className="px-4 py-1 bg-amber-50/50 dark:bg-amber-950/10 border-b flex items-center gap-1.5">
                    <AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" />
                    <span className="text-[10px] text-amber-600">{rowSpecificErrs.join(' \u00b7 ')}</span>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
