import { Badge } from '@/components/ui/badge';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  ChevronRight,
  AlertTriangle,
  Users,
  Plus,
  Check,
  RotateCcw,
  X,
  Info,
} from 'lucide-react';
import type { WorkflowStagingRecord, TargetSchema } from '@/lib/api';
import { STAGING_TARGET_CONFIG } from '../../constants';
import { InlineEditField } from '../../components/InlineEditField';

interface TableRowData {
  record: WorkflowStagingRecord;
  displayName: string;
  data: Record<string, unknown>;
  workflowName: string;
}

interface StagingTableRowProps {
  row: TableRowData;
  isExpanded: boolean;
  onToggleExpand: (id: string) => void;
  onStopPropagation: (e: React.MouseEvent) => void;
  onApprove: (id: string) => void;
  onReject: (id: string) => void;
  onRetry: (id: string) => void;
  onInlineFieldSave: (recordId: string, fieldKey: string, newValue: string) => void;
  isRetrying: boolean;
  commonWarningSet: Set<string>;
  schemasMap: Record<string, TargetSchema>;
}

export function StagingTableRow({
  row,
  isExpanded,
  onToggleExpand,
  onStopPropagation,
  onApprove,
  onReject,
  onRetry,
  onInlineFieldSave,
  isRetrying,
  commonWarningSet,
  schemasMap,
}: StagingTableRowProps) {
  const { record, displayName, data, workflowName } = row;
  const config = STAGING_TARGET_CONFIG[record.target_type];
  const Icon = config?.icon || Users;

  const detailFields = Object.entries(data)
    .filter(([k, v]) => v != null && !['first_name', 'last_name', 'name', 'title'].includes(k))
    .slice(0, 3);

  const allFields = Object.entries(data).filter(([, v]) => v != null && v !== '');

  let validationErrs: string[] = [];
  if (record.validation_errors) {
    try { validationErrs = JSON.parse(record.validation_errors); } catch { /* intentionally empty */ }
  }

  const hasCommonWarning = validationErrs.some(e => commonWarningSet.has(e));
  const rowSpecificErrs = validationErrs.filter(e => !commonWarningSet.has(e));

  const schema = schemasMap[record.target_type];
  const schemaFields = schema?.fields || {};
  const existingKeys = new Set(allFields.map(([k]) => k));
  const missingFields = Object.entries(schemaFields).filter(([k]) => !existingKeys.has(k));
  const canEdit = record.status === 'pending_review';

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

        <span className="text-xs text-muted-foreground truncate">{workflowName || record.workflow_run_id.slice(0, 8)}</span>
        <span className="text-xs text-muted-foreground">{config?.label || record.target_type}</span>

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
          <Badge variant="outline" className={`text-xs ${
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
              <span key={key} className="text-xs text-muted-foreground truncate">
                <span className="opacity-60">{key}:</span> {dv}
              </span>
            );
          })}
        </div>

        <div className="flex items-center gap-0.5 justify-end" onClick={onStopPropagation}>
          {record.status === 'pending_review' && (
            <>
              <button onClick={() => onApprove(record.id)} className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900" title="Approve & Commit">
                <Check className="h-3 w-3 text-green-600" />
              </button>
              <button onClick={() => onReject(record.id)} className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900" title="Reject">
                <X className="h-3 w-3 text-red-500" />
              </button>
            </>
          )}
          {record.status === 'error' && (
            <button onClick={() => onRetry(record.id)} className="p-1 rounded hover:bg-amber-100 dark:hover:bg-amber-900" title="Retry" disabled={isRetrying}>
              <RotateCcw className="h-3 w-3 text-amber-600" />
            </button>
          )}
        </div>
      </div>

      {/* Expanded detail panel with inline editing */}
      {isExpanded && (
        <div className="border-b bg-muted/20 px-4 py-3" onClick={onStopPropagation}>
          <div className="grid grid-cols-[1fr_1fr] lg:grid-cols-[1fr_1fr_1fr] gap-x-6 gap-y-2">
            {allFields.map(([key, value]) => {
              const dv = value == null ? '' : typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);
              const isLong = dv.length > 80;
              return (
                <div key={key} className={isLong ? 'col-span-2 lg:col-span-3' : ''}>
                  <dt className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                    {key.replace(/_/g, ' ')}
                    {schemaFields[key]?.required && <span className="text-red-400 ml-0.5">*</span>}
                  </dt>
                  <InlineEditField
                    fieldKey={key}
                    value={dv}
                    recordId={record.id}
                    onSave={onInlineFieldSave}
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

          <div className="flex items-center gap-4 mt-3 pt-2 border-t border-muted text-xs text-muted-foreground">
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
        </div>
      )}

      {/* Compact inline warnings when NOT expanded */}
      {!isExpanded && rowSpecificErrs.length > 0 && record.status !== 'rejected' && (
        <div className="px-4 py-1 bg-amber-50/50 dark:bg-amber-950/10 border-b flex items-center gap-1.5">
          <AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" />
          <span className="text-xs text-amber-600">{rowSpecificErrs.join(' \u00b7 ')}</span>
        </div>
      )}
    </div>
  );
}
