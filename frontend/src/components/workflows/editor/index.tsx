import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { workflowKeys } from '@/lib/query-keys';
import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
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
  Save,
  ArrowRight,
  Eye,
  Loader2,
  Network,
  Bell,
  ChevronDown,
  ChevronRight,
  AlertTriangle,
  List,
  Lock,
  LockOpen,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { WorkflowTriggersPanel } from '../WorkflowTriggersPanel';
import { workflowsApi } from '@/lib/api';
import type {
  WorkflowNode,
  WorkflowConnection,
  WorkflowDefinition,
  PreviewNodeResult,
  AvailableModel,
} from '@/lib/api';

import { getNodeTypeDef } from './node-types';
import { WorkflowGraphView } from './WorkflowGraphView';
import { NodePicker } from './NodePicker';
import { NodeConfigPanel } from './NodeConfigPanel';

// Re-export for external consumers
export { NODE_TYPES, getNodeTypeDef } from './node-types';

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
  const [idLocked, setIdLocked] = useState(true);

  const [defaultModel, setDefaultModel] = useState('');

  // Nodes and connections
  const [nodes, setNodes] = useState<WorkflowNode[]>([]);
  const [connections, setConnections] = useState<WorkflowConnection[]>([]);

  // Fetch available models
  const { data: availableModels = [] } = useQuery({
    queryKey: workflowKeys.models(),
    queryFn: () => workflowsApi.listAvailableModels(),
    staleTime: 60 * 60 * 1000,
    enabled: open,
  });

  // UI state
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showNodePicker, setShowNodePicker] = useState(false);

  // ── Preview state ────────────────────────────────────────────────────
  const [previewResults, setPreviewResults] = useState<PreviewNodeResult[] | null>(null);
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [triggersOpen, setTriggersOpen] = useState(false);
  const [showGraph, setShowGraph] = useState(false);
  const [metadataExpanded, setMetadataExpanded] = useState(isNew);
  const [pendingDeleteNodeId, setPendingDeleteNodeId] = useState<string | null>(null);
  const [leftPanelWidth, setLeftPanelWidth] = useState(340);
  const isDraggingRef = useRef(false);
  const bodyRef = useRef<HTMLDivElement>(null);

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
    (nodeId: string, key: string, value: unknown) => {
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

  // Compute which nodes have validation issues or zero records in preview results
  const previewWarningNodes = useMemo(() => {
    const warnings = new Map<string, string>(); // nodeId -> warning message
    if (!previewResults) return warnings;
    for (const r of previewResults) {
      try {
        const parsed = JSON.parse(r.output);
        // Check if output is an array of records with validation_errors
        if (Array.isArray(parsed)) {
          const withErrors = parsed.filter((rec: Record<string, unknown>) =>
            rec.validation_errors && Array.isArray(rec.validation_errors) && rec.validation_errors.length > 0
          );
          if (withErrors.length > 0) {
            warnings.set(r.node_id, `${withErrors.length} record${withErrors.length !== 1 ? 's' : ''} with validation issues`);
          } else if (parsed.length === 0 && (r.node_type === 'llm_extract' || r.node_type === 'output')) {
            warnings.set(r.node_id, 'No records extracted');
          }
        } else if (parsed && typeof parsed === 'object') {
          // Check if the result object itself has validation_errors
          if (parsed.validation_errors && Array.isArray(parsed.validation_errors) && parsed.validation_errors.length > 0) {
            warnings.set(r.node_id, `${parsed.validation_errors.length} validation issue${parsed.validation_errors.length !== 1 ? 's' : ''}`);
          }
          // Check for records array inside the result
          if (parsed.records && Array.isArray(parsed.records)) {
            const withErrors = parsed.records.filter((rec: Record<string, unknown>) =>
              rec.validation_errors && Array.isArray(rec.validation_errors) && rec.validation_errors.length > 0
            );
            if (withErrors.length > 0) {
              warnings.set(r.node_id, `${withErrors.length} record${withErrors.length !== 1 ? 's' : ''} with validation issues`);
            } else if (parsed.records.length === 0 && (r.node_type === 'llm_extract' || r.node_type === 'output')) {
              warnings.set(r.node_id, 'No records extracted');
            }
          }
        }
      } catch {
        // Not JSON, skip
      }
    }
    return warnings;
  }, [previewResults]);

  // Drag-to-resize handler for the column divider
  const startResize = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDraggingRef.current = true;
    const startX = e.clientX;
    const startWidth = leftPanelWidth;

    const onMouseMove = (ev: MouseEvent) => {
      if (!isDraggingRef.current) return;
      const containerRect = bodyRef.current?.getBoundingClientRect();
      const maxWidth = containerRect ? containerRect.width - 280 : 800;
      const newWidth = Math.max(200, Math.min(maxWidth, startWidth + ev.clientX - startX));
      setLeftPanelWidth(newWidth);
    };
    const onMouseUp = () => {
      isDraggingRef.current = false;
      document.removeEventListener('mousemove', onMouseMove);
      document.removeEventListener('mouseup', onMouseUp);
      document.body.style.cursor = '';
      document.body.style.userSelect = '';
    };
    document.addEventListener('mousemove', onMouseMove);
    document.addEventListener('mouseup', onMouseUp);
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
  }, [leftPanelWidth]);

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
              variant={showGraph ? 'secondary' : 'ghost'}
              onClick={() => setShowGraph(!showGraph)}
              disabled={nodes.length === 0}
              className="gap-1.5"
            >
              {showGraph ? (
                <><List className="h-3.5 w-3.5" />List</>
              ) : (
                <><Network className="h-3.5 w-3.5" />Graph</>
              )}
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
              variant="ghost"
              onClick={() => setTriggersOpen(true)}
              disabled={isNew}
              className="gap-1.5"
              title={isNew ? 'Save workflow first to manage triggers' : 'Manage triggers'}
            >
              <Bell className="h-3.5 w-3.5" />
              Triggers
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
        <div ref={bodyRef} className="flex flex-1 min-h-0">
          {/* Left: Canvas / Node list */}
          <div
            className="flex flex-col shrink-0"
            style={{ width: selectedNodeId ? leftPanelWidth : undefined, flex: selectedNodeId ? undefined : 1 }}
          >
            {/* Workflow metadata (collapsible) */}
            <div className="border-b bg-muted/30">
              <button
                onClick={() => setMetadataExpanded(!metadataExpanded)}
                className="w-full flex items-center gap-2 px-4 py-2 hover:bg-muted/50 transition-colors text-left"
              >
                {metadataExpanded ? (
                  <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                ) : (
                  <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
                )}
                <span className="text-sm font-medium truncate flex-1">
                  {name || 'Untitled Workflow'}
                </span>
                {!metadataExpanded && description && (
                  <span className="text-xs text-muted-foreground truncate max-w-[140px]">
                    {description}
                  </span>
                )}
              </button>
              {metadataExpanded && (
                <div className="px-4 pb-3 space-y-2">
                  <div>
                    <Label className="text-xs text-muted-foreground">Name</Label>
                    <Input
                      value={name}
                      onChange={(e) => {
                        setName(e.target.value);
                        if (isNew && idLocked) {
                          setId(
                            e.target.value
                              .toLowerCase()
                              .replace(/[^a-z0-9]+/g, '_')
                              .replace(/^_|_$/g, '')
                          );
                        }
                      }}
                      placeholder="My Workflow"
                      className="h-8 text-sm mt-1"
                    />
                    {isNew && (
                      <div className="flex items-center gap-1.5 mt-1">
                        <span className="text-[10px] text-muted-foreground">
                          ID: <code className="font-mono">{id || 'auto_generated'}</code>
                        </span>
                        <button
                          type="button"
                          onClick={() => setIdLocked(!idLocked)}
                          className="p-0.5 rounded hover:bg-muted transition-colors"
                          title={idLocked ? 'Unlock to edit ID manually' : 'Lock to auto-generate from name'}
                        >
                          {idLocked ? (
                            <Lock className="h-3 w-3 text-muted-foreground" />
                          ) : (
                            <LockOpen className="h-3 w-3 text-primary" />
                          )}
                        </button>
                      </div>
                    )}
                    {isNew && !idLocked && (
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
                        className="h-7 text-xs mt-1 font-mono"
                      />
                    )}
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
                    <Select value={defaultModel || '__auto__'} onValueChange={(v) => setDefaultModel(v === '__auto__' ? '' : v)}>
                      <SelectTrigger className="h-8 text-sm mt-1">
                        <SelectValue placeholder="Auto (highest priority)" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="__auto__">Auto (highest priority)</SelectItem>
                        {availableModels.map((m: AvailableModel) => (
                          <SelectItem key={m.id} value={m.id}>
                            <span className="flex items-center gap-2">
                              <span>{m.label}</span>
                              <span className="text-muted-foreground text-xs">
                                ${(m.cost_per_million_input / 100).toFixed(2)}/M in
                              </span>
                            </span>
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>
                </div>
              )}
            </div>

            {/* Node list or inline graph */}
            {showGraph ? (
              <ScrollArea className="flex-1">
                <div className="p-4">
                  <WorkflowGraphView
                    nodes={nodes}
                    connections={connections}
                    selectedNodeId={selectedNodeId}
                    onSelectNode={(nodeId) => {
                      setSelectedNodeId(selectedNodeId === nodeId ? null : nodeId);
                    }}
                  />
                </div>
              </ScrollArea>
            ) : (
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
                                  className="text-[10px] px-1.5 text-amber-600 border-amber-200 gap-0.5"
                                  title={previewWarningNodes.get(node.id)}
                                >
                                  <AlertTriangle className="h-2.5 w-2.5" />
                                </Badge>
                              )}
                              <Badge
                                variant="outline"
                                className="text-[10px] px-1.5"
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
            )}
          </div>

          {/* Resize handle */}
          {selectedNodeId && (
            <div
              onMouseDown={startResize}
              className="w-1 hover:w-1.5 bg-border hover:bg-primary/40 cursor-col-resize shrink-0 transition-colors"
            />
          )}
          {!selectedNodeId && <div className="w-px bg-border shrink-0" />}

          {/* Right: Node configuration panel (n8n style) */}
          <div className={cn(
            'flex flex-col bg-muted/20',
            selectedNodeId ? 'flex-1 min-w-[280px] max-w-[480px]' : 'w-[320px]'
          )}>
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
                      <div key={r.node_id} className={cn(
                        'rounded-lg border bg-card p-3',
                        previewWarningNodes.has(r.node_id) && 'border-amber-300 bg-amber-50/30 dark:bg-amber-950/10'
                      )}>
                        <div className="flex items-center gap-2 mb-2">
                          <div className={cn('w-5 h-5 rounded flex items-center justify-center text-white text-[10px]', typeDef?.color ?? 'bg-gray-500')}>
                            {r.node_name.charAt(0)}
                          </div>
                          <span className="text-sm font-medium">{r.node_name}</span>
                          <Badge variant="outline" className="text-[10px]">{r.node_type}</Badge>
                          {previewWarningNodes.has(r.node_id) && (
                            <Badge variant="outline" className="text-[10px] text-amber-600 border-amber-200 gap-0.5">
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

      {/* Delete confirmation modal */}
      <Dialog open={!!pendingDeleteNodeId} onOpenChange={(open) => !open && setPendingDeleteNodeId(null)}>
        <DialogContent className="max-w-sm">
          <div className="flex flex-col items-center text-center py-2">
            <div className="w-10 h-10 rounded-full bg-destructive/10 flex items-center justify-center mb-3">
              <AlertTriangle className="h-5 w-5 text-destructive" />
            </div>
            <DialogTitle className="text-base">Delete Node</DialogTitle>
            <p className="text-sm text-muted-foreground mt-1.5">
              Delete <strong>{nodes.find(n => n.id === pendingDeleteNodeId)?.name ?? 'this node'}</strong>?
              This will also remove its connections. This cannot be undone.
            </p>
            <div className="flex gap-2 mt-4 w-full">
              <Button
                variant="outline"
                size="sm"
                className="flex-1"
                onClick={() => setPendingDeleteNodeId(null)}
              >
                Cancel
              </Button>
              <Button
                variant="destructive"
                size="sm"
                className="flex-1"
                onClick={() => {
                  if (pendingDeleteNodeId) {
                    deleteNode(pendingDeleteNodeId);
                    setPendingDeleteNodeId(null);
                  }
                }}
              >
                Delete
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {!isNew && (
        <WorkflowTriggersPanel
          open={triggersOpen}
          onOpenChange={setTriggersOpen}
          workflowId={id}
          workflowName={name}
        />
      )}
    </Dialog>
  );
}

export default WorkflowEditor;
