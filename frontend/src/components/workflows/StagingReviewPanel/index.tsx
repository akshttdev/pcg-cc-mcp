import { useState, useMemo, useCallback, useEffect } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { workflowKeys } from '@/lib/query-keys';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  CheckCheck,
  XCircle,
  Loader2,
  ArrowLeft,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { stagingApi, schemasApi } from '@/lib/api';
import type { WorkflowStagingRecord, CommitResult, TargetSchema } from '@/lib/api';

import { RecordRow } from './RecordRow';

// ─── Shared content component (used by both inline and dialog modes) ───

interface StagingReviewContentProps {
  workflowRunId: string;
  workflowName?: string;
  onBack?: () => void;
  /** When true, data is always fetched (no `enabled` guard) */
  alwaysEnabled?: boolean;
}

export function StagingReviewContent({
  workflowRunId,
  workflowName,
  onBack,
  alwaysEnabled,
}: StagingReviewContentProps) {
  const queryClient = useQueryClient();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editData, setEditData] = useState<Record<string, unknown>>({});
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [filter, setFilter] = useState<'all' | 'valid' | 'duplicates' | 'validation_issues' | 'approved' | 'rejected' | 'error'>('all');
  const [commitErrors, setCommitErrors] = useState<Record<string, string>>({});

  const { data: records = [], isLoading } = useQuery({
    queryKey: workflowKeys.staging(workflowRunId),
    queryFn: () => stagingApi.listByRun(workflowRunId),
    enabled: alwaysEnabled ? !!workflowRunId : !!workflowRunId,
    refetchInterval: editingId ? false : 10_000,
  });

  // Fetch schemas for target types in records
  const targetTypes = useMemo(() => [...new Set(records.map(r => r.target_type))], [records]);

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

  const hasValidationErrors = useCallback((r: WorkflowStagingRecord): boolean => {
    if (!r.validation_errors) return false;
    try {
      const errs = JSON.parse(r.validation_errors);
      return Array.isArray(errs) && errs.length > 0;
    } catch { return false; }
  }, []);

  const validationIssueCount = useMemo(() => records.filter(hasValidationErrors).length, [records, hasValidationErrors]);

  // Auto-select "Validation Issues" tab if all records have validation errors
  useEffect(() => {
    if (records.length > 0) {
      const allHaveIssues = records.every(r => {
        if (!r.validation_errors) return false;
        try {
          const errs = typeof r.validation_errors === 'string' ? JSON.parse(r.validation_errors) : r.validation_errors;
          return Array.isArray(errs) && errs.length > 0;
        } catch {
          return false;
        }
      });
      if (allHaveIssues) {
        setFilter('validation_issues');
      }
    }
  }, [records]);

  const avgConfidence = useMemo(() => {
    const withConf = records.filter(r => r.confidence !== null && r.confidence !== undefined);
    if (withConf.length === 0) return null;
    const sum = withConf.reduce((acc, r) => acc + (r.confidence ?? 0), 0);
    return sum / withConf.length;
  }, [records]);

  const grouped = useMemo(() => {
    let filtered = records;
    switch (filter) {
      case 'valid':
        filtered = records.filter(r => !r.duplicate_of_id && r.status === 'pending_review');
        break;
      case 'duplicates':
        filtered = records.filter(r => r.duplicate_of_id != null);
        break;
      case 'validation_issues':
        filtered = records.filter(hasValidationErrors);
        break;
      case 'approved':
        filtered = records.filter(r => r.status === 'approved');
        break;
      case 'rejected':
        filtered = records.filter(r => r.status === 'rejected');
        break;
      case 'error':
        filtered = records.filter(r => r.status === 'error');
        break;
    }
    const groups: Record<string, WorkflowStagingRecord[]> = {};
    for (const r of filtered) {
      if (!groups[r.target_type]) groups[r.target_type] = [];
      groups[r.target_type].push(r);
    }
    return groups;
  }, [records, filter, hasValidationErrors]);

  const pendingCount = records.filter(r => r.status === 'pending_review').length;
  const approvedCount = records.filter(r => r.status === 'approved').length;
  const committedCount = records.filter(r => r.status === 'committed').length;
  const duplicateCount = records.filter(r => r.duplicate_of_id != null).length;
  const validPendingCount = records.filter(r => r.status === 'pending_review' && !r.duplicate_of_id).length;
  const errorCount = records.filter(r => r.status === 'error').length;
  const duplicatePendingCount = records.filter(r => r.status === 'pending_review' && r.duplicate_of_id != null).length;

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { status?: string; record_data?: unknown } }) =>
      stagingApi.update(id, data),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: workflowKeys.staging(workflowRunId) }),
  });

  // Inline field save for expanded view (schema-aware editing)
  const handleInlineFieldSave = useCallback((recordId: string, fieldKey: string, newValue: string) => {
    const record = records.find(r => r.id === recordId);
    if (!record) return;
    try {
      const data = JSON.parse(record.record_data);
      data[fieldKey] = newValue;
      updateMutation.mutate({ id: recordId, data: { record_data: data } });
    } catch { /* intentionally empty */ }
  }, [records, updateMutation]);

  const commitMutation = useMutation({
    mutationFn: (id: string) => stagingApi.commit(id),
    onSuccess: (result: CommitResult) => {
      if (result.error) {
        setCommitErrors(prev => ({ ...prev, [result.id]: result.error! }));
      }
      queryClient.invalidateQueries({ queryKey: workflowKeys.staging(workflowRunId) });
    },
  });

  const batchCommitMutation = useMutation({
    mutationFn: () => stagingApi.batchCommit(workflowRunId),
    onSuccess: (result) => {
      const newErrors: Record<string, string> = {};
      for (const r of result.results) {
        if (r.error) {
          newErrors[r.id] = r.error;
        }
      }
      if (Object.keys(newErrors).length > 0) {
        setCommitErrors(prev => ({ ...prev, ...newErrors }));
      }
      queryClient.invalidateQueries({ queryKey: workflowKeys.staging(workflowRunId) });
    },
  });

  const autoApproveMutation = useMutation({
    mutationFn: async () => {
      await stagingApi.autoApprove(workflowRunId);
      // Auto-commit after approving
      return stagingApi.batchCommit(workflowRunId);
    },
    onSuccess: (result) => {
      if (result?.results) {
        const newErrors: Record<string, string> = {};
        for (const r of result.results) {
          if (r.error) newErrors[r.id] = r.error;
        }
        if (Object.keys(newErrors).length > 0) {
          setCommitErrors(prev => ({ ...prev, ...newErrors }));
        }
      }
      queryClient.invalidateQueries({ queryKey: workflowKeys.staging(workflowRunId) });
    },
  });

  const rejectDuplicatesMutation = useMutationWithToast({
    mutationFn: () => stagingApi.rejectDuplicates(workflowRunId),
    successMessage: 'Duplicates rejected',
    errorMessage: 'Failed to reject duplicates',
    invalidateKeys: [workflowKeys.staging(workflowRunId)],
  });

  const retryMutation = useMutation({
    mutationFn: async (id: string) => {
      await stagingApi.update(id, { status: 'approved' });
      return stagingApi.commit(id);
    },
    onSuccess: (result: CommitResult) => {
      if (result.error) {
        setCommitErrors(prev => ({ ...prev, [result.id]: result.error! }));
      } else {
        setCommitErrors(prev => {
          const next = { ...prev };
          delete next[result.id];
          return next;
        });
      }
      queryClient.invalidateQueries({ queryKey: workflowKeys.staging(workflowRunId) });
    },
  });

  const handleApprove = useCallback((id: string) => {
    // Auto-commit: approve then immediately commit to CRM
    updateMutation.mutate({ id, data: { status: 'approved' } }, {
      onSuccess: () => {
        commitMutation.mutate(id);
      },
    });
  }, [updateMutation, commitMutation]);

  const handleReject = useCallback((id: string) => {
    updateMutation.mutate({ id, data: { status: 'rejected' } });
  }, [updateMutation]);

  const handleStartEdit = useCallback((record: WorkflowStagingRecord) => {
    try {
      setEditData(JSON.parse(record.record_data));
      setEditingId(record.id);
    } catch {
      setEditData({});
      setEditingId(record.id);
    }
  }, []);

  const handleCancelEdit = useCallback(() => {
    setEditingId(null);
  }, []);

  const handleSaveEdit = useCallback((id: string) => {
    updateMutation.mutate({ id, data: { record_data: editData } });
    setEditingId(null);
  }, [updateMutation, editData]);

  const handleEditField = useCallback((key: string, value: string) => {
    setEditData(prev => ({ ...prev, [key]: value }));
  }, []);

  const handleRetry = useCallback((id: string) => {
    retryMutation.mutate(id);
  }, [retryMutation]);

  const handleRejectDuplicates = useCallback(() => {
    rejectDuplicatesMutation.mutate();
  }, [rejectDuplicatesMutation]);

  const handleAutoApprove = useCallback(() => {
    autoApproveMutation.mutate();
  }, [autoApproveMutation]);

  const handleClearFilter = useCallback(() => {
    setFilter('all');
  }, []);

  const handleStopPropagation = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  const toggleExpand = useCallback((id: string) => {
    setExpandedId(prev => prev === id ? null : id);
  }, []);

  // Flatten all records for table view
  const flatRecords = useMemo(() => {
    const result: { record: WorkflowStagingRecord; targetType: string; displayName: string; data: Record<string, unknown> }[] = [];
    for (const [targetType, groupRecords] of Object.entries(grouped)) {
      for (const record of groupRecords) {
        let data: Record<string, unknown> = {};
        try { data = JSON.parse(record.record_data); } catch { /* intentionally empty */ }
        const displayName = data.first_name
          ? `${String(data.first_name)} ${String(data.last_name || '')}`
          : String(data.name || data.title || 'Untitled');
        result.push({ record, targetType, displayName, data });
      }
    }
    return result;
  }, [grouped]);

  return (
    <div className="flex flex-col h-full">
      {/* Compact header */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-b">
        {onBack && (
          <Button variant="ghost" size="sm" onClick={onBack} className="h-7 w-7 p-0">
            <ArrowLeft className="h-4 w-4" />
          </Button>
        )}
        <h2 className="text-sm font-semibold">Review Staged Records</h2>
        {workflowName && (
          <span className="text-xs text-muted-foreground">— {workflowName}</span>
        )}
        <div className="flex-1" />
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <span>{records.length} total</span>
          {avgConfidence !== null && (
            <Badge variant="outline" className={cn('text-[9px] h-5',
              avgConfidence >= 0.8 ? 'text-green-600 border-green-200'
                : avgConfidence >= 0.5 ? 'text-amber-600 border-amber-200'
                : 'text-red-600 border-red-200'
            )}>
              avg {Math.round(avgConfidence * 100)}%
            </Badge>
          )}
          {duplicateCount > 0 && (
            <Badge variant="outline" className="text-[9px] h-5 text-amber-600 border-amber-200">{duplicateCount} dups</Badge>
          )}
          {validationIssueCount > 0 && (
            <Badge variant="outline" className="text-[9px] h-5 text-amber-600 border-amber-200">{validationIssueCount} issues</Badge>
          )}
          <Badge variant="outline" className="text-[9px] h-5">{pendingCount} pending</Badge>
          <Badge variant="outline" className="text-[9px] h-5 text-green-600 border-green-200">{approvedCount} approved</Badge>
          <Badge variant="outline" className="text-[9px] h-5 text-blue-600 border-blue-200">{committedCount} committed</Badge>
          {errorCount > 0 && (
            <Badge variant="outline" className="text-[9px] h-5 text-red-600 border-red-200">{errorCount} errors</Badge>
          )}
        </div>
      </div>

      {/* Filter tabs + smart actions toolbar */}
      {records.length > 0 && (
        <div className="flex items-center gap-2 px-4 py-1.5 border-b bg-muted/30">
          <div className="flex items-center gap-0.5 mr-2">
            {(['all', 'valid', 'duplicates', 'validation_issues', 'approved', 'rejected', 'error'] as const).map((f) => {
              const counts = {
                all: records.length,
                valid: validPendingCount,
                duplicates: duplicateCount,
                validation_issues: validationIssueCount,
                approved: approvedCount,
                rejected: records.filter(r => r.status === 'rejected').length,
                error: errorCount,
              };
              if (counts[f] === 0 && f !== 'all') return null;
              const labels: Record<string, string> = {
                all: 'All',
                valid: 'Valid',
                duplicates: 'Dups',
                validation_issues: 'Issues',
                approved: 'Approved',
                rejected: 'Rejected',
                error: 'Errors',
              };
              return (
                <button
                  key={f}
                  onClick={() => setFilter(f)}
                  className={cn(
                    'px-2 py-0.5 rounded text-xs transition-colors',
                    filter === f
                      ? 'bg-primary text-primary-foreground'
                      : 'hover:bg-muted text-muted-foreground',
                    f === 'validation_issues' && counts[f] > 0 && filter !== f && 'text-amber-600',
                    f === 'error' && counts[f] > 0 && filter !== f && 'text-red-600'
                  )}
                >
                  {labels[f]} ({counts[f]})
                </button>
              );
            })}
          </div>

          <div className="flex-1" />

          {duplicatePendingCount > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-6 text-xs gap-1 text-amber-600 border-amber-200 hover:bg-amber-50"
              onClick={handleRejectDuplicates}
              disabled={rejectDuplicatesMutation.isPending}
            >
              <XCircle className="h-3 w-3" />
              {rejectDuplicatesMutation.isPending ? 'Removing...' : `Remove ${duplicatePendingCount} dups`}
            </Button>
          )}

          {validPendingCount > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-6 text-xs gap-1 text-green-600 border-green-200 hover:bg-green-50"
              onClick={handleAutoApprove}
              disabled={autoApproveMutation.isPending}
            >
              <CheckCheck className="h-3 w-3" />
              {autoApproveMutation.isPending ? 'Approving & committing...' : `Approve & commit ${validPendingCount}`}
            </Button>
          )}
        </div>
      )}

      {/* Table-style records */}
      <ScrollArea className="flex-1">
        {isLoading && (
          <div className="text-center py-12 text-muted-foreground">
            <Loader2 className="h-6 w-6 mx-auto mb-2 animate-spin" />
            <p className="text-sm">Loading staged records...</p>
          </div>
        )}

        {!isLoading && records.length === 0 && (
          <div className="text-center py-12 text-muted-foreground">
            <p className="text-sm">No staged records for this workflow run.</p>
          </div>
        )}

        {!isLoading && records.length > 0 && flatRecords.length === 0 && (
          <div className="text-center py-12 text-muted-foreground">
            <p className="text-sm">No records match the "{filter}" filter.</p>
            <Button size="sm" variant="ghost" className="mt-2" onClick={handleClearFilter}>
              Show all records
            </Button>
          </div>
        )}

        {flatRecords.length > 0 && (
          <div className="w-full">
            {/* Table header */}
            <div className="grid grid-cols-[20px_28px_minmax(120px,1fr)_80px_60px_60px_minmax(160px,2fr)_100px] gap-2 px-4 py-1.5 text-xs font-medium text-muted-foreground uppercase tracking-wider border-b bg-muted/20 sticky top-0">
              <div />
              <div />
              <div>Name</div>
              <div>Type</div>
              <div>Status</div>
              <div>Conf.</div>
              <div>Details</div>
              <div className="text-right">Actions</div>
            </div>

            {/* Table rows */}
            {flatRecords.map(({ record, targetType, displayName, data }) => (
              <RecordRow
                key={record.id}
                record={record}
                targetType={targetType}
                displayName={displayName}
                data={data}
                isExpanded={expandedId === record.id}
                editingId={editingId}
                editData={editData}
                schemasMap={schemasMap}
                commitErrors={commitErrors}
                retryPending={retryMutation.isPending}
                hasValidationErrors={hasValidationErrors}
                onToggleExpand={toggleExpand}
                onStartEdit={handleStartEdit}
                onApprove={handleApprove}
                onReject={handleReject}
                onRetry={handleRetry}
                onInlineFieldSave={handleInlineFieldSave}
                onEditField={handleEditField}
                onCancelEdit={handleCancelEdit}
                onSaveEdit={handleSaveEdit}
                onStopPropagation={handleStopPropagation}
              />
            ))}
          </div>
        )}

        {batchCommitMutation.isSuccess && (
          <div className={cn(
            'mx-4 my-3 rounded-lg border p-3 text-center',
            batchCommitMutation.data?.errors
              ? 'border-amber-200 bg-amber-50 dark:bg-amber-950/20'
              : 'border-green-200 bg-green-50 dark:bg-green-950/20'
          )}>
            {batchCommitMutation.data?.errors ? (
              <p className="text-xs font-medium text-amber-700">
                {batchCommitMutation.data.committed} committed, {batchCommitMutation.data.errors} failed
              </p>
            ) : (
              <p className="text-xs font-medium text-green-700">Records committed successfully</p>
            )}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}

// ─── Dialog wrapper (backward compat for RunsTab etc.) ───

interface StagingReviewPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workflowRunId: string;
  workflowName?: string;
}

export function StagingReviewPanel({
  open,
  onOpenChange,
  workflowRunId,
  workflowName,
}: StagingReviewPanelProps) {
  if (!open) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[90vw] w-full max-h-[90vh] p-0 flex flex-col overflow-hidden">
        <DialogTitle className="sr-only">Review Staged Records</DialogTitle>
        <StagingReviewContent
          workflowRunId={workflowRunId}
          workflowName={workflowName}
        />
      </DialogContent>
    </Dialog>
  );
}
