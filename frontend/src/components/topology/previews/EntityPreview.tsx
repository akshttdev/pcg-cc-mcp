import { ExternalLink } from 'lucide-react';
import { useMemo } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import type { VizNode } from '@/lib/graph/adapter';

export interface EntityPreviewProps {
  node: VizNode;
  onClose?: () => void;
  onOpenProfile?: (node: VizNode) => void;
}

/**
 * Fallback preview for any node type that doesn't have a bespoke component
 * yet. Shows label, type chip, metadata JSON, and an "Open profile" link
 * when we know where the detail page lives.
 */
export function EntityPreview({
  node,
  onClose,
  onOpenProfile,
}: EntityPreviewProps) {
  const parsedMeta = useMemo(() => {
    if (!node.metadata) return null;
    try {
      return JSON.parse(node.metadata) as Record<string, unknown>;
    } catch {
      return null;
    }
  }, [node.metadata]);

  return (
    <div
      className="flex w-80 flex-col gap-3 rounded-lg border border-slate-700 bg-slate-900/95 p-4 shadow-xl backdrop-blur"
      data-testid={`topology-preview-${node.node_type}`}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex flex-col gap-1">
          <div className="text-sm font-semibold text-slate-100">
            {node.label}
          </div>
          <Badge variant="outline" className="w-fit text-xs">
            {node.node_type}
          </Badge>
        </div>
        {onClose && (
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            data-testid="topology-preview-close"
          >
            ×
          </Button>
        )}
      </div>

      {parsedMeta && Object.keys(parsedMeta).length > 0 && (
        <dl className="grid grid-cols-2 gap-1 text-xs">
          {Object.entries(parsedMeta).map(([k, v]) => (
            <div key={k} className="contents">
              <dt className="truncate text-slate-400">{k}</dt>
              <dd className="truncate text-slate-200">{String(v)}</dd>
            </div>
          ))}
        </dl>
      )}

      <div className="flex items-center gap-2 text-xs text-slate-500">
        <span className="truncate">{node.ref_table}</span>
        <span>·</span>
        <span className="truncate">{node.ref_id.slice(0, 8)}</span>
      </div>

      {onOpenProfile && (
        <Button
          variant="secondary"
          size="sm"
          onClick={() => onOpenProfile(node)}
          data-testid="topology-preview-open-profile"
          className="gap-1"
        >
          Open profile
          <ExternalLink className="h-3 w-3" />
        </Button>
      )}
    </div>
  );
}
