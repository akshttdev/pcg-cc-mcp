import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Users,
  AlertTriangle,
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
import type { WorkflowStagingRecord, FieldDef } from '@/lib/api';

import { ConfidenceBadge } from './ConfidenceBadge';
import { InlineField } from './InlineField';
import { RecordActions } from './RecordActions';
import { TARGET_TYPE_CONFIG } from './constants';

interface RecordRowProps {
  record: WorkflowStagingRecord;
  targetType: string;
  displayName: string;
  data: Record<string, unknown>;
  isExpanded: boolean;
  editingId: string | null;
  editData: Record<string, unknown>;
  schemasMap: Record<string, { fields?: Record<string, FieldDef> }>;
  commitErrors: Record<string, string>;
  retryPending: boolean;
  hasValidationErrors: (r: WorkflowStagingRecord) => boolean;
  onToggleExpand: (id: string) => void;
  onStartEdit: (record: WorkflowStagingRecord) => void;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
  onRetry: (id: string) => void;
  onInlineFieldSave: (recordId: string, key: string, value: string) => void;
  onEditField: (key: string, value: string) => void;
  onCancelEdit: () => void;
  onSaveEdit: (id: string) => void;
  onStopPropagation: (e: React.MouseEvent) => void;
}

export function RecordRow({
  record,
  targetType,
  displayName,
  data,
  isExpanded,
  editingId,
  editData,
  schemasMap,
  commitErrors,
  retryPending,
  hasValidationErrors,
  onToggleExpand,
  onStartEdit,
  onApprove,
  onReject,
  onRetry,
  onInlineFieldSave,
  onEditField,
  onCancelEdit,
  onSaveEdit,
  onStopPropagation,
}: RecordRowProps) {
  const config = TARGET_TYPE_CONFIG[targetType as keyof typeof TARGET_TYPE_CONFIG];
  const Icon = config?.icon || Users;

  // Get key detail fields (first 4 non-name fields)
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
    <div>
      <div
        role="row"
        tabIndex={0}
        onClick={() => onToggleExpand(record.id)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            onToggleExpand(record.id);
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

        {/* Actions */}
        <RecordActions
          record={record}
          onEdit={onStartEdit}
          onApprove={onApprove}
          onReject={onReject}
          onRetry={onRetry}
          retryPending={retryPending}
          onStopPropagation={onStopPropagation}
        />
      </div>

      {/* Expanded detail panel */}
      {isExpanded && editingId !== record.id && (() => {
        const schema = schemasMap[record.target_type];
        const schemaFields = schema?.fields || {};
        const existingKeys = new Set(allFields.map(([k]) => k));
        const missingFields = Object.entries(schemaFields).filter(([k]) => !existingKeys.has(k));
        const canEdit = record.status === 'pending_review';

        return (
        <div className="border-b bg-muted/20 px-4 py-3" onClick={onStopPropagation}>
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
                      onSave={onInlineFieldSave}
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
                    onClick={() => onInlineFieldSave(record.id, fieldName, '')}
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
      {editingId === record.id && (
        <div className="px-4 py-2 bg-muted/20 border-b space-y-1.5">
          <div className="grid grid-cols-2 gap-2">
            {Object.entries(editData).map(([key, value]) => (
              <div key={key} className="flex items-center gap-2">
                <Label className="text-xs text-muted-foreground w-20 shrink-0 text-right">{key}</Label>
                <Input
                  value={value == null ? '' : typeof value === 'string' ? value : JSON.stringify(value)}
                  onChange={(e) => onEditField(key, e.target.value)}
                  className="h-6 text-xs"
                />
              </div>
            ))}
          </div>
          <div className="flex gap-2 justify-end">
            <Button size="sm" variant="ghost" onClick={onCancelEdit} className="h-5 text-xs px-2">Cancel</Button>
            <Button size="sm" onClick={() => onSaveEdit(record.id)} className="h-5 text-xs px-2">Save</Button>
          </div>
        </div>
      )}

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
}
