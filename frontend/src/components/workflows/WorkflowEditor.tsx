import { useState, useCallback, useEffect, useMemo } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Plus,
  X,
  Trash2,
  Settings2,
  Zap,
  FileSearch,
  Brain,
  Filter,
  Merge,
  Save,
  ArrowRight,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  WorkflowNode,
  WorkflowConnection,
  WorkflowDefinition,
} from '@/lib/api';

// ── Node type registry (n8n-style) ──────────────────────────────────────────

export const NODE_TYPES = [
  {
    type: 'llm_extract',
    label: 'LLM Extract',
    description: 'Extract structured data using an LLM prompt',
    icon: FileSearch,
    color: 'bg-blue-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
    },
  },
  {
    type: 'llm_analyze',
    label: 'LLM Analyze',
    description: 'Analyze content and generate insights',
    icon: Brain,
    color: 'bg-purple-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
    },
  },
  {
    type: 'llm_summarize',
    label: 'LLM Summarize',
    description: 'Summarize and condense content',
    icon: Zap,
    color: 'bg-amber-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
    },
  },
  {
    type: 'transform',
    label: 'Transform',
    description: 'Transform data format or structure',
    icon: Settings2,
    color: 'bg-green-500',
    defaultParameters: {
      transform_expression: '',
    },
  },
  {
    type: 'filter',
    label: 'Filter',
    description: 'Filter results based on conditions',
    icon: Filter,
    color: 'bg-orange-500',
    defaultParameters: {
      condition: '',
    },
  },
  {
    type: 'merge',
    label: 'Merge',
    description: 'Merge results from multiple inputs',
    icon: Merge,
    color: 'bg-teal-500',
    defaultParameters: {
      merge_strategy: 'combine',
    },
  },
] as const;

type NodeTypeDef = (typeof NODE_TYPES)[number];

function getNodeTypeDef(type: string): NodeTypeDef | undefined {
  return NODE_TYPES.find((t) => t.type === type);
}

// ── Props ───────────────────────────────────────────────────────────────────

interface WorkflowEditorProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workflow?: WorkflowDefinition | null;
  onSave: (data: {
    id: string;
    name: string;
    description?: string;
    nodes: WorkflowNode[];
    connections: WorkflowConnection[];
  }) => void;
  isSaving?: boolean;
}

// ── Main editor component ───────────────────────────────────────────────────

