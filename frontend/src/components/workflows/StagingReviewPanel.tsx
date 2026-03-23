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
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Users,
  Building2,
  Handshake,
  ListTodo,
  Check,
  X,
  CheckCheck,
  XCircle,
  Loader2,
  AlertTriangle,
  Edit3,
  RotateCcw,
  ArrowLeft,
  ChevronRight,
  Plus,
} from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { stagingApi, schemasApi } from '@/lib/api';
import type { WorkflowStagingRecord, CommitResult, FieldDef, TargetSchema } from '@/lib/api';

// Inline confidence badge helper
const ConfidenceBadge = ({ value }: { value: number | null }) => {
  if (value === null || value === undefined) return null;
  const pct = Math.round(value * 100);
  const color = value >= 0.8 ? 'text-green-600 border-green-200'
    : value >= 0.5 ? 'text-amber-600 border-amber-200'
    : 'text-red-600 border-red-200';
  return (
    <Badge variant="outline" className={cn('text-xs', color)}>
      {pct}%
    </Badge>
  );
};

// Inline editable field — schema-aware (click-to-edit)
function InlineField({
  fieldKey,
  value,
  recordId,
  onSave,
  fieldDef,
}: {
  fieldKey: string;
  value: string;
  recordId: string;
  onSave: (recordId: string, key: string, value: string) => void;
  fieldDef?: FieldDef;
}) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(value);
  const inputRef = useCallback((el: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | null) => {
    if (el) el.focus();
  }, []);

  const handleSave = useCallback(() => {
    setEditing(false);
    if (editValue !== value) onSave(recordId, fieldKey, editValue);
  }, [editValue, value, recordId, fieldKey, onSave]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); handleSave(); }
    if (e.key === 'Escape') { setEditValue(value); setEditing(false); }
  }, [handleSave, value]);

  if (editing) {
    const hasEnum = fieldDef?.enum_values && fieldDef.enum_values.length > 0;
    const isLong = value.length > 80 || fieldDef?.type === 'array';
    const inputType = fieldDef?.type === 'number' ? 'number'
      : fieldDef?.format === 'email' ? 'email'
      : fieldDef?.format === 'url' ? 'url'
      : (fieldDef?.format === 'date' || fieldDef?.format === 'date-time') ? 'date'
      : 'text';

    if (hasEnum) {
      return (
        <select ref={inputRef} value={editValue} onChange={e => { setEditValue(e.target.value); }}
          onBlur={handleSave} className="w-full text-xs bg-background border rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring">
          <option value="">— select —</option>
          {fieldDef!.enum_values!.map(v => <option key={v} value={v}>{v.replace(/_/g, ' ')}</option>)}
        </select>
      );
    }
    if (isLong) {
      return (
        <textarea ref={inputRef} value={editValue} onChange={e => setEditValue(e.target.value)}
          onBlur={handleSave} onKeyDown={handleKeyDown}
          placeholder={fieldDef?.type === 'array' ? 'Comma-separated values' : undefined}
          className="w-full text-xs bg-background border rounded px-1.5 py-1 focus:outline-none focus:ring-1 focus:ring-ring min-h-[60px] resize-y" />
      );
    }
    return (
      <input ref={inputRef} type={inputType} value={editValue} onChange={e => setEditValue(e.target.value)}
        onBlur={handleSave} onKeyDown={handleKeyDown}
        className="w-full text-xs bg-background border rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring" />
    );
  }

  return (
    <dd
      onClick={() => { setEditValue(value); setEditing(true); }}
      className="text-xs mt-0.5 cursor-text hover:bg-muted/40 rounded px-1 -mx-1 transition-colors truncate"
      title={`Click to edit${fieldDef ? ` (${fieldDef.type})` : ''}`}
    >
      {value || <span className="text-muted-foreground italic">empty</span>}
    </dd>
  );
}

const TARGET_TYPE_CONFIG = {
  crm_contact: { label: 'Contacts', icon: Users, color: 'text-blue-500' },
  company: { label: 'Companies', icon: Building2, color: 'text-purple-500' },
  crm_deal: { label: 'Deals', icon: Handshake, color: 'text-green-500' },
  task: { label: 'Tasks', icon: ListTodo, color: 'text-orange-500' },
} as const;

