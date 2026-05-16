import { RefreshCw } from 'lucide-react';

import { Button } from '@/components/ui/button';

import type { TopologyScope } from '../useTopologyData';

export interface ScopeBarProps {
  scope: TopologyScope;
  onSyncClick?: () => void;
  isSyncing?: boolean;
  canSync?: boolean;
  stats?: { nodes: number; edges: number };
}

/**
 * Thin header strip above the canvas — shows the current scope and a
 * sync button (admin-only). Mode toggle lives in a separate component.
 */
export function ScopeBar({
  scope,
  onSyncClick,
  isSyncing = false,
  canSync = false,
  stats,
}: ScopeBarProps) {
  return (
    <div
      className="flex items-center justify-between gap-2 border-b border-slate-800 bg-slate-900/70 px-3 py-1.5 text-xs text-slate-300"
      data-testid="topology-scope-bar"
    >
      <div className="flex items-center gap-2">
        <span className="uppercase tracking-wide text-slate-500">Scope:</span>
        <span className="font-medium text-slate-200">{formatScope(scope)}</span>
        {stats && (
          <span className="ml-2 text-slate-500">
            {stats.nodes} nodes · {stats.edges} edges
          </span>
        )}
      </div>
      {canSync && onSyncClick && (
        <Button
          variant="outline"
          size="sm"
          onClick={onSyncClick}
          disabled={isSyncing}
          className="h-7 gap-1 text-xs"
          data-testid="topology-sync-button"
        >
          <RefreshCw
            className={isSyncing ? 'h-3 w-3 animate-spin' : 'h-3 w-3'}
          />
          {isSyncing ? 'Syncing…' : 'Sync'}
        </Button>
      )}
    </div>
  );
}

function formatScope(scope: TopologyScope): string {
  switch (scope.kind) {
    case 'global':
      return 'All orgs';
    case 'org':
      return `Organization ${scope.orgId.slice(0, 8)}`;
    case 'focused':
      return `${scope.focusType}:${scope.focusId.slice(0, 8)} (depth ${scope.depth ?? 2})`;
  }
}