export function WorkflowEditor({
  open,
  onOpenChange,
  workflow,
  onSave,
  isSaving,
}: WorkflowEditorProps) {
  const isNew = !workflow;

  // Workflow metadata
  const [id, setId] = useState('');
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');

  // Nodes and connections
  const [nodes, setNodes] = useState<WorkflowNode[]>([]);
  const [connections, setConnections] = useState<WorkflowConnection[]>([]);

  // UI state
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showNodePicker, setShowNodePicker] = useState(false);

  // Reset form when workflow changes
  useEffect(() => {
    if (open) {
      if (workflow) {
        setId(workflow.id);
        setName(workflow.name);
        setDescription(workflow.description || '');
        setNodes(structuredClone(workflow.nodes));
        setConnections(structuredClone(workflow.connections));
      } else {
        setId('');
        setName('');
        setDescription('');
        setNodes([]);
        setConnections([]);
      }
      setSelectedNodeId(null);
      setShowNodePicker(false);
    }
  }, [open, workflow]);

  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId]
  );

  // ── Node operations ─────────────────────────────────────────────────────

  const addNode = useCallback(
    (nodeType: string) => {
      const typeDef = getNodeTypeDef(nodeType);
      if (!typeDef) return;

      const newId = `${nodeType}_${Date.now().toString(36)}`;
      const maxY = nodes.reduce((max, n) => Math.max(max, n.position.y), 0);

      const newNode: WorkflowNode = {
        id: newId,
        name: typeDef.label,
        type: nodeType,
        parameters: { ...typeDef.defaultParameters },
        position: { x: 100, y: maxY + 150 },
      };

      setNodes((prev) => [...prev, newNode]);
      setSelectedNodeId(newId);
      setShowNodePicker(false);
    },
    [nodes]
  );

  const updateNode = useCallback(
    (nodeId: string, updates: Partial<WorkflowNode>) => {
      setNodes((prev) =>
        prev.map((n) => (n.id === nodeId ? { ...n, ...updates } : n))
      );
    },
    []
  );

  const updateNodeParameter = useCallback(
    (nodeId: string, key: string, value: any) => {
      setNodes((prev) =>
        prev.map((n) =>
          n.id === nodeId
            ? { ...n, parameters: { ...n.parameters, [key]: value } }
            : n
        )
      );
    },
    []
  );

  const deleteNode = useCallback(
    (nodeId: string) => {
      setNodes((prev) => prev.filter((n) => n.id !== nodeId));
      setConnections((prev) =>
        prev.filter((c) => c.source !== nodeId && c.target !== nodeId)
      );
      if (selectedNodeId === nodeId) setSelectedNodeId(null);
    },
    [selectedNodeId]
  );

  // ── Connection operations ───────────────────────────────────────────────

  const addConnection = useCallback(
    (source: string, target: string) => {
      if (source === target) return;
      const exists = connections.some(
        (c) => c.source === source && c.target === target
      );
      if (exists) return;
      setConnections((prev) => [
        ...prev,
        { source, target, source_output: 0, target_input: 0 },
      ]);
    },
    [connections]
  );

  const removeConnection = useCallback(
    (source: string, target: string) => {
      setConnections((prev) =>
        prev.filter((c) => !(c.source === source && c.target === target))
      );
    },
    []
  );

  // ── Get inputs for a node ───────────────────────────────────────────────

  const getNodeInputs = useCallback(
    (nodeId: string) =>
      connections
        .filter((c) => c.target === nodeId)
        .map((c) => nodes.find((n) => n.id === c.source))
        .filter(Boolean) as WorkflowNode[],
    [connections, nodes]
  );

  // ── Save ────────────────────────────────────────────────────────────────

  const handleSave = useCallback(() => {
    if (!id.trim() || !name.trim()) return;
    onSave({
      id: id.trim(),
      name: name.trim(),
      description: description.trim() || undefined,
      nodes,
      connections,
    });
  }, [id, name, description, nodes, connections, onSave]);

  const canSave = id.trim() && name.trim() && nodes.length > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[1100px] h-[85vh] p-0 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b">
          <div className="flex items-center gap-3">
            <DialogTitle className="text-lg">
              {isNew ? 'New Workflow' : `Edit: ${workflow?.name}`}
            </DialogTitle>
            {workflow?.is_system && (
              <Badge variant="secondary" className="text-xs">
                System
              </Badge>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleSave}
              disabled={!canSave || isSaving}
              className="gap-1.5"
            >
              <Save className="h-3.5 w-3.5" />
              {isSaving ? 'Saving...' : 'Save Workflow'}
            </Button>
          </div>
        </div>

        {/* Body: two-panel layout like n8n */}
        <div className="flex flex-1 min-h-0">
          {/* Left: Canvas / Node list */}
          <div className="flex-1 flex flex-col border-r">
            {/* Workflow metadata */}
            <div className="px-4 py-3 border-b bg-muted/30 space-y-2">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <Label className="text-xs text-muted-foreground">
                    Workflow ID
                  </Label>
                  <Input
                    value={id}
                    onChange={(e) =>
                      setId(
                        e.target.value
                          .toLowerCase()
                          .replace(/[^a-z0-9_]/g, '_')
                      )
                    }
                    placeholder="my_workflow"
                    className="h-8 text-sm mt-1"
                    disabled={!isNew}
                  />
                </div>
                <div>
                  <Label className="text-xs text-muted-foreground">Name</Label>
                  <Input
                    value={name}
                    onChange={(e) => setName(e.target.value)}
                    placeholder="My Workflow"
                    className="h-8 text-sm mt-1"
                  />
                </div>
              </div>
              <div>
                <Label className="text-xs text-muted-foreground">
                  Description
                </Label>
                <Input
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="What does this workflow do?"
                  className="h-8 text-sm mt-1"
                />
              </div>
            </div>

            {/* Node list (canvas representation) */}
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
                            {typeDef?.label ?? node.type}
                            {node.parameters.output_schema &&
                              ` → ${node.parameters.output_schema}`}
                          </div>
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          <Badge
                            variant="outline"
                            className="text-[10px] px-1.5"
                          >
                            {idx + 1}
                          </Badge>
                          <button
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteNode(node.id);
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
          </div>

          {/* Right: Node configuration panel (n8n style) */}
          <div className="w-[420px] flex flex-col bg-muted/20">
            {selectedNode ? (
              <NodeConfigPanel
                node={selectedNode}
                allNodes={nodes}
                connections={connections}
                onUpdate={(updates) => updateNode(selectedNode.id, updates)}
                onUpdateParameter={(key, value) =>
                  updateNodeParameter(selectedNode.id, key, value)
                }
                onAddConnection={(sourceId) =>
                  addConnection(sourceId, selectedNode.id)
                }
                onRemoveConnection={(sourceId) =>
                  removeConnection(sourceId, selectedNode.id)
                }
                onClose={() => setSelectedNodeId(null)}
              />
            ) : (
              <div className="flex-1 flex items-center justify-center text-muted-foreground p-8">
                <div className="text-center">
                  <Settings2 className="h-10 w-10 mx-auto mb-3 opacity-30" />
                  <p className="text-sm font-medium">Node Configuration</p>
                  <p className="text-xs mt-1">
                    Select a node to configure its settings
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// ── Node type picker ────────────────────────────────────────────────────────

function NodePicker({
  onSelect,
  onClose,
}: {
  onSelect: (type: string) => void;
  onClose: () => void;
}) {
  return (
    <div className="rounded-lg border bg-card shadow-lg p-2 space-y-1">
      <div className="flex items-center justify-between px-2 pb-1">
        <span className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
          Add Node
        </span>
        <button
          onClick={onClose}
          className="p-0.5 rounded hover:bg-muted transition-colors"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
      {NODE_TYPES.map((nt) => {
        const Icon = nt.icon;
        return (
          <button
            key={nt.type}
            onClick={() => onSelect(nt.type)}
            className={cn(
              'w-full flex items-center gap-3 rounded-md px-2 py-2',
              'hover:bg-accent transition-colors text-left'
            )}
          >
            <div
              className={cn(
                'w-8 h-8 rounded-md flex items-center justify-center text-white shrink-0',
                nt.color
              )}
            >
              <Icon className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <div className="text-sm font-medium">{nt.label}</div>
              <div className="text-xs text-muted-foreground">
                {nt.description}
              </div>
            </div>
          </button>
        );
      })}
    </div>
  );
}

// ── Node configuration panel ────────────────────────────────────────────────

function NodeConfigPanel({
  node,
  allNodes,
  connections,
  onUpdate,
  onUpdateParameter,
  onAddConnection,
  onRemoveConnection,
  onClose,
}: {
  node: WorkflowNode;
  allNodes: WorkflowNode[];
  connections: WorkflowConnection[];
  onUpdate: (updates: Partial<WorkflowNode>) => void;
  onUpdateParameter: (key: string, value: any) => void;
  onAddConnection: (sourceId: string) => void;
  onRemoveConnection: (sourceId: string) => void;
  onClose: () => void;
}) {
  const typeDef = getNodeTypeDef(node.type);
  const Icon = typeDef?.icon ?? Zap;

  // Current inputs (nodes connected TO this node)
  const currentInputs = connections
    .filter((c) => c.target === node.id)
    .map((c) => c.source);

  // Available nodes to connect from (exclude self and already-connected)
  const availableInputs = allNodes.filter(
    (n) => n.id !== node.id && !currentInputs.includes(n.id)
  );

  const isLLMNode = ['llm_extract', 'llm_analyze', 'llm_summarize'].includes(
    node.type
  );

  return (
    <div className="flex flex-col h-full">
      {/* Panel header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b">
        <div
          className={cn(
            'w-8 h-8 rounded-lg flex items-center justify-center text-white shrink-0',
            typeDef?.color ?? 'bg-gray-500'
          )}
        >
          <Icon className="h-4 w-4" />
        </div>
        <div className="flex-1 min-w-0">
          <div className="font-medium text-sm">{typeDef?.label}</div>
          <div className="text-xs text-muted-foreground">{node.id}</div>
        </div>
        <button
          onClick={onClose}
          className="p-1 rounded hover:bg-muted transition-colors"
        >
          <X className="h-4 w-4" />
        </button>
      </div>

      <ScrollArea className="flex-1">
        <div className="p-4 space-y-5">
          {/* Node name */}
          <div>
            <Label className="text-xs text-muted-foreground">Node Name</Label>
            <Input
              value={node.name}
              onChange={(e) => onUpdate({ name: e.target.value })}
              className="h-8 text-sm mt-1"
            />
          </div>

          {/* Node type */}
          <div>
            <Label className="text-xs text-muted-foreground">Node Type</Label>
            <Select
              value={node.type}
              onValueChange={(v) => onUpdate({ type: v })}
            >
              <SelectTrigger className="h-8 text-sm mt-1">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {NODE_TYPES.map((nt) => (
                  <SelectItem key={nt.type} value={nt.type}>
                    {nt.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          {/* Input connections */}
          <div>
            <Label className="text-xs text-muted-foreground mb-2 block">
              Input Connections
            </Label>
            <div className="space-y-1.5">
              {currentInputs.map((sourceId) => {
                const sourceNode = allNodes.find((n) => n.id === sourceId);
                const srcDef = sourceNode
                  ? getNodeTypeDef(sourceNode.type)
                  : undefined;
                const SrcIcon = srcDef?.icon ?? Zap;
                return (
                  <div
                    key={sourceId}
                    className="flex items-center gap-2 rounded-md border px-2 py-1.5 bg-card"
                  >
                    <div
                      className={cn(
                        'w-5 h-5 rounded flex items-center justify-center text-white',
                        srcDef?.color ?? 'bg-gray-500'
                      )}
                    >
                      <SrcIcon className="h-3 w-3" />
                    </div>
                    <span className="text-sm flex-1 truncate">
                      {sourceNode?.name ?? sourceId}
                    </span>
                    <button
                      onClick={() => onRemoveConnection(sourceId)}
                      className="p-0.5 rounded hover:bg-destructive/10 hover:text-destructive"
                    >
                      <X className="h-3 w-3" />
                    </button>
                  </div>
                );
              })}

              {availableInputs.length > 0 && (
                <Select onValueChange={(v) => onAddConnection(v)}>
                  <SelectTrigger className="h-8 text-sm">
                    <SelectValue placeholder="+ Connect input from..." />
                  </SelectTrigger>
                  <SelectContent>
                    {availableInputs.map((n) => (
                      <SelectItem key={n.id} value={n.id}>
                        {n.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}

              {currentInputs.length === 0 && availableInputs.length === 0 && (
                <p className="text-xs text-muted-foreground italic">
                  No inputs — this node receives raw data source content.
                </p>
              )}
            </div>
          </div>

          {/* Divider */}
          <div className="border-t" />

          {/* Type-specific parameters */}
          <div>
            <Label className="text-xs text-muted-foreground mb-2 block">
              Parameters
            </Label>

            {isLLMNode && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Prompt Template</Label>
                  <Textarea
                    value={node.parameters.prompt_template ?? ''}
                    onChange={(e) =>
                      onUpdateParameter('prompt_template', e.target.value)
                    }
                    placeholder={`Analyze the following content and...\n\nContent:\n{{content}}\n\nPrevious results:\n{{previous_results}}`}
                    className="mt-1 text-sm font-mono min-h-[200px] resize-y"
                  />
                  <p className="text-[10px] text-muted-foreground mt-1">
                    Use {'{{content}}'} for data source text and{' '}
                    {'{{previous_results}}'} for upstream node outputs.
                  </p>
                </div>
                <div>
                  <Label className="text-xs">Output Schema</Label>
                  <Input
                    value={node.parameters.output_schema ?? ''}
                    onChange={(e) =>
                      onUpdateParameter('output_schema', e.target.value)
                    }
                    placeholder="e.g. companies[], contacts[], opportunities[]"
                    className="h-8 text-sm mt-1"
                  />
                </div>
              </div>
            )}

            {node.type === 'transform' && (
              <div>
                <Label className="text-xs">Transform Expression</Label>
                <Textarea
                  value={node.parameters.transform_expression ?? ''}
                  onChange={(e) =>
                    onUpdateParameter('transform_expression', e.target.value)
                  }
                  placeholder="Describe the transformation to apply..."
                  className="mt-1 text-sm font-mono min-h-[120px]"
                />
              </div>
            )}

            {node.type === 'filter' && (
              <div>
                <Label className="text-xs">Filter Condition</Label>
                <Textarea
                  value={node.parameters.condition ?? ''}
                  onChange={(e) =>
                    onUpdateParameter('condition', e.target.value)
                  }
                  placeholder="Describe the filter condition..."
                  className="mt-1 text-sm font-mono min-h-[120px]"
                />
              </div>
            )}

            {node.type === 'merge' && (
              <div>
                <Label className="text-xs">Merge Strategy</Label>
                <Select
                  value={node.parameters.merge_strategy ?? 'combine'}
                  onValueChange={(v) =>
                    onUpdateParameter('merge_strategy', v)
                  }
                >
                  <SelectTrigger className="h-8 text-sm mt-1">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="combine">
                      Combine (merge all inputs)
                    </SelectItem>
                    <SelectItem value="append">
                      Append (concatenate)
                    </SelectItem>
                    <SelectItem value="deduplicate">
                      Deduplicate
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>
            )}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
