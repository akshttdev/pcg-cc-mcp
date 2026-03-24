// ─── StagingTab ───────────────────────────────────────────────────────────────
//
// Staging tab: review, approve, reject, and commit records extracted by workflows.

import { useState, useMemo, useCallback } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { workflowKeys } from '@/lib/query-keys';
import { Button } from '@/components/ui/button';
import {
  ClipboardCheck,
  Hammer,
} from 'lucide-react';
import { stagingApi } from '@/lib/api';
import { StagingReviewContent } from '@/components/workflows/StagingReviewPanel';
import { useAuth } from '@/contexts/AuthContext';
import { VALID_GLOBAL_FILTERS } from '../../constants';
import type { GlobalFilterType } from '../../types';
import { StagingCardGrid } from './StagingCardGrid';
import { StagingFilterToolbar } from './StagingFilterToolbar';
import { StagingTableRow } from './StagingTableRow';
import { StagingHeader } from './StagingHeader';
import { useStagingData } from './useStagingData';

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

  const {
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
  } = useStagingData(orgId);

  const queryClient = useQueryClient();

  // Global batch mutations
  const batchApproveMutation = useMutationWithToast({
    mutationFn: async () => {
      await stagingApi.batchAction(pendingNonDuplicate.map(r => r.id), 'approve');
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

  // Per-record mutations
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { status?: string; record_data?: unknown } }) =>
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

  const handleInlineFieldSave = useCallback((recordId: string, fieldKey: string, newValue: string) => {
    const record = pendingRecords.find(r => r.id === recordId);
    if (!record) return;
    try {
      const data = JSON.parse(record.record_data);
      data[fieldKey] = newValue;
      updateMutation.mutate({ id: recordId, data: { record_data: data } });
    } catch { /* intentionally empty */ }
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

  // Sorting state
  const [sortField, setSortField] = useState<'name' | 'type' | 'workflow' | 'status' | 'confidence'>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  const handleSort = useCallback((field: typeof sortField) => {
    setSortDir(prev => sortField === field ? (prev === 'asc' ? 'desc' : 'asc') : 'asc');
    setSortField(field);
  }, [sortField]);

  // Filtered records
  const filteredRecords = useMemo(() => {
    let result = pendingRecords;

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

    if (typeFilter !== 'all') {
      result = result.filter(r => r.target_type === typeFilter);
    }

    if (workflowFilter !== 'all') {
      result = result.filter(r => r.workflow_run_id === workflowFilter);
    }

    return result;
  }, [pendingRecords, globalFilter, typeFilter, workflowFilter]);

  // Table rows with sorting
  const tableRows = useMemo(() => {
    const rows = filteredRecords.map(record => {
      let data: Record<string, unknown> = {};
      try { data = JSON.parse(record.record_data); } catch { /* intentionally empty */ }
      const displayName = (data.first_name as string)
        ? `${data.first_name} ${data.last_name || ''}`
        : (data.name as string) || (data.title as string) || 'Untitled';
      return { record, displayName, data, workflowName: runNameMap[record.workflow_run_id] || '' };
    });

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

  const filterTabs = [
    { key: 'all' as const, label: 'All', count: pendingRecords.length, color: '' },
    { key: 'valid' as const, label: 'Valid', count: pendingNonDuplicate.length, color: '' },
    { key: 'duplicates' as const, label: 'Dups', count: totalDuplicates, color: 'text-amber-600' },
    { key: 'issues' as const, label: 'Issues', count: totalWithIssues, color: 'text-amber-600' },
    { key: 'approved' as const, label: 'Approved', count: totalApproved, color: 'text-green-600' },
    { key: 'rejected' as const, label: 'Rejected', count: totalRejected, color: 'text-red-600' },
    { key: 'error' as const, label: 'Errors', count: totalError, color: 'text-red-600' },
  ];

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
      <StagingHeader
        pendingCount={pendingRecords.length}
        totalPending={totalPending}
        totalApproved={totalApproved}
        totalDuplicates={totalDuplicates}
        groupedCount={grouped.length}
        viewMode={viewMode}
        setCardsView={setCardsView}
        setTableView={setTableView}
        pendingNonDuplicateCount={pendingNonDuplicate.length}
        handleRejectDups={handleRejectDups}
        handleApproveAll={handleApproveAll}
        isRejectingDups={batchRejectDupsMutation.isPending}
        isApprovingAll={batchApproveMutation.isPending}
      />

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
        <StagingCardGrid grouped={grouped} selectRun={selectRun} />
      ) : (
        /* Global table view */
        <div className="border rounded-lg overflow-hidden">
          <StagingFilterToolbar
            globalFilter={globalFilter}
            updateParams={updateParams}
            filterTabs={filterTabs}
            uniqueTypes={uniqueTypes}
            typeFilter={typeFilter}
            uniqueWorkflows={uniqueWorkflows}
            workflowFilter={workflowFilter}
            filteredCount={filteredRecords.length}
            commonWarnings={viewMode === 'table' ? commonWarnings : []}
            sortField={sortField}
            sortDir={sortDir}
            handleSort={handleSort}
          />

          {/* Table rows */}
          {tableRows.length === 0 && (
            <div className="text-center py-8 text-muted-foreground text-sm">
              No records match the selected filter.
            </div>
          )}
          {tableRows.map((row) => (
            <StagingTableRow
              key={row.record.id}
              row={row}
              isExpanded={expandedId === row.record.id}
              onToggleExpand={handleToggleExpand}
              onStopPropagation={handleStopPropagation}
              onApprove={handleApproveRecord}
              onReject={handleRejectRecord}
              onRetry={handleRetryRecord}
              onInlineFieldSave={handleInlineFieldSave}
              isRetrying={retryMutation.isPending}
              commonWarningSet={commonWarningSet}
              schemasMap={schemasMap}
            />
          ))}
        </div>
      )}
    </div>
  );
}