// ─── Record action buttons (extracted to avoid anonymous functions in props) ───

function RecordActions({
  record,
  onEdit,
  onApprove,
  onReject,
  onRetry,
  retryPending,
  onStopPropagation,
}: {
  record: WorkflowStagingRecord;
  onEdit: (record: WorkflowStagingRecord) => void;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
  onRetry: (id: string) => void;
  retryPending: boolean;
  onStopPropagation: (e: React.MouseEvent) => void;
}) {
  const handleEdit = useCallback(() => onEdit(record), [onEdit, record]);
  const handleApprove = useCallback(() => onApprove(record.id), [onApprove, record.id]);
  const handleReject = useCallback(() => onReject(record.id), [onReject, record.id]);
  const handleRetry = useCallback(() => onRetry(record.id), [onRetry, record.id]);

  return (
    <div className="flex items-center gap-0.5 justify-end" onClick={onStopPropagation}>
      {record.status === 'pending_review' && (
        <>
          <button onClick={handleEdit} className="p-1 rounded hover:bg-muted" title="Edit">
            <Edit3 className="h-3 w-3 text-muted-foreground" />
          </button>
          <button onClick={handleApprove} className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900" title="Approve & Commit">
            <Check className="h-3 w-3 text-green-600" />
          </button>
          <button onClick={handleReject} className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900" title="Reject">
            <X className="h-3 w-3 text-red-500" />
          </button>
        </>
      )}
      {record.status === 'error' && (
        <>
          <button onClick={handleEdit} className="p-1 rounded hover:bg-muted" title="Edit">
            <Edit3 className="h-3 w-3 text-muted-foreground" />
          </button>
          <button onClick={handleRetry} className="p-1 rounded hover:bg-amber-100 dark:hover:bg-amber-900" title="Retry" disabled={retryPending}>
            <RotateCcw className={cn('h-3 w-3 text-amber-600', retryPending && 'animate-spin')} />
          </button>
        </>
      )}
    </div>
  );
}

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

  const renderRecordFields = useCallback((record: WorkflowStagingRecord) => {
    try { JSON.parse(record.record_data); } catch { return null; }

    if (editingId === record.id) {
      return (
        <div className="px-4 py-2 bg-muted/20 border-b space-y-1.5">
          <div className="grid grid-cols-2 gap-2">
            {Object.entries(editData).map(([key, value]) => (
              <div key={key} className="flex items-center gap-2">
                <Label className="text-xs text-muted-foreground w-20 shrink-0 text-right">{key}</Label>
                <Input
                  value={value == null ? '' : typeof value === 'string' ? value : JSON.stringify(value)}
                  onChange={(e) => handleEditField(key, e.target.value)}
                  className="h-6 text-xs"
                />
              </div>
            ))}
          </div>
          <div className="flex gap-2 justify-end">
            <Button size="sm" variant="ghost" onClick={handleCancelEdit} className="h-5 text-xs px-2">Cancel</Button>
            <Button size="sm" onClick={() => handleSaveEdit(record.id)} className="h-5 text-xs px-2">Save</Button>
          </div>
        </div>
      );
    }

    return null;
  }, [editingId, editData, handleEditField, handleCancelEdit, handleSaveEdit]);

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
            {flatRecords.map(({ record, targetType, displayName, data }) => {
              const config = TARGET_TYPE_CONFIG[targetType as keyof typeof TARGET_TYPE_CONFIG];
              const Icon = config?.icon || Users;
              const isExpanded = expandedId === record.id;

              // Get key detail fields (first 3 non-name fields)
              const detailFields = Object.entries(data)
                .filter(([k, v]) => v != null && !['first_name', 'last_name', 'name', 'title'].includes(k))
                .slice(0, 4);

              // All data fields for expanded view
              const allFields = Object.entries(data).filter(([, v]) => v != null && v !== '');

              let validationErrs: string[] = [];
              if (record.validation_errors) {
                try { validationErrs = JSON.parse(record.validation_errors); } catch { /* intentionally empty */ }
              }

              return (
                <div key={record.id}>
                  <div
                    role="row"
                    tabIndex={0}
                    onClick={() => toggleExpand(record.id)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        toggleExpand(record.id);
                      }
                    }}
                    className={cn(
                      'grid grid-cols-[20px_28px_minmax(120px,1fr)_80px_60px_60px_minmax(160px,2fr)_100px] gap-2 px-4 py-1.5 items-center border-b text-xs hover:bg-muted/30 transition-colors cursor-pointer select-none focus:outline-none focus:ring-1 focus:ring-ring focus:ring-inset',
                      record.status === 'approved' && 'bg-green-50/30 dark:bg-green-950/10',
                      record.status === 'rejected' && 'bg-red-50/30 dark:bg-red-950/10 opacity-50',
                      record.status === 'committed' && 'bg-blue-50/30 dark:bg-blue-950/10',
                      record.status === 'error' && 'bg-red-50/50 dark:bg-red-950/20',
                      hasValidationErrors(record) && record.status === 'pending_review' && 'bg-amber-50/20 dark:bg-amber-950/5',
                      isExpanded && 'bg-muted/40 border-b-0',
                    )}
                  >
                    {/* Expand chevron */}
                    <ChevronRight className={cn('h-3.5 w-3.5 text-muted-foreground transition-transform', isExpanded && 'rotate-90')} />

                    {/* Type icon */}
                    <Icon className={cn('h-3.5 w-3.5', config?.color || 'text-muted-foreground')} />

                    {/* Name + flags */}
                    <div className="flex items-center gap-1.5 min-w-0">
                      <span className="font-medium truncate">{displayName}</span>
                      {record.duplicate_of_id && (
                        <span title="Potential duplicate"><AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" /></span>
                      )}
                      {validationErrs.length > 0 && !record.duplicate_of_id && (
                        <span title={`${validationErrs.length} validation issue(s)`}><AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" /></span>
                      )}
                    </div>

                    {/* Type label */}
                    <span className="text-xs text-muted-foreground">{config?.label || targetType}</span>

                    {/* Status */}
                    <Badge
                      variant="outline"
                      className={cn('text-[9px] h-5 px-1.5 justify-center', {
                        'text-amber-600 border-amber-200': record.status === 'pending_review',
                        'text-green-600 border-green-200': record.status === 'approved',
                        'text-red-600 border-red-200': record.status === 'rejected',
                        'text-blue-600 border-blue-200': record.status === 'committed',
                        'text-red-700 border-red-300': record.status === 'error',
                      })}
                    >
                      {record.status === 'pending_review' ? 'pending' : record.status}
                    </Badge>

                    {/* Confidence */}
                    <ConfidenceBadge value={record.confidence} />

                    {/* Key detail fields inline */}
                    <div className="flex items-center gap-3 min-w-0 overflow-hidden">
                      {detailFields.map(([key, value]) => {
                        const dv = value == null ? '' : typeof value === 'object' ? JSON.stringify(value) : String(value);
                        return (
                          <span key={key} className="text-xs text-muted-foreground truncate">
                            <span className="opacity-60">{key}:</span> {dv}
                          </span>
                        );
                      })}
                    </div>

                    {/* Actions — stop propagation so clicks don't toggle expand */}
                    <RecordActions
                      record={record}
                      onEdit={handleStartEdit}
                      onApprove={handleApprove}
                      onReject={handleReject}
                      onRetry={handleRetry}
                      retryPending={retryMutation.isPending}
                      onStopPropagation={handleStopPropagation}
                    />
                  </div>

                  {/* Expanded detail panel — schema-aware inline editing */}
                  {isExpanded && editingId !== record.id && (() => {
                    const schema = schemasMap[record.target_type];
                    const schemaFields = schema?.fields || {};
                    const existingKeys = new Set(allFields.map(([k]) => k));
                    const missingFields = Object.entries(schemaFields).filter(([k]) => !existingKeys.has(k));
                    const canEdit = record.status === 'pending_review';

                    return (
                    <div className="border-b bg-muted/20 px-4 py-3" onClick={handleStopPropagation}>
                      <div className="grid grid-cols-[1fr_1fr] lg:grid-cols-[1fr_1fr_1fr] gap-x-6 gap-y-2">
                        {allFields.map(([key, value]) => {
                          const dv = value == null ? '' : typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);
                          const isLong = dv.length > 80;
                          const fd = schemaFields[key];
                          return (
                            <div key={key} className={cn(isLong && 'col-span-2 lg:col-span-3')}>
                              <dt className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                                {key.replace(/_/g, ' ')}
                                {fd?.required && <span className="text-red-400 ml-0.5">*</span>}
                              </dt>
                              {canEdit ? (
                                <InlineField
                                  fieldKey={key}
                                  value={dv}
                                  recordId={record.id}
                                  onSave={handleInlineFieldSave}
                                  fieldDef={fd}
                                />
                              ) : (
                                <dd className={cn('text-xs mt-0.5', isLong ? 'whitespace-pre-wrap break-words' : 'truncate')}>
                                  {dv || <span className="text-muted-foreground italic">empty</span>}
                                </dd>
                              )}
                            </div>
                          );
                        })}
                      </div>

                      {/* Add field dropdown for missing schema fields */}
                      {canEdit && missingFields.length > 0 && (
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <button className="mt-2 flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors">
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
                                <span className="ml-auto text-xs text-muted-foreground pl-4">{fieldDef.type}</span>
                              </DropdownMenuItem>
                            ))}
                          </DropdownMenuContent>
                        </DropdownMenu>
                      )}

                      {/* Metadata row */}
                      <div className="flex items-center gap-4 mt-3 pt-2 border-t border-muted text-xs text-muted-foreground">
                        <span>ID: <code className="text-[9px]">{record.id.slice(0, 8)}</code></span>
                        <span>Run: <code className="text-[9px]">{record.workflow_run_id.slice(0, 8)}</code></span>
                        {record.duplicate_of_id && (
                          <span className="text-amber-600">Duplicate of: <code className="text-[9px]">{record.duplicate_of_id.slice(0, 8)}</code></span>
                        )}
                        {(record as any).committed_entity_id && (
                          <span className="text-blue-600">Entity: <code className="text-[9px]">{(record as any).committed_entity_id.slice(0, 8)}</code></span>
                        )}
                        {record.created_at && <span>Created: {new Date(record.created_at).toLocaleString()}</span>}
                      </div>

                      {/* Validation errors in expanded view */}
                      {validationErrs.length > 0 && (
                        <div className="mt-2 pt-2 border-t border-amber-200 dark:border-amber-800">
                          <p className="text-xs font-medium text-amber-600 mb-1">Validation Issues</p>
                          <ul className="space-y-0.5">
                            {validationErrs.map((err, i) => (
                              <li key={i} className="text-xs text-amber-600 flex items-start gap-1.5">
                                <AlertTriangle className="h-3 w-3 shrink-0 mt-0.5" />
                                {err}
                              </li>
                            ))}
                          </ul>
                        </div>
                      )}

                      {/* Error message in expanded view */}
                      {(record.error_message || commitErrors[record.id]) && (
                        <div className="mt-2 pt-2 border-t border-red-200 dark:border-red-800">
                          <p className="text-xs font-medium text-red-600 mb-1">Error</p>
                          <p className="text-xs text-red-600">{record.error_message || commitErrors[record.id]}</p>
                        </div>
                      )}
                    </div>
                    );
                  })()}

                  {/* Expandable: edit form */}
                  {editingId === record.id && renderRecordFields(record)}

                  {/* Compact inline warnings when NOT expanded */}
                  {!isExpanded && (record.error_message || commitErrors[record.id]) && (
                    <div className="px-4 py-1 bg-red-50/50 dark:bg-red-950/20 border-b">
                      <p className="text-xs text-red-600">{record.error_message || commitErrors[record.id]}</p>
                    </div>
                  )}

                  {!isExpanded && validationErrs.length > 0 && record.status !== 'rejected' && editingId !== record.id && (
                    <div className="px-4 py-1 bg-amber-50/50 dark:bg-amber-950/10 border-b flex items-center gap-1.5">
                      <AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" />
                      <span className="text-xs text-amber-600">{validationErrs.join(' · ')}</span>
                    </div>
                  )}
                </div>
              );
            })}
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
