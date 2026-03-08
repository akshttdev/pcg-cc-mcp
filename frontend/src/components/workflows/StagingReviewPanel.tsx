import { useState, useMemo, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
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
  Send,
  Edit3,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { stagingApi } from '@/lib/api';
import type { WorkflowStagingRecord } from '@/lib/api';

// Inline confidence badge helper
const ConfidenceBadge = ({ value }: { value: number | null }) => {
  if (value === null || value === undefined) return null;
  const pct = Math.round(value * 100);
  const color = value >= 0.8 ? 'text-green-600 border-green-200'
    : value >= 0.5 ? 'text-amber-600 border-amber-200'
    : 'text-red-600 border-red-200';
  return (
    <Badge variant="outline" className={cn('text-[10px]', color)}>
      {pct}%
    </Badge>
  );
};

const TARGET_TYPE_CONFIG = {
  crm_contact: { label: 'Contacts', icon: Users, color: 'text-blue-500' },
  company: { label: 'Companies', icon: Building2, color: 'text-purple-500' },
  crm_deal: { label: 'Deals', icon: Handshake, color: 'text-green-500' },
  task: { label: 'Tasks', icon: ListTodo, color: 'text-orange-500' },
} as const;

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
  const queryClient = useQueryClient();
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editData, setEditData] = useState<Record<string, any>>({});
  const [filter, setFilter] = useState<'all' | 'valid' | 'duplicates' | 'validation_issues' | 'approved' | 'rejected'>('all');

  const { data: records = [], isLoading } = useQuery({
    queryKey: ['staging', workflowRunId],
    queryFn: () => stagingApi.listByRun(workflowRunId),
    enabled: open && !!workflowRunId,
  });

  const hasValidationErrors = useCallback((r: WorkflowStagingRecord): boolean => {
    if (!r.validation_errors) return false;
    try {
      const errs = JSON.parse(r.validation_errors);
      return Array.isArray(errs) && errs.length > 0;
    } catch { return false; }
  }, []);

  const validationIssueCount = useMemo(() => records.filter(hasValidationErrors).length, [records, hasValidationErrors]);

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
  const duplicatePendingCount = records.filter(r => r.status === 'pending_review' && r.duplicate_of_id != null).length;

  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { status?: string; record_data?: any } }) =>
      stagingApi.update(id, data),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['staging', workflowRunId] }),
  });

  const batchMutation = useMutation({
    mutationFn: ({ ids, action }: { ids: string[]; action: 'approve' | 'reject' }) =>
      stagingApi.batchAction(ids, action),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['staging', workflowRunId] }),
  });

  const commitMutation = useMutation({
    mutationFn: (id: string) => stagingApi.commit(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['staging', workflowRunId] }),
  });

  const batchCommitMutation = useMutation({
    mutationFn: () => stagingApi.batchCommit(workflowRunId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['staging', workflowRunId] }),
  });

  const autoApproveMutation = useMutation({
    mutationFn: () => stagingApi.autoApprove(workflowRunId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['staging', workflowRunId] }),
  });

  const rejectDuplicatesMutation = useMutation({
    mutationFn: () => stagingApi.rejectDuplicates(workflowRunId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['staging', workflowRunId] }),
  });

  const handleApprove = (id: string) => updateMutation.mutate({ id, data: { status: 'approved' } });
  const handleReject = (id: string) => updateMutation.mutate({ id, data: { status: 'rejected' } });

  const handleApproveAll = () => {
    const pendingIds = records.filter(r => r.status === 'pending_review').map(r => r.id);
    if (pendingIds.length > 0) batchMutation.mutate({ ids: pendingIds, action: 'approve' });
  };

  const handleRejectAll = () => {
    const pendingIds = records.filter(r => r.status === 'pending_review').map(r => r.id);
    if (pendingIds.length > 0) batchMutation.mutate({ ids: pendingIds, action: 'reject' });
  };

  const handleStartEdit = (record: WorkflowStagingRecord) => {
    try {
      setEditData(JSON.parse(record.record_data));
      setEditingId(record.id);
    } catch {
      setEditData({});
      setEditingId(record.id);
    }
  };

  const handleSaveEdit = (id: string) => {
    updateMutation.mutate({ id, data: { record_data: editData } });
    setEditingId(null);
  };

  const renderRecordFields = (record: WorkflowStagingRecord) => {
    let data: Record<string, any> = {};
    try { data = JSON.parse(record.record_data); } catch { return null; }

    if (editingId === record.id) {
      return (
        <div className="space-y-2 mt-2">
          {Object.entries(editData).map(([key, value]) => (
            <div key={key} className="flex items-center gap-2">
              <Label className="text-[10px] text-muted-foreground w-24 shrink-0 text-right">{key}</Label>
              <Input
                value={typeof value === 'string' ? value : JSON.stringify(value) || ''}
                onChange={(e) => setEditData(prev => ({ ...prev, [key]: e.target.value }))}
                className="h-7 text-xs"
              />
            </div>
          ))}
          <div className="flex gap-2 justify-end mt-2">
            <Button size="sm" variant="ghost" onClick={() => setEditingId(null)} className="h-6 text-xs">Cancel</Button>
            <Button size="sm" onClick={() => handleSaveEdit(record.id)} className="h-6 text-xs">Save</Button>
          </div>
        </div>
      );
    }

    return (
      <div className="grid grid-cols-2 gap-x-4 gap-y-0.5 mt-2">
        {Object.entries(data).map(([key, value]) => {
          if (value === null || value === undefined) return null;
          const displayValue = typeof value === 'object' ? JSON.stringify(value) : String(value);
          return (
            <div key={key} className="flex items-baseline gap-1.5 py-0.5">
              <span className="text-[10px] text-muted-foreground shrink-0">{key}:</span>
              <span className="text-xs truncate">{displayValue}</span>
            </div>
          );
        })}
      </div>
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[900px] w-full max-h-[85vh] p-0 flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <div>
            <DialogTitle className="text-lg">Review Staged Records</DialogTitle>
            {workflowName && (
              <p className="text-sm text-muted-foreground mt-0.5">{workflowName}</p>
            )}
          </div>
          <div className="flex items-center gap-3">
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <span>{records.length} total</span>
              {avgConfidence !== null && (
                <Badge variant="outline" className={cn('text-[10px]',
                  avgConfidence >= 0.8 ? 'text-green-600 border-green-200'
                    : avgConfidence >= 0.5 ? 'text-amber-600 border-amber-200'
                    : 'text-red-600 border-red-200'
                )}>
                  avg {Math.round(avgConfidence * 100)}%
                </Badge>
              )}
              {duplicateCount > 0 && (
                <Badge variant="outline" className="text-amber-600 border-amber-200">
                  {duplicateCount} duplicates
                </Badge>
              )}
              {validationIssueCount > 0 && (
                <Badge variant="outline" className="text-amber-600 border-amber-200">
                  {validationIssueCount} validation issues
                </Badge>
              )}
              <Badge variant="outline">{pendingCount} pending</Badge>
              <Badge variant="outline" className="text-green-600 border-green-200">{approvedCount} approved</Badge>
              <Badge variant="outline" className="text-blue-600 border-blue-200">{committedCount} committed</Badge>
            </div>
          </div>
        </div>

        {/* Filter tabs + smart actions toolbar */}
        {records.length > 0 && (
          <div className="flex items-center gap-2 px-6 py-2 border-b bg-muted/30">
            {/* Filter tabs */}
            <div className="flex items-center gap-1 mr-2">
              {(['all', 'valid', 'duplicates', 'validation_issues', 'approved', 'rejected'] as const).map((f) => {
                const counts = {
                  all: records.length,
                  valid: validPendingCount,
                  duplicates: duplicateCount,
                  validation_issues: validationIssueCount,
                  approved: approvedCount,
                  rejected: records.filter(r => r.status === 'rejected').length,
                };
                if (counts[f] === 0 && f !== 'all') return null;
                const labels: Record<string, string> = {
                  all: 'All',
                  valid: 'Valid',
                  duplicates: 'Duplicates',
                  validation_issues: 'Validation Issues',
                  approved: 'Approved',
                  rejected: 'Rejected',
                };
                return (
                  <button
                    key={f}
                    onClick={() => setFilter(f)}
                    className={cn(
                      'px-2 py-1 rounded text-xs transition-colors',
                      filter === f
                        ? 'bg-primary text-primary-foreground'
                        : 'hover:bg-muted text-muted-foreground',
                      f === 'validation_issues' && counts[f] > 0 && filter !== f && 'text-amber-600'
                    )}
                  >
                    {labels[f]}
                    {counts[f] > 0 && ` (${counts[f]})`}
                  </button>
                );
              })}
            </div>

            <div className="flex-1" />

            {/* Smart action buttons */}
            {duplicatePendingCount > 0 && (
              <Button
                size="sm"
                variant="outline"
                className="h-7 text-xs gap-1 text-amber-600 border-amber-200 hover:bg-amber-50"
                onClick={() => rejectDuplicatesMutation.mutate()}
                disabled={rejectDuplicatesMutation.isPending}
              >
                <XCircle className="h-3 w-3" />
                {rejectDuplicatesMutation.isPending ? 'Removing...' : `Remove ${duplicatePendingCount} duplicates`}
              </Button>
            )}

            {validPendingCount > 0 && (
              <Button
                size="sm"
                variant="outline"
                className="h-7 text-xs gap-1 text-green-600 border-green-200 hover:bg-green-50"
                onClick={() => autoApproveMutation.mutate()}
                disabled={autoApproveMutation.isPending}
              >
                <CheckCheck className="h-3 w-3" />
                {autoApproveMutation.isPending ? 'Approving...' : `Auto-approve ${validPendingCount} valid (\u226570% confidence)`}
              </Button>
            )}

            {approvedCount > 0 && (
              <Button
                size="sm"
                className="h-7 text-xs gap-1"
                onClick={() => batchCommitMutation.mutate()}
                disabled={batchCommitMutation.isPending}
              >
                <Send className="h-3 w-3" />
                {batchCommitMutation.isPending ? 'Committing...' : `Commit ${approvedCount} to CRM`}
              </Button>
            )}
          </div>
        )}

        {/* Records list */}
        <ScrollArea className="flex-1">
          <div className="p-6 space-y-6">
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

            {!isLoading && records.length > 0 && Object.keys(grouped).length === 0 && (
              <div className="text-center py-12 text-muted-foreground">
                <p className="text-sm">No records match the "{filter}" filter.</p>
                <Button size="sm" variant="ghost" className="mt-2" onClick={() => setFilter('all')}>
                  Show all records
                </Button>
              </div>
            )}

            {Object.entries(grouped).map(([targetType, groupRecords]) => {
              const config = TARGET_TYPE_CONFIG[targetType as keyof typeof TARGET_TYPE_CONFIG];
              if (!config) return null;
              const Icon = config.icon;

              return (
                <div key={targetType}>
                  <div className="flex items-center gap-2 mb-3">
                    <Icon className={cn('h-4 w-4', config.color)} />
                    <h3 className="text-sm font-medium">{config.label}</h3>
                    <Badge variant="outline" className="text-[10px]">{groupRecords.length}</Badge>
                  </div>

                  <div className="space-y-2">
                    {groupRecords.map((record) => {
                      let data: Record<string, any> = {};
                      try { data = JSON.parse(record.record_data); } catch {}
                      const displayName = data.first_name
                        ? `${data.first_name} ${data.last_name || ''}`
                        : data.name || data.title || 'Untitled';

                      return (
                        <div
                          key={record.id}
                          className={cn(
                            'rounded-lg border p-3',
                            record.status === 'approved' && 'border-green-200 bg-green-50/50 dark:bg-green-950/20',
                            record.status === 'rejected' && 'border-red-200 bg-red-50/50 dark:bg-red-950/20 opacity-60',
                            record.status === 'committed' && 'border-blue-200 bg-blue-50/50 dark:bg-blue-950/20',
                            record.status === 'error' && 'border-red-300 bg-red-50 dark:bg-red-950/30',
                            hasValidationErrors(record) && record.status !== 'rejected' && 'border-amber-300 bg-amber-50/30 dark:bg-amber-950/10',
                          )}
                        >
                          <div className="flex items-center gap-2">
                            <span className="text-sm font-medium flex-1">{displayName}</span>

                            {record.duplicate_of_id && (
                              <Badge variant="outline" className="text-[10px] text-amber-600 border-amber-200 gap-1">
                                <AlertTriangle className="h-2.5 w-2.5" /> Potential duplicate
                              </Badge>
                            )}

                            {record.validation_errors && (() => {
                              try {
                                const errs = JSON.parse(record.validation_errors);
                                return errs.length > 0 ? (
                                  <Badge variant="outline" className="text-[10px] text-amber-600 border-amber-200 gap-1">
                                    <AlertTriangle className="h-2.5 w-2.5" /> {errs.length} validation {errs.length === 1 ? 'issue' : 'issues'}
                                  </Badge>
                                ) : null;
                              } catch { return null; }
                            })()}

                            <Badge
                              variant="outline"
                              className={cn('text-[10px]', {
                                'text-amber-600 border-amber-200': record.status === 'pending_review',
                                'text-green-600 border-green-200': record.status === 'approved',
                                'text-red-600 border-red-200': record.status === 'rejected',
                                'text-blue-600 border-blue-200': record.status === 'committed',
                                'text-red-700 border-red-300': record.status === 'error',
                              })}
                            >
                              {record.status.replace('_', ' ')}
                            </Badge>

                            <ConfidenceBadge value={record.confidence} />

                            {record.status === 'pending_review' && (
                              <div className="flex items-center gap-1">
                                <button
                                  onClick={() => handleStartEdit(record)}
                                  className="p-1 rounded hover:bg-muted transition-colors"
                                  title="Edit"
                                >
                                  <Edit3 className="h-3.5 w-3.5 text-muted-foreground" />
                                </button>
                                <button
                                  onClick={() => handleApprove(record.id)}
                                  className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900 transition-colors"
                                  title="Approve"
                                >
                                  <Check className="h-3.5 w-3.5 text-green-600" />
                                </button>
                                <button
                                  onClick={() => handleReject(record.id)}
                                  className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900 transition-colors"
                                  title="Reject"
                                >
                                  <X className="h-3.5 w-3.5 text-red-500" />
                                </button>
                              </div>
                            )}

                            {record.status === 'approved' && (
                              <button
                                onClick={() => commitMutation.mutate(record.id)}
                                className="p-1 rounded hover:bg-blue-100 dark:hover:bg-blue-900 transition-colors"
                                title="Commit to CRM"
                                disabled={commitMutation.isPending}
                              >
                                <Send className="h-3.5 w-3.5 text-blue-500" />
                              </button>
                            )}
                          </div>

                          {record.error_message && (
                            <p className="text-xs text-red-600 mt-1">{record.error_message}</p>
                          )}

                          {(() => {
                            let validationErrs: string[] = [];
                            if (record.validation_errors) {
                              try { validationErrs = JSON.parse(record.validation_errors); } catch {}
                            }
                            return validationErrs.length > 0 ? (
                              <div className="flex items-start gap-1.5 mt-1.5 p-2 rounded bg-amber-50 dark:bg-amber-950/30 border border-amber-200 dark:border-amber-800">
                                <AlertTriangle className="h-3.5 w-3.5 text-amber-500 shrink-0 mt-0.5" />
                                <div className="space-y-0.5">
                                  <p className="text-[11px] font-medium text-amber-700 dark:text-amber-400">Validation warnings</p>
                                  {validationErrs.map((err, i) => (
                                    <p key={i} className="text-[10px] text-amber-600 dark:text-amber-500">{err}</p>
                                  ))}
                                </div>
                              </div>
                            ) : null;
                          })()}

                          {renderRecordFields(record)}
                        </div>
                      );
                    })}
                  </div>
                </div>
              );
            })}

            {batchCommitMutation.isSuccess && (
              <div className="rounded-lg border border-green-200 bg-green-50 dark:bg-green-950/20 p-4 text-center">
                <CheckCheck className="h-5 w-5 text-green-600 mx-auto mb-1" />
                <p className="text-sm font-medium text-green-700">Records committed successfully</p>
              </div>
            )}
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
