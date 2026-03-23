import {
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Info,
} from 'lucide-react';
import { STAGING_TARGET_CONFIG } from '../../constants';
import type { GlobalFilterType } from '../../types';

interface FilterTab {
  key: GlobalFilterType | 'all';
  label: string;
  count: number;
  color: string;
}

interface CommonWarning {
  msg: string;
  count: number;
}

interface StagingFilterToolbarProps {
  globalFilter: GlobalFilterType | 'all';
  updateParams: (updates: Record<string, string | null>) => void;
  filterTabs: FilterTab[];
  uniqueTypes: string[];
  typeFilter: string;
  uniqueWorkflows: [string, string][];
  workflowFilter: string;
  filteredCount: number;
  commonWarnings: CommonWarning[];
  sortField: string;
  sortDir: 'asc' | 'desc';
  handleSort: (field: 'name' | 'type' | 'workflow' | 'status' | 'confidence') => void;
}

export function StagingFilterToolbar({
  globalFilter,
  updateParams,
  filterTabs,
  uniqueTypes,
  typeFilter,
  uniqueWorkflows,
  workflowFilter,
  filteredCount,
  commonWarnings,
  sortField,
  sortDir,
  handleSort,
}: StagingFilterToolbarProps) {
  const SortIcon = ({ field }: { field: string }) => {
    if (sortField === field) {
      return sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />;
    }
    return <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />;
  };

  return (
    <>
      {/* Filter toolbar */}
      <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/20 flex-wrap">
        {/* Status filter tabs */}
        <div className="flex items-center gap-0.5 mr-2">
          {filterTabs.map(({ key, label, count, color }) => {
            if (count === 0 && key !== 'all') return null;
            return (
              <button
                key={key}
                onClick={() => updateParams({ filter: key })}
                className={`px-2 py-0.5 rounded text-xs transition-colors ${
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
            className="h-6 text-xs rounded border bg-background px-1.5 text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
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
            className="h-6 text-xs rounded border bg-background px-1.5 text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring max-w-[200px]"
          >
            <option value="all">All workflows</option>
            {uniqueWorkflows.map(([id, name]) => (
              <option key={id} value={id}>{name}</option>
            ))}
          </select>
        )}

        <div className="flex-1" />
        <span className="text-xs text-muted-foreground">{filteredCount} record{filteredCount !== 1 ? 's' : ''}</span>
      </div>

      {/* Common warnings banner */}
      {commonWarnings.length > 0 && (
        <div className="px-4 py-2 border-b bg-blue-50/60 dark:bg-blue-950/15 flex items-start gap-2">
          <Info className="h-3.5 w-3.5 text-blue-400 shrink-0 mt-0.5" />
          <div className="text-xs text-blue-700 dark:text-blue-400">
            {commonWarnings.map(w => (
              <div key={w.msg}>{w.msg} <span className="text-blue-500">({w.count} records)</span></div>
            ))}
          </div>
        </div>
      )}

      {/* Sortable table header */}
      <div className="grid grid-cols-[20px_28px_minmax(120px,1fr)_minmax(100px,0.7fr)_80px_60px_60px_minmax(160px,1.5fr)_100px] gap-2 px-4 py-1.5 text-xs font-medium text-muted-foreground uppercase tracking-wider border-b bg-muted/10">
        <div />
        <div />
        <button onClick={() => handleSort('name')} className="flex items-center gap-1 hover:text-foreground text-left">
          Name <SortIcon field="name" />
        </button>
        <button onClick={() => handleSort('workflow')} className="flex items-center gap-1 hover:text-foreground text-left">
          Workflow <SortIcon field="workflow" />
        </button>
        <button onClick={() => handleSort('type')} className="flex items-center gap-1 hover:text-foreground text-left">
          Type <SortIcon field="type" />
        </button>
        <button onClick={() => handleSort('status')} className="flex items-center gap-1 hover:text-foreground text-left">
          Status <SortIcon field="status" />
        </button>
        <button onClick={() => handleSort('confidence')} className="flex items-center gap-1 hover:text-foreground text-left">
          Conf. <SortIcon field="confidence" />
        </button>
        <div>Details</div>
        <div className="text-right">Actions</div>
      </div>
    </>
  );
}
