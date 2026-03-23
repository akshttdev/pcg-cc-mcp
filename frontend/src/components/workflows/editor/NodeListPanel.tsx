import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Plus,
  Trash2,
  Zap,
  ArrowRight,
  AlertTriangle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { WorkflowNode } from '@/lib/api';
import { getNodeTypeDef } from './node-types';
import { NodePicker } from './NodePicker';

interface NodeListPanelProps {
  nodes: WorkflowNode[];
  selectedNodeId: string | null;
  setSelectedNodeId: (id: string | null) => void;
  showNodePicker: boolean;
  setShowNodePicker: (show: boolean) => void;
  addNode: (nodeType: string) => void;
  getNodeInputs: (nodeId: string) => WorkflowNode[];
  previewWarningNodes: Map<string, string>;
  setPendingDeleteNodeId: (id: string | null) => void;
}

export function NodeListPanel({
  nodes,
  selectedNodeId,
  setSelectedNodeId,
  showNodePicker,
  setShowNodePicker,
  addNode,
  getNodeInputs,
  previewWarningNodes,
  setPendingDeleteNodeId,
}: NodeListPanelProps) {
  return (
    <>
      <ScrollArea className="flex-1">
        <div className="p-4 space-y-2">
          {nodes.length === 0 && (
            <div className="text-center py-16 text-muted-foreground">
              <Zap className="h-10 w-10 mx-auto mb-3 opacity-30" />
              <p className="text-sm font-medium">No nodes yet</p>
              <p className="text-xs mt-1">
                Add your first node to start building the workflow
              </p>
              <Button
                size="sm"
                variant="outline"
                className="mt-4 gap-1.5"
                onClick={() => setShowNodePicker(true)}
              >
                <Plus className="h-3.5 w-3.5" />
                Add Node
              </Button>
            </div>
          )}

          {nodes.map((node, idx) => {
            const typeDef = getNodeTypeDef(node.type);
            const Icon = typeDef?.icon ?? Zap;
            const inputs = getNodeInputs(node.id);
            const isSelected = selectedNodeId === node.id;

            return (
              <div key={node.id} className="relative">
                {/* Connection lines from inputs */}
                {inputs.length > 0 && (
                  <div className="flex items-center gap-1 mb-1 ml-6 text-xs text-muted-foreground">
                    <ArrowRight className="h-3 w-3" />
                    from:{' '}
                    {inputs.map((inp) => inp.name).join(', ')}
                  </div>
                )}

                {/* Node card (n8n style) */}
                <button
                  onClick={() =>
                    setSelectedNodeId(isSelected ? null : node.id)
                  }
                  className={cn(
                    'w-full text-left rounded-lg border-2 transition-all',
                    'hover:shadow-md cursor-pointer',
                    'flex items-center gap-3 p-3',
                    isSelected
                      ? 'border-primary shadow-md bg-accent/50'
                      : 'border-border bg-card hover:border-primary/40'
                  )}
                >
                  <div
                    className={cn(
                      'w-9 h-9 rounded-lg flex items-center justify-center text-white shrink-0',
                      typeDef?.color ?? 'bg-gray-500'
                    )}
                  >
                    <Icon className="h-4.5 w-4.5" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm truncate">
                      {node.name}
                    </div>
                    <div className="text-xs text-muted-foreground truncate">
                      {node.type.startsWith('output_')
                        ? `→ ${typeDef?.description ?? node.type}`
                        : (typeDef?.label ?? node.type)}
                      {node.parameters.output_schema &&
                        !node.type.startsWith('output_') &&
                        ` → ${node.parameters.output_schema}`}
                      {node.parameters.model && (
                        <Badge variant="outline" className="ml-1.5 text-[9px] px-1">
                          {node.parameters.model.split('/').pop()?.replace(/-/g, ' ') || node.parameters.model}
                        </Badge>
                      )}
                    </div>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    {previewWarningNodes.has(node.id) && (
                      <Badge
                        variant="outline"
                        className="text-xs px-1.5 text-amber-600 border-amber-200 gap-0.5"
                        title={previewWarningNodes.get(node.id)}
                      >
                        <AlertTriangle className="h-2.5 w-2.5" />
                      </Badge>
                    )}
                    <Badge
                      variant="outline"
                      className="text-xs px-1.5"
                    >
                      {idx + 1}
                    </Badge>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        setPendingDeleteNodeId(node.id);
                      }}
                      className="p-1 rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </button>
              </div>
            );
          })}

          {/* Add node button */}
          {nodes.length > 0 && (
            <div className="pt-2">
              {showNodePicker ? (
                <NodePicker
                  onSelect={addNode}
                  onClose={() => setShowNodePicker(false)}
                />
              ) : (
                <button
                  onClick={() => setShowNodePicker(true)}
                  className={cn(
                    'w-full rounded-lg border-2 border-dashed border-border',
                    'hover:border-primary/40 hover:bg-accent/30',
                    'transition-all p-3 flex items-center justify-center gap-2',
                    'text-sm text-muted-foreground hover:text-foreground'
                  )}
                >
                  <Plus className="h-4 w-4" />
                  Add Node
                </button>
              )}
            </div>
          )}
        </div>
      </ScrollArea>

      {showNodePicker && nodes.length === 0 && (
        <div className="p-4 border-t">
          <NodePicker
            onSelect={addNode}
            onClose={() => setShowNodePicker(false)}
          />
        </div>
      )}
    </>
  );
}
