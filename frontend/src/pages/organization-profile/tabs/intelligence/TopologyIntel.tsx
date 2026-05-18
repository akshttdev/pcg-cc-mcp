import { Archive, Maximize2, Minimize2, Network } from 'lucide-react';
import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';

import { type TopologyScope, TopologyShell } from '@/components/topology';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

import { TopologyIntelView } from './TopologyView';

export interface TopologyIntelProps {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  /** Whether the signed-in user is a platform admin — controls the Sync
   *  button visibility on the shell. Defaults false until Phase 6 wires
   *  live identity. */
  canSync?: boolean;
}

/**
 * Intelligence › Topology tab.
 *
 * Primary: the live entity-graph canvas (TopologyShell) scoped to this org.
 * Archive: the legacy `topology_snapshot` knowledge-source grid, preserved
 * because it's sourced from real research artifacts and still useful.
 *
 * Supports deep-link query params:
 *   ?focus_type=company&focus_id=<uuid>&depth=2  — opens focused BFS
 *   ?mode=2d|3d                                   — initial camera mode
 *   ?archive=1                                    — land on the archive tab
 */
export function TopologyIntel({
  orgId,
  projectEntries,
  canSync = false,
}: TopologyIntelProps) {
  const [searchParams, setSearchParams] = useSearchParams();

  const focusType = searchParams.get('focus_type') ?? undefined;
  const focusId = searchParams.get('focus_id') ?? undefined;
  const depthParam = Number(searchParams.get('depth') ?? '2');
  const archiveParam = searchParams.get('archive') === '1';

  const scope = useMemo<TopologyScope>(() => {
    if (focusType && focusId) {
      return {
        kind: 'focused',
        focusType,
        focusId,
        depth: Number.isFinite(depthParam) ? depthParam : 2,
      };
    }
    return { kind: 'org', orgId };
  }, [focusType, focusId, depthParam, orgId]);

  const [tab, setTab] = useState<'graph' | 'archive'>(
    archiveParam ? 'archive' : 'graph'
  );
  const [fullscreen, setFullscreen] = useState(false);

  const switchTab = (next: 'graph' | 'archive') => {
    setTab(next);
    const params = new URLSearchParams(searchParams);
    if (next === 'archive') params.set('archive', '1');
    else params.delete('archive');
    setSearchParams(params, { replace: true });
  };

  return (
    <div
      className={cn(
        'flex flex-col gap-3',
        fullscreen && 'fixed inset-0 z-50 bg-slate-950 p-3'
      )}
      data-testid="topology-intel"
    >
      <div className="flex items-center gap-2">
        <div className="flex items-center gap-1 rounded-md border border-slate-800 bg-slate-900/70 p-0.5">
          <SubTab
            active={tab === 'graph'}
            onClick={() => switchTab('graph')}
            icon={<Network className="h-3 w-3" />}
            label="Graph"
            testId="topology-intel-tab-graph"
          />
          <SubTab
            active={tab === 'archive'}
            onClick={() => switchTab('archive')}
            icon={<Archive className="h-3 w-3" />}
            label="Archive"
            testId="topology-intel-tab-archive"
          />
        </div>
        {tab === 'graph' && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => setFullscreen((f) => !f)}
            data-testid="topology-intel-fullscreen"
            className="gap-1.5"
          >
            {fullscreen ? (
              <>
                <Minimize2 className="h-3.5 w-3.5" />
                Exit fullscreen
              </>
            ) : (
              <>
                <Maximize2 className="h-3.5 w-3.5" />
                Fullscreen
              </>
            )}
          </Button>
        )}
      </div>

      {tab === 'graph' ? (
        <div
          className={cn(
            'overflow-hidden rounded-lg border border-slate-800',
            fullscreen
              ? 'flex-1 min-h-0'
              : 'h-[calc(100vh-13rem)] min-h-[560px]'
          )}
        >
          <TopologyShell scope={scope} orgId={orgId} canSync={canSync} />
        </div>
      ) : (
        <TopologyIntelView projectEntries={projectEntries} />
      )}
    </div>
  );
}

function SubTab({
  active,
  onClick,
  icon,
  label,
  testId,
}: {
  active: boolean;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
  testId: string;
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={onClick}
      data-testid={testId}
      className={cn(
        'h-7 gap-1 rounded-sm px-2 text-xs',
        active && 'bg-slate-700 text-slate-50'
      )}
    >
      {icon}
      {label}
    </Button>
  );
}
