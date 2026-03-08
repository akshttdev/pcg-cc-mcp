import { useState, useCallback, useEffect, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
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
  Eye,
  Loader2,
  Network,
  Cpu,
  Users,
  Building2,
  Handshake,
  ListTodo,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { workflowsApi, type AvailableModel } from '@/lib/api';
import type {
  WorkflowNode,
  WorkflowConnection,
  WorkflowDefinition,
  PreviewNodeResult,
  AvailableModel,
} from '@/lib/api';

// ── Node type registry (n8n-style) ──────────────────────────────────────────

interface NodeTypeDefinition {
  type: string;
  label: string;
  description: string;
  icon: any;
  color: string;
  defaultParameters: Record<string, any>;
}

export const NODE_TYPES: NodeTypeDefinition[] = [
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
  {
    type: 'output_crm_contacts',
    label: 'Output: CRM Contacts',
    description: 'Send extracted contacts to CRM',
    icon: Users,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'crm_contact',
      on_duplicate: 'flag_for_review',
    },
  },
  {
    type: 'output_crm_companies',
    label: 'Output: Companies',
    description: 'Send extracted companies to company records',
    icon: Building2,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'company',
      on_duplicate: 'flag_for_review',
    },
  },
  {
    type: 'output_crm_deals',
    label: 'Output: CRM Deals',
    description: 'Create deals/opportunities in CRM pipeline',
    icon: Handshake,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'crm_deal',
      on_duplicate: 'flag_for_review',
    },
  },
  {
    type: 'output_tasks',
    label: 'Output: Tasks',
    description: 'Create tasks from workflow results',
    icon: ListTodo,
    color: 'bg-emerald-600',
    defaultParameters: {
      target_type: 'task',
      on_duplicate: 'skip',
    },
  },
];

