import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Eye, X, Loader2, AlertTriangle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type { PreviewNodeResult } from '@/lib/api';
import { getNodeTypeDef } from './node-types';

interface PreviewPanelProps {
  showPreview: boolean;
  setShowPreview: (show: boolean) => void;
  isPreviewing: boolean;
  previewResults: PreviewNodeResult[] | null;
  previewWarningNodes: Map<string, string>;
}

export function PreviewPanel({
  showPreview,
  setShowPreview,
  isPreviewing,
  previewResults,
  previewWarningNodes,
}: PreviewPanelProps) {
  if (!showPreview) return null;

  return (
    <div className="border-t bg-card max-h-[40%] flex flex-col">
      <div className="flex items-center justify-between px-4 py-2 border-b bg-muted/30">
        <div className="flex items-center gap-2">
          <Eye className="h-4 w-4 text-muted-foreground" />
          <span className="text-sm font-medium">Preview Results</span>
          {isPreviewing && <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />}
        </div>
        <button onClick={() => setShowPreview(false)} className="p-1 rounded hover:bg-muted">
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
      <ScrollArea className="flex-1 p-4">
        {isPreviewing && !previewResults && (
          <div className="text-center py-8 text-muted-foreground">
            <Loader2 className="h-6 w-6 mx-auto mb-2 animate-spin" />
            <p className="text-sm">Running workflow preview...</p>
          </div>
        )}
        {previewResults && (
          <div className="space-y-3">
            {previewResults.map((r) => {
              const typeDef = getNodeTypeDef(r.node_type);
              return (
                <div key={r.node_id} className={cn(
                  'rounded-lg border bg-card p-3',
                  previewWarningNodes.has(r.node_id) && 'border-amber-300 bg-amber-50/30 dark:bg-amber-950/10'
                )}>
                  <div className="flex items-center gap-2 mb-2">
                    <div className={cn('w-5 h-5 rounded flex items-center justify-center text-white text-xs', typeDef?.color ?? 'bg-gray-500')}>
                      {r.node_name.charAt(0)}
                    </div>
                    <span className="text-sm font-medium">{r.node_name}</span>
                    <Badge variant="outline" className="text-xs">{r.node_type}</Badge>
                    {previewWarningNodes.has(r.node_id) && (
                      <Badge variant="outline" className="text-xs text-amber-600 border-amber-200 gap-0.5">
                        <AlertTriangle className="h-2.5 w-2.5" />
                        {previewWarningNodes.get(r.node_id)}
                      </Badge>
                    )}
                  </div>
                  <pre className="text-xs bg-muted/50 rounded p-2 overflow-auto max-h-[200px] whitespace-pre-wrap font-mono">
                    {(() => {
                      try { return JSON.stringify(JSON.parse(r.output), null, 2); }
                      catch { return r.output; }
                    })()}
                  </pre>
                  {r.usage && (
                    <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
                      {r.usage.model_used && <span>Model: {r.usage.model_used}</span>}
                      {r.usage.provider && <span>Provider: {r.usage.provider}</span>}
                      {r.usage.input_tokens != null && <span>In: {r.usage.input_tokens.toLocaleString()}</span>}
                      {r.usage.output_tokens != null && <span>Out: {r.usage.output_tokens.toLocaleString()}</span>}
                      {r.usage.estimated_cost_micros != null && (
                        <span>Cost: ${(r.usage.estimated_cost_micros / 1_000_000).toFixed(4)}</span>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </ScrollArea>
    </div>
  );
}