function getNodeTypeDef(type: string): NodeTypeDefinition | undefined {
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
    default_model?: string;
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

  const [defaultModel, setDefaultModel] = useState('');

  // Nodes and connections
  const [nodes, setNodes] = useState<WorkflowNode[]>([]);
  const [connections, setConnections] = useState<WorkflowConnection[]>([]);

  // Fetch available models
  const { data: availableModels = [] } = useQuery({
    queryKey: ['workflowModels'],
    queryFn: () => workflowsApi.listAvailableModels(),
    staleTime: 60 * 60 * 1000,
    enabled: open,
  });

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
        setDefaultModel(workflow?.default_model || '');
        setNodes(structuredClone(workflow.nodes));
        setConnections(structuredClone(workflow.connections));
      } else {
        setId('');
        setName('');
        setDescription('');
        setDefaultModel('');
        setNodes([]);
        setConnections([]);
      }
      setSelectedNodeId(null);
      setShowNodePicker(false);
      setPreviewResults(null);
      setShowPreview(false);
      setShowGraph(false);
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
      default_model: defaultModel || undefined,
    });
  }, [id, name, description, nodes, connections, defaultModel, onSave]);

  const canSave = id.trim() && name.trim() && nodes.length > 0;

  // ── Preview state ────────────────────────────────────────────────────
  const [previewResults, setPreviewResults] = useState<PreviewNodeResult[] | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [showGraph, setShowGraph] = useState(false);

  const handlePreview = useCallback(async () => {
    if (nodes.length === 0) return;
    setIsPreviewing(true);
    setShowPreview(true);
    try {
      const results = await workflowsApi.previewWorkflow({ nodes, connections });
      setPreviewResults(results);
    } catch (e) {
      setPreviewResults([{ node_id: 'error', node_name: 'Error', node_type: 'error', output: String(e) }]);
    } finally {
      setIsPreviewing(false);
    }
  }, [nodes, connections]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-[90vw] w-[1400px] h-[90vh] p-0 flex flex-col overflow-hidden">
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
              variant="ghost"
              onClick={() => setShowGraph(true)}
              disabled={nodes.length === 0}
              className="gap-1.5"
            >
              <Network className="h-3.5 w-3.5" />
              Graph
            </Button>
            <Button
              size="sm"
              variant="ghost"
              onClick={handlePreview}
              disabled={nodes.length === 0 || isPreviewing}
              className="gap-1.5"
            >
              <Eye className="h-3.5 w-3.5" />
              {isPreviewing ? 'Running...' : 'Preview'}
            </Button>
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
              <div>
                <Label className="text-xs text-muted-foreground">Default Model</Label>
                <Select value={defaultModel} onValueChange={setDefaultModel}>
                  <SelectTrigger className="h-8 text-sm mt-1">
                    <SelectValue placeholder="Auto (highest priority)" />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="">Auto (highest priority)</SelectItem>
                    {availableModels.map((m: AvailableModel) => (
                      <SelectItem key={m.id} value={m.id}>
                        <div className="flex items-center gap-2">
                          <span>{m.label}</span>
                          <span className="text-muted-foreground text-xs">
                            ${(m.cost_per_million_input / 100).toFixed(2)}/M in
                          </span>
                        </div>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
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
                            {node.parameters.model && (
                              <Badge variant="outline" className="ml-1.5 text-[9px] px-1">
                                {node.parameters.model.split('/').pop()?.replace(/-/g, ' ') || node.parameters.model}
                              </Badge>
                            )}
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
          <div className="w-[480px] flex flex-col bg-muted/20">
            {selectedNode ? (
              <NodeConfigPanel
                node={selectedNode}
                allNodes={nodes}
                connections={connections}
                availableModels={availableModels}
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

        {/* Preview results panel (slides up from bottom) */}
        {showPreview && (
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
                      <div key={r.node_id} className="rounded-lg border bg-card p-3">
                        <div className="flex items-center gap-2 mb-2">
                          <div className={cn('w-5 h-5 rounded flex items-center justify-center text-white text-[10px]', typeDef?.color ?? 'bg-gray-500')}>
                            {r.node_name.charAt(0)}
                          </div>
                          <span className="text-sm font-medium">{r.node_name}</span>
                          <Badge variant="outline" className="text-[10px]">{r.node_type}</Badge>
                        </div>
                        <pre className="text-xs bg-muted/50 rounded p-2 overflow-auto max-h-[200px] whitespace-pre-wrap font-mono">
                          {(() => {
                            try { return JSON.stringify(JSON.parse(r.output), null, 2); }
                            catch { return r.output; }
                          })()}
                        </pre>
                        {r.usage && (
                          <div className="flex items-center gap-3 mt-2 text-[10px] text-muted-foreground">
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
        )}

      </DialogContent>

      {/* Graph preview modal */}
      <Dialog open={showGraph} onOpenChange={setShowGraph}>
        <DialogContent className="max-w-[85vw] w-[1000px] max-h-[85vh] p-0 flex flex-col overflow-hidden">
          <div className="flex items-center justify-between px-6 py-4 border-b">
            <DialogTitle className="text-lg flex items-center gap-2">
              <Network className="h-5 w-5" />
              Workflow Graph
            </DialogTitle>
          </div>
          <div className="flex-1 overflow-auto p-6">
            <WorkflowGraphView nodes={nodes} connections={connections} />
          </div>
        </DialogContent>
      </Dialog>
    </Dialog>
  );
}

// ── Static graph visualization ────────────────────────────────────────────────

function WorkflowGraphView({ nodes, connections }: { nodes: WorkflowNode[]; connections: WorkflowConnection[] }) {
  // Build layout: topological layers
  const layers = useMemo(() => {
    const deps: Record<string, string[]> = {};
    for (const c of connections) {
      if (!deps[c.target]) deps[c.target] = [];
      deps[c.target].push(c.source);
    }

    const nodeDepth: Record<string, number> = {};
    const getDepth = (id: string, visited: Set<string> = new Set()): number => {
      if (nodeDepth[id] !== undefined) return nodeDepth[id];
      if (visited.has(id)) return 0;
      visited.add(id);
      const nodeDeps = deps[id] ?? [];
      const depth = nodeDeps.length === 0 ? 0 : Math.max(...nodeDeps.map(d => getDepth(d, visited))) + 1;
      nodeDepth[id] = depth;
      return depth;
    };
    nodes.forEach(n => getDepth(n.id));

    const maxDepth = Math.max(0, ...Object.values(nodeDepth));
    const result: WorkflowNode[][] = [];
    for (let d = 0; d <= maxDepth; d++) {
      result.push(nodes.filter(n => (nodeDepth[n.id] ?? 0) === d));
    }
    return result;
  }, [nodes, connections]);

  if (nodes.length === 0) {
    return (
      <div className="text-center py-12 text-muted-foreground">
        <Network className="h-8 w-8 mx-auto mb-2 opacity-30" />
        <p className="text-sm">No nodes to visualize</p>
      </div>
    );
  }

  const nodeWidth = 160;
  const nodeHeight = 56;
  const layerGapX = 220;
  const nodeGapY = 80;
  const padX = 40;
  const padY = 40;

  const totalWidth = padX * 2 + layers.length * layerGapX;
  const maxNodesInLayer = Math.max(1, ...layers.map(l => l.length));
  const totalHeight = padY * 2 + maxNodesInLayer * nodeGapY;

  // Compute positions
  const nodePositions: Record<string, { x: number; y: number }> = {};
  layers.forEach((layer, li) => {
    const layerHeight = layer.length * nodeGapY;
    const startY = (totalHeight - layerHeight) / 2;
    layer.forEach((node, ni) => {
      nodePositions[node.id] = {
        x: padX + li * layerGapX,
        y: startY + ni * nodeGapY,
      };
    });
  });

  return (
    <div className="overflow-auto">
      <svg width={totalWidth} height={totalHeight} className="mx-auto">
        <defs>
          <marker id="arrowhead" markerWidth="8" markerHeight="6" refX="8" refY="3" orient="auto">
            <path d="M0,0 L8,3 L0,6" fill="currentColor" className="text-muted-foreground" />
          </marker>
        </defs>

        {/* Connection lines */}
        {connections.map((conn, i) => {
          const from = nodePositions[conn.source];
          const to = nodePositions[conn.target];
          if (!from || !to) return null;
          const x1 = from.x + nodeWidth;
          const y1 = from.y + nodeHeight / 2;
          const x2 = to.x;
          const y2 = to.y + nodeHeight / 2;
          const midX = (x1 + x2) / 2;
          return (
            <path
              key={i}
              d={`M${x1},${y1} C${midX},${y1} ${midX},${y2} ${x2},${y2}`}
              stroke="currentColor"
              className="text-muted-foreground/50"
              strokeWidth="2"
              fill="none"
              markerEnd="url(#arrowhead)"
            />
          );
        })}

        {/* Node boxes */}
        {nodes.map((node) => {
          const pos = nodePositions[node.id];
          if (!pos) return null;
          const typeDef = getNodeTypeDef(node.type);
          const colorMap: Record<string, string> = {
            'bg-blue-500': '#3b82f6', 'bg-purple-500': '#a855f7',
            'bg-emerald-500': '#10b981', 'bg-amber-500': '#f59e0b',
            'bg-orange-500': '#f97316', 'bg-teal-500': '#14b8a6',
            'bg-green-500': '#22c55e', 'bg-emerald-600': '#059669',
          };
          const fill = colorMap[typeDef?.color ?? ''] ?? '#6b7280';
          return (
            <g key={node.id}>
              <rect
                x={pos.x} y={pos.y}
                width={nodeWidth} height={nodeHeight}
                rx="8" ry="8"
                fill="hsl(var(--card))"
                stroke={fill}
                strokeWidth="2"
              />
              <rect
                x={pos.x} y={pos.y}
                width="6" height={nodeHeight}
                rx="8" ry="0"
                fill={fill}
              />
              <text
                x={pos.x + 16} y={pos.y + 22}
                fontSize="12" fontWeight="600"
                fill="currentColor" className="text-foreground"
              >
                {node.name.length > 16 ? node.name.slice(0, 15) + '...' : node.name}
              </text>
              <text
                x={pos.x + 16} y={pos.y + 40}
                fontSize="10"
                fill="currentColor" className="text-muted-foreground"
              >
                {typeDef?.label ?? node.type}
              </text>
            </g>
          );
        })}
      </svg>
    </div>
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
  const processingNodes = NODE_TYPES.filter(nt => !nt.type.startsWith('output_'));
  const outputNodes = NODE_TYPES.filter(nt => nt.type.startsWith('output_'));

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
      {processingNodes.map((nt) => {
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
      {outputNodes.length > 0 && (
        <>
          <div className="border-t my-1" />
          <div className="px-2 py-0.5">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Outputs</span>
          </div>
          {outputNodes.map((nt) => {
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
        </>
      )}
    </div>
  );
}

// ── Node configuration panel ────────────────────────────────────────────────

function NodeConfigPanel({
  node,
  allNodes,
  connections,
  availableModels,
  onUpdate,
  onUpdateParameter,
  onAddConnection,
  onRemoveConnection,
  onClose,
}: {
  node: WorkflowNode;
  allNodes: WorkflowNode[];
  connections: WorkflowConnection[];
  availableModels: AvailableModel[];
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
                {/* Model selection */}
                <div>
                  <Label className="text-xs">Model</Label>
                  <Select
                    value={node.parameters.model || ''}
                    onValueChange={(v) => onUpdateParameter('model', v || undefined)}
                  >
                    <SelectTrigger className="h-8 text-sm mt-1">
                      <SelectValue placeholder="Use workflow default" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="">Use workflow default</SelectItem>
                      {availableModels.map((m) => (
                        <SelectItem key={m.id} value={m.id}>
                          {m.label}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <p className="text-[10px] text-muted-foreground mt-1">
                    Override the workflow's default model for this node.
                  </p>
                </div>
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

            {node.type.startsWith('output_') && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Target</Label>
                  <div className="mt-1 text-sm text-muted-foreground bg-muted/50 rounded px-2 py-1.5">
                    {node.type === 'output_crm_contacts' && 'CRM Contacts'}
                    {node.type === 'output_crm_companies' && 'Companies'}
                    {node.type === 'output_crm_deals' && 'CRM Deals'}
                    {node.type === 'output_tasks' && 'Tasks'}
                  </div>
                </div>
                <div>
                  <Label className="text-xs">On Duplicate</Label>
                  <Select
                    value={node.parameters.on_duplicate || 'flag_for_review'}
                    onValueChange={(v) => onUpdateParameter('on_duplicate', v)}
                  >
                    <SelectTrigger className="h-8 text-sm mt-1">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="flag_for_review">Flag for review</SelectItem>
                      <SelectItem value="skip">Skip duplicates</SelectItem>
                      <SelectItem value="update_existing">Update existing</SelectItem>
                      <SelectItem value="create_anyway">Create anyway</SelectItem>
                    </SelectContent>
                  </Select>
                  <p className="text-[10px] text-muted-foreground mt-1">
                    What to do when a matching record already exists.
                  </p>
                </div>
                <div className="rounded-md border border-dashed border-muted-foreground/30 p-2.5 bg-muted/20">
                  <p className="text-[10px] text-muted-foreground">
                    Connect an LLM node as input. The schema for the target type will be automatically injected into the upstream LLM prompt so it produces correctly formatted output.
                  </p>
                </div>
              </div>
            )}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
