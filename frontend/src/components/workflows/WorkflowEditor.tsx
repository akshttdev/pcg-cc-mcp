import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
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
  Link,
  Unlink,
  Users,
  Building2,
  Handshake,
  ListTodo,
  ChevronDown,
  ChevronRight,
  AlertTriangle,
  List,
  Database,
  GitBranch,
  Bell,
  Bot,
  Globe,
  PenSquare,
  RefreshCw,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { WorkflowTriggersPanel } from './WorkflowTriggersPanel';
import { workflowsApi, crmPipelinesApi, DATA_TYPE_OPTIONS } from '@/lib/api';
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
    type: 'data_source',
    label: 'Data Source',
    description: 'Marks this workflow as data-source-driven (source selected at run time)',
    icon: Database,
    color: 'bg-slate-600',
    defaultParameters: {
      accepted_types: '', // optional: comma-separated data types this workflow can process
    },
  },
  {
    type: 'llm_extract',
    label: 'LLM Extract',
    description: 'Extract structured data using an LLM prompt',
    icon: FileSearch,
    color: 'bg-blue-500',
    defaultParameters: {
      prompt_template: '',
      output_schema: '',
      output_mode: 'auto',
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
      output_mode: 'auto',
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
      output_mode: 'auto',
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
  // --- Control Flow ---
  {
    type: 'conditional',
    label: 'Conditional',
    description: 'Branch execution based on a condition (routes to different downstream nodes)',
    icon: GitBranch,
    color: 'bg-yellow-600',
    defaultParameters: {
      condition: '',
      true_label: 'Yes',
      false_label: 'No',
    },
  },
  // --- Action Nodes ---
  {
    type: 'send_notification',
    label: 'Send Notification',
    description: 'Send an in-app notification or email alert',
    icon: Bell,
    color: 'bg-pink-500',
    defaultParameters: {
      notification_type: 'in_app',
      recipient: '',
      subject: '',
      message_template: '',
    },
  },
  {
    type: 'assign_to_agent',
    label: 'Assign to Agent',
    description: 'Create a task and assign it to an AI agent for autonomous execution',
    icon: Bot,
    color: 'bg-indigo-600',
    defaultParameters: {
      agent_codename: '',
      task_title_template: '',
      task_description_template: '',
      completion_criteria: '',
      auto_start: true,
    },
  },
  {
    type: 'http_request',
    label: 'HTTP Request',
    description: 'Make an external API call and use the response downstream',
    icon: Globe,
    color: 'bg-sky-500',
    defaultParameters: {
      method: 'GET',
      url: '',
      headers: '{}',
      body: '',
      output_path: '', // JSONPath to extract from response
    },
  },
  // --- CRM Action Nodes ---
  {
    type: 'update_crm_contact',
    label: 'Update CRM Contact',
    description: 'Update fields on existing CRM contacts (e.g., lifecycle stage, tags)',
    icon: PenSquare,
    color: 'bg-cyan-600',
    defaultParameters: {
      match_field: 'email',
      update_fields: '{}',
    },
  },
  {
    type: 'update_crm_deal',
    label: 'Update CRM Deal',
    description: 'Update deal stage, value, or other fields on existing deals',
    icon: RefreshCw,
    color: 'bg-cyan-600',
    defaultParameters: {
      match_field: 'name',
      update_fields: '{}',
    },
  },
  {
    type: 'update_crm_company',
    label: 'Update Company',
    description: 'Update fields on existing company records (e.g., relationship, industry)',
    icon: Building2,
    color: 'bg-cyan-600',
    defaultParameters: {
      match_field: 'name',
      update_fields: '{}',
    },
  },
];

export function getNodeTypeDef(type: string): NodeTypeDefinition | undefined {
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

  // Compute which nodes have validation issues or zero records in preview results
  const previewWarningNodes = useMemo(() => {
    const warnings = new Map<string, string>(); // nodeId -> warning message
    if (!previewResults) return warnings;
    for (const r of previewResults) {
      try {
        const parsed = JSON.parse(r.output);
        // Check if output is an array of records with validation_errors
        if (Array.isArray(parsed)) {
          const withErrors = parsed.filter((rec: any) =>
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
            const withErrors = parsed.records.filter((rec: any) =>
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
  const [isPreviewing, setIsPreviewing] = useState(false);
  const [showPreview, setShowPreview] = useState(false);
  const [triggersOpen, setTriggersOpen] = useState(false);
  const [showGraph, setShowGraph] = useState(false);
  const [metadataExpanded, setMetadataExpanded] = useState(isNew);
  const [pendingDeleteNodeId, setPendingDeleteNodeId] = useState<string | null>(null);
  const [leftPanelWidth, setLeftPanelWidth] = useState(340);
  const isDraggingRef = useRef(false);
  const bodyRef = useRef<HTMLDivElement>(null);

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
                    onSelectNode={(id) => {
                      setSelectedNodeId(id);
                      setShowGraph(false);
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
            selectedNodeId ? 'flex-1 min-w-[280px]' : 'w-[320px]'
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

// ── Static graph visualization ────────────────────────────────────────────────

function WorkflowGraphView({ nodes, connections, onSelectNode }: { nodes: WorkflowNode[]; connections: WorkflowConnection[]; onSelectNode?: (id: string) => void }) {
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
            <g
              key={node.id}
              onClick={() => onSelectNode?.(node.id)}
              className={onSelectNode ? 'cursor-pointer' : undefined}
            >
              <rect
                x={pos.x} y={pos.y}
                width={nodeWidth} height={nodeHeight}
                rx="8" ry="8"
                fill="hsl(var(--card))"
                stroke={fill}
                strokeWidth="2"
              />
              {onSelectNode && (
                <rect
                  x={pos.x} y={pos.y}
                  width={nodeWidth} height={nodeHeight}
                  rx="8" ry="8"
                  fill="transparent"
                  className="hover:fill-primary/5"
                />
              )}
              <rect
                x={pos.x} y={pos.y}
                width="6" height={nodeHeight}
                rx="8" ry="0"
                fill={fill}
              />
              <title>{node.name} ({typeDef?.label ?? node.type})</title>
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
  const sourceNodes = NODE_TYPES.filter(nt => nt.type === 'data_source');
  const llmNodes = NODE_TYPES.filter(nt => nt.type.startsWith('llm_'));
  const transformNodes = NODE_TYPES.filter(nt => ['transform', 'filter', 'merge', 'conditional'].includes(nt.type));
  const actionNodes = NODE_TYPES.filter(nt => ['send_notification', 'assign_to_agent', 'http_request', 'update_crm_contact', 'update_crm_deal', 'update_crm_company'].includes(nt.type));
  const outputNodes = NODE_TYPES.filter(nt => nt.type.startsWith('output_'));
  const processingNodes = [...sourceNodes, ...llmNodes, ...transformNodes];

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
      {actionNodes.length > 0 && (
        <>
          <div className="border-t my-1" />
          <div className="px-2 py-0.5">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">Actions</span>
          </div>
          {actionNodes.map((nt) => {
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
                <div className={cn('w-8 h-8 rounded-md flex items-center justify-center text-white shrink-0', nt.color)}>
                  <Icon className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <div className="text-sm font-medium">{nt.label}</div>
                  <div className="text-xs text-muted-foreground">{nt.description}</div>
                </div>
              </button>
            );
          })}
        </>
      )}
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

  // Available nodes to connect from (exclude self, already-connected, and output nodes)
  const availableInputs = allNodes.filter(
    (n) => n.id !== node.id && !currentInputs.includes(n.id) && !n.type.startsWith('output_')
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
                    className="flex items-center gap-2 rounded-md border bg-primary/5 border-primary/20 px-2 py-1.5"
                  >
                    <div
                      className={cn(
                        'w-5 h-5 rounded flex items-center justify-center text-white shrink-0',
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
                      className="p-0.5 rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
                      title="Disconnect"
                    >
                      <Unlink className="h-3.5 w-3.5" />
                    </button>
                  </div>
                );
              })}

              {currentInputs.length === 0 && (
                <p className="text-xs text-muted-foreground italic">
                  No inputs — receives raw data source content.
                </p>
              )}

              {availableInputs.length > 0 && (
                <Select onValueChange={(v) => onAddConnection(v)}>
                  <SelectTrigger className="h-8 text-sm border-dashed">
                    <div className="flex items-center gap-1.5 text-muted-foreground">
                      <Link className="h-3.5 w-3.5" />
                      <span>Add input connection...</span>
                    </div>
                  </SelectTrigger>
                  <SelectContent>
                    {availableInputs.map((n) => {
                      const nDef = getNodeTypeDef(n.type);
                      return (
                        <SelectItem key={n.id} value={n.id}>
                          <span className="flex items-center gap-2">
                            <span
                              className={cn(
                                'w-4 h-4 rounded flex items-center justify-center text-white shrink-0 text-[10px]',
                                nDef?.color ?? 'bg-gray-500'
                              )}
                            >
                              {(nDef?.label ?? '?')[0]}
                            </span>
                            {n.name}
                          </span>
                        </SelectItem>
                      );
                    })}
                  </SelectContent>
                </Select>
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

            {node.type === 'data_source' && (
              <DataSourceNodeConfig
                acceptedTypes={node.parameters.accepted_types || ''}
                onUpdateAcceptedTypes={(v) => onUpdateParameter('accepted_types', v)}
              />
            )}

            {isLLMNode && (
              <div className="space-y-3">
                {/* Model selection */}
                <div>
                  <Label className="text-xs">Model</Label>
                  <Select
                    value={node.parameters.model || '__default__'}
                    onValueChange={(v) => onUpdateParameter('model', v === '__default__' ? undefined : v)}
                  >
                    <SelectTrigger className="h-8 text-sm mt-1">
                      <SelectValue placeholder="Use workflow default" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="__default__">Use workflow default</SelectItem>
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
                  <div className="flex flex-wrap gap-1 mt-1.5 mb-1.5">
                    <button
                      type="button"
                      onClick={() => {
                        const ta = document.querySelector<HTMLTextAreaElement>(`[data-prompt-node="${node.id}"]`);
                        if (ta) {
                          const pos = ta.selectionStart ?? ta.value.length;
                          const before = ta.value.slice(0, pos);
                          const after = ta.value.slice(pos);
                          onUpdateParameter('prompt_template', before + '{{content}}' + after);
                        } else {
                          onUpdateParameter('prompt_template', (node.parameters.prompt_template ?? '') + '{{content}}');
                        }
                      }}
                      className="inline-flex items-center gap-1 rounded-md bg-blue-500/10 border border-blue-500/20 px-2 py-0.5 text-[11px] font-mono text-blue-700 dark:text-blue-300 hover:bg-blue-500/20 transition-colors"
                    >
                      {'{{content}}'}
                      <span className="text-[9px] font-sans text-muted-foreground">raw input</span>
                    </button>
                    {currentInputs.map((sourceId) => {
                      const sourceNode = allNodes.find((n) => n.id === sourceId);
                      const rawSchema = (sourceNode?.parameters?.output_schema as string) ?? '';
                      const schemaName = rawSchema.replace(/\[\]$/, '');
                      if (!schemaName) return null;
                      const varName = `{{${schemaName}}}`;
                      return (
                        <button
                          key={sourceId}
                          type="button"
                          onClick={() => {
                            const ta = document.querySelector<HTMLTextAreaElement>(`[data-prompt-node="${node.id}"]`);
                            if (ta) {
                              const pos = ta.selectionStart ?? ta.value.length;
                              const before = ta.value.slice(0, pos);
                              const after = ta.value.slice(pos);
                              onUpdateParameter('prompt_template', before + varName + after);
                            } else {
                              onUpdateParameter('prompt_template', (node.parameters.prompt_template ?? '') + varName);
                            }
                          }}
                          className="inline-flex items-center gap-1 rounded-md bg-green-500/10 border border-green-500/20 px-2 py-0.5 text-[11px] font-mono text-green-700 dark:text-green-300 hover:bg-green-500/20 transition-colors"
                        >
                          {varName}
                          <span className="text-[9px] font-sans text-muted-foreground">
                            from {sourceNode?.name ?? sourceId}
                          </span>
                        </button>
                      );
                    })}
                    {currentInputs.length > 0 && (
                      <button
                        type="button"
                        onClick={() => {
                          const ta = document.querySelector<HTMLTextAreaElement>(`[data-prompt-node="${node.id}"]`);
                          if (ta) {
                            const pos = ta.selectionStart ?? ta.value.length;
                            const before = ta.value.slice(0, pos);
                            const after = ta.value.slice(pos);
                            onUpdateParameter('prompt_template', before + '{{previous_results}}' + after);
                          } else {
                            onUpdateParameter('prompt_template', (node.parameters.prompt_template ?? '') + '{{previous_results}}');
                          }
                        }}
                        className="inline-flex items-center gap-1 rounded-md bg-purple-500/10 border border-purple-500/20 px-2 py-0.5 text-[11px] font-mono text-purple-700 dark:text-purple-300 hover:bg-purple-500/20 transition-colors"
                      >
                        {'{{previous_results}}'}
                        <span className="text-[9px] font-sans text-muted-foreground">
                          all inputs combined
                        </span>
                      </button>
                    )}
                  </div>
                  <Textarea
                    data-prompt-node={node.id}
                    value={node.parameters.prompt_template ?? ''}
                    onChange={(e) =>
                      onUpdateParameter('prompt_template', e.target.value)
                    }
                    placeholder={`Enter extraction instructions here. The source content is automatically included.\n\nExample for contacts:\nExtract all people mentioned. For each: first_name, last_name, email, phone, company_name, job_title.\n\nExample for companies:\nExtract all companies mentioned. For each: name, website, industry, description.\n\nTip: Use {{content}} to reference the source data explicitly.`}
                    className="text-sm font-mono min-h-[200px] resize-y"
                  />
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
                <div>
                  <Label className="text-xs">Output Mode</Label>
                  <div className="flex gap-1 mt-1">
                    {([
                      { value: 'auto', label: 'Auto', desc: 'JSON if schema set' },
                      { value: 'structured', label: 'Structured', desc: 'Always JSON' },
                      { value: 'text', label: 'Text', desc: 'Raw response' },
                    ] as const).map((opt) => (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() => onUpdateParameter('output_mode', opt.value)}
                        className={cn(
                          'flex-1 rounded-md px-2 py-1.5 text-xs font-medium transition-all border',
                          (node.parameters.output_mode || 'auto') === opt.value
                            ? 'bg-primary text-primary-foreground border-primary'
                            : 'bg-transparent border-border/60 hover:bg-accent text-muted-foreground'
                        )}
                      >
                        {opt.label}
                      </button>
                    ))}
                  </div>
                  <p className="text-[10px] text-muted-foreground mt-1">
                    Auto uses JSON when output schema is defined, text otherwise.
                  </p>
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

            {node.type === 'conditional' && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Condition</Label>
                  <Textarea
                    value={node.parameters.condition ?? ''}
                    onChange={(e) => onUpdateParameter('condition', e.target.value)}
                    placeholder="Describe the branching condition. e.g., 'If confidence > 0.8' or 'If lifecycle_stage equals SQL'"
                    className="mt-1 text-sm font-mono min-h-[100px]"
                  />
                </div>
                <div className="grid grid-cols-2 gap-2">
                  <div>
                    <Label className="text-xs">True Branch Label</Label>
                    <Input
                      value={node.parameters.true_label ?? 'Yes'}
                      onChange={(e) => onUpdateParameter('true_label', e.target.value)}
                      className="h-8 text-sm mt-1"
                    />
                  </div>
                  <div>
                    <Label className="text-xs">False Branch Label</Label>
                    <Input
                      value={node.parameters.false_label ?? 'No'}
                      onChange={(e) => onUpdateParameter('false_label', e.target.value)}
                      className="h-8 text-sm mt-1"
                    />
                  </div>
                </div>
              </div>
            )}

            {node.type === 'send_notification' && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Notification Type</Label>
                  <Select
                    value={node.parameters.notification_type ?? 'in_app'}
                    onValueChange={(v) => onUpdateParameter('notification_type', v)}
                  >
                    <SelectTrigger className="h-8 text-sm mt-1"><SelectValue /></SelectTrigger>
                    <SelectContent>
                      <SelectItem value="in_app">In-App Notification</SelectItem>
                      <SelectItem value="email">Email</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <div>
                  <Label className="text-xs">Recipient</Label>
                  <Input
                    value={node.parameters.recipient ?? ''}
                    onChange={(e) => onUpdateParameter('recipient', e.target.value)}
                    placeholder="User email or 'admin' or 'assignee'"
                    className="h-8 text-sm mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs">Subject</Label>
                  <Input
                    value={node.parameters.subject ?? ''}
                    onChange={(e) => onUpdateParameter('subject', e.target.value)}
                    placeholder="Notification subject"
                    className="h-8 text-sm mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs">Message Template</Label>
                  <Textarea
                    value={node.parameters.message_template ?? ''}
                    onChange={(e) => onUpdateParameter('message_template', e.target.value)}
                    placeholder="Message body. Use {{variable}} for dynamic content."
                    className="mt-1 text-sm min-h-[80px]"
                  />
                </div>
              </div>
            )}

            {node.type === 'assign_to_agent' && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Agent Codename</Label>
                  <Input
                    value={node.parameters.agent_codename ?? ''}
                    onChange={(e) => onUpdateParameter('agent_codename', e.target.value)}
                    placeholder="e.g., auri, nora, astra"
                    className="h-8 text-sm mt-1"
                  />
                  <p className="text-xs text-muted-foreground mt-1">The agent to assign the task to.</p>
                </div>
                <div>
                  <Label className="text-xs">Task Title Template</Label>
                  <Input
                    value={node.parameters.task_title_template ?? ''}
                    onChange={(e) => onUpdateParameter('task_title_template', e.target.value)}
                    placeholder="e.g., Implement {{feature_name}}"
                    className="h-8 text-sm mt-1"
                  />
                </div>
                <div>
                  <Label className="text-xs">Task Description Template</Label>
                  <Textarea
                    value={node.parameters.task_description_template ?? ''}
                    onChange={(e) => onUpdateParameter('task_description_template', e.target.value)}
                    placeholder="Describe what the agent should do. Use {{variable}} for upstream data."
                    className="mt-1 text-sm min-h-[80px]"
                  />
                </div>
                <div>
                  <Label className="text-xs">Completion Criteria</Label>
                  <Textarea
                    value={node.parameters.completion_criteria ?? ''}
                    onChange={(e) => onUpdateParameter('completion_criteria', e.target.value)}
                    placeholder="What must be true for this task to be done?"
                    className="mt-1 text-sm min-h-[60px]"
                  />
                </div>
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={node.parameters.auto_start ?? true}
                    onChange={(e) => onUpdateParameter('auto_start', e.target.checked)}
                    className="rounded"
                  />
                  <Label className="text-xs">Auto-start agent execution after task creation</Label>
                </div>
              </div>
            )}

            {node.type === 'http_request' && (
              <div className="space-y-3">
                <div className="grid grid-cols-3 gap-2">
                  <div>
                    <Label className="text-xs">Method</Label>
                    <Select
                      value={node.parameters.method ?? 'GET'}
                      onValueChange={(v) => onUpdateParameter('method', v)}
                    >
                      <SelectTrigger className="h-8 text-sm mt-1"><SelectValue /></SelectTrigger>
                      <SelectContent>
                        <SelectItem value="GET">GET</SelectItem>
                        <SelectItem value="POST">POST</SelectItem>
                        <SelectItem value="PUT">PUT</SelectItem>
                        <SelectItem value="PATCH">PATCH</SelectItem>
                        <SelectItem value="DELETE">DELETE</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="col-span-2">
                    <Label className="text-xs">URL</Label>
                    <Input
                      value={node.parameters.url ?? ''}
                      onChange={(e) => onUpdateParameter('url', e.target.value)}
                      placeholder="https://api.example.com/endpoint"
                      className="h-8 text-sm mt-1"
                    />
                  </div>
                </div>
                <div>
                  <Label className="text-xs">Headers (JSON)</Label>
                  <Textarea
                    value={node.parameters.headers ?? '{}'}
                    onChange={(e) => onUpdateParameter('headers', e.target.value)}
                    placeholder='{"Authorization": "Bearer {{token}}"}'
                    className="mt-1 text-sm font-mono min-h-[60px]"
                  />
                </div>
                <div>
                  <Label className="text-xs">Request Body</Label>
                  <Textarea
                    value={node.parameters.body ?? ''}
                    onChange={(e) => onUpdateParameter('body', e.target.value)}
                    placeholder="Request body (for POST/PUT). Use {{variable}} for upstream data."
                    className="mt-1 text-sm font-mono min-h-[60px]"
                  />
                </div>
                <div>
                  <Label className="text-xs">Output Path (JSONPath)</Label>
                  <Input
                    value={node.parameters.output_path ?? ''}
                    onChange={(e) => onUpdateParameter('output_path', e.target.value)}
                    placeholder="e.g., data.results or leave empty for full response"
                    className="h-8 text-sm mt-1"
                  />
                </div>
              </div>
            )}

            {(node.type === 'update_crm_contact' || node.type === 'update_crm_deal' || node.type === 'update_crm_company') && (
              <div className="space-y-3">
                <div>
                  <Label className="text-xs">Match Field</Label>
                  <Input
                    value={node.parameters.match_field ?? 'email'}
                    onChange={(e) => onUpdateParameter('match_field', e.target.value)}
                    placeholder={node.type === 'update_crm_contact' ? 'email' : 'name'}
                    className="h-8 text-sm mt-1"
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    Field used to find the existing record to update.
                  </p>
                </div>
                <div>
                  <Label className="text-xs">Fields to Update (JSON)</Label>
                  <Textarea
                    value={node.parameters.update_fields ?? '{}'}
                    onChange={(e) => onUpdateParameter('update_fields', e.target.value)}
                    placeholder={'{"lifecycle_stage": "{{new_stage}}", "tags": ["{{tag}}"]}\nUse {{variable}} for upstream data.'}
                    className="mt-1 text-sm font-mono min-h-[100px]"
                  />
                </div>
              </div>
            )}

            {node.type.startsWith('output_') && (
              <OutputNodeConfig
                node={node}
                onUpdateParameter={onUpdateParameter}
              />
            )}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

// ── Target schema definitions for output nodes ──────────────────────────────

const TARGET_SCHEMAS: Record<string, { label: string; fields: { name: string; type: string; required?: boolean }[] }> = {
  crm_contact: {
    label: 'CRM Contact',
    fields: [
      { name: 'first_name', type: 'string', required: true },
      { name: 'last_name', type: 'string', required: true },
      { name: 'email', type: 'string', required: true },
      { name: 'phone', type: 'string' },
      { name: 'company_name', type: 'string' },
      { name: 'job_title', type: 'string' },
      { name: 'department', type: 'string' },
      { name: 'linkedin_url', type: 'url' },
      { name: 'lifecycle_stage', type: 'enum: subscriber|lead|mql|sql|opportunity|customer' },
      { name: 'source', type: 'string' },
      { name: 'tags', type: 'string[]' },
    ],
  },
  company: {
    label: 'Company',
    fields: [
      { name: 'name', type: 'string', required: true },
      { name: 'domain', type: 'url' },
      { name: 'industry', type: 'string' },
      { name: 'size', type: 'string' },
      { name: 'description', type: 'string' },
      { name: 'phone', type: 'string' },
      { name: 'email', type: 'string' },
      { name: 'address', type: 'string' },
      { name: 'city', type: 'string' },
      { name: 'country', type: 'string' },
      { name: 'linkedin_url', type: 'url' },
      { name: 'tags', type: 'string[]' },
    ],
  },
  crm_deal: {
    label: 'CRM Deal',
    fields: [
      { name: 'name', type: 'string', required: true },
      { name: 'amount', type: 'number' },
      { name: 'currency', type: 'string (ISO 4217)' },
      { name: 'probability', type: 'number (0-100)' },
      { name: 'expected_close_date', type: 'date (YYYY-MM-DD)' },
      { name: 'description', type: 'string' },
      { name: 'contact_email', type: 'string' },
      { name: 'tags', type: 'string[]' },
    ],
  },
  task: {
    label: 'Task',
    fields: [
      { name: 'title', type: 'string', required: true },
      { name: 'description', type: 'string' },
      { name: 'status', type: 'enum: todo|inprogress|done' },
      { name: 'priority', type: 'enum: low|medium|high|critical' },
      { name: 'tags', type: 'string[]' },
    ],
  },
};

// ── Output node config with schema preview + pipeline selector ──────────────

function OutputNodeConfig({
  node,
  onUpdateParameter,
}: {
  node: WorkflowNode;
  onUpdateParameter: (key: string, value: any) => void;
}) {
  const targetType = node.parameters.target_type as string;
  const schema = TARGET_SCHEMAS[targetType];
  const [schemaExpanded, setSchemaExpanded] = useState(false);

  // Fetch pipelines for CRM Deals output
  const { data: pipelines = [] } = useQuery({
    queryKey: ['crmPipelines'],
    queryFn: async () => {
      try {
        return await crmPipelinesApi.listOrgPipelines('01010101-0101-0101-0101-010101010101');
      } catch {
        return [];
      }
    },
    enabled: node.type === 'output_crm_deals',
    staleTime: 5 * 60 * 1000,
  });

  // Fetch stages for selected pipeline
  const selectedPipelineId = node.parameters.pipeline_id as string | undefined;
  const { data: pipelineWithStages } = useQuery({
    queryKey: ['crmPipelineStages', selectedPipelineId],
    queryFn: () => crmPipelinesApi.getPipeline(selectedPipelineId!),
    enabled: !!selectedPipelineId && node.type === 'output_crm_deals',
    staleTime: 5 * 60 * 1000,
  });
  const stages: any[] = (pipelineWithStages as any)?.stages ?? [];

  return (
    <div className="space-y-3">
      <div>
        <Label className="text-xs">Target</Label>
        <div className="mt-1 text-sm text-muted-foreground bg-muted/50 rounded px-2 py-1.5 flex items-center gap-2">
          {node.type === 'output_crm_contacts' && <><Users className="h-3.5 w-3.5 text-blue-500" /> CRM Contacts</>}
          {node.type === 'output_crm_companies' && <><Building2 className="h-3.5 w-3.5 text-purple-500" /> Companies</>}
          {node.type === 'output_crm_deals' && <><Handshake className="h-3.5 w-3.5 text-green-500" /> CRM Deals</>}
          {node.type === 'output_tasks' && <><ListTodo className="h-3.5 w-3.5 text-orange-500" /> Tasks</>}
        </div>
      </div>

      {/* Pipeline & Stage selector for CRM Deals */}
      {node.type === 'output_crm_deals' && (
        <>
          <div>
            <Label className="text-xs">Pipeline</Label>
            <Select
              value={selectedPipelineId || '__none__'}
              onValueChange={(v) => {
                onUpdateParameter('pipeline_id', v === '__none__' ? undefined : v);
                onUpdateParameter('stage_id', undefined);
              }}
            >
              <SelectTrigger className="h-8 text-sm mt-1">
                <SelectValue placeholder="Select pipeline..." />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="__none__">Auto-assign</SelectItem>
                {pipelines.map((p: any) => (
                  <SelectItem key={p.id} value={p.id}>{p.name}</SelectItem>
                ))}
              </SelectContent>
            </Select>
            <p className="text-[10px] text-muted-foreground mt-1">
              Which CRM pipeline to create deals in.
            </p>
          </div>
          {stages.length > 0 && (
            <div>
              <Label className="text-xs">Initial Stage</Label>
              <Select
                value={(node.parameters.stage_id as string) || '__first__'}
                onValueChange={(v) => onUpdateParameter('stage_id', v === '__first__' ? undefined : v)}
              >
                <SelectTrigger className="h-8 text-sm mt-1">
                  <SelectValue placeholder="First stage" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__first__">First stage ({stages[0]?.name})</SelectItem>
                  {stages.map((s: any) => (
                    <SelectItem key={s.id} value={s.id}>{s.name}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <p className="text-[10px] text-muted-foreground mt-1">
                Stage new deals start in.
              </p>
            </div>
          )}
        </>
      )}

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

      {/* Schema preview */}
      {schema && (
        <div className="rounded-md border border-muted-foreground/20 overflow-hidden">
          <button
            onClick={() => setSchemaExpanded(!schemaExpanded)}
            className="w-full flex items-center gap-2 px-2.5 py-1.5 bg-muted/30 hover:bg-muted/50 transition-colors text-left"
          >
            {schemaExpanded ? (
              <ChevronDown className="h-3 w-3 text-muted-foreground shrink-0" />
            ) : (
              <ChevronRight className="h-3 w-3 text-muted-foreground shrink-0" />
            )}
            <span className="text-[11px] font-medium text-muted-foreground">
              {schema.label} Schema ({schema.fields.length} fields)
            </span>
          </button>
          {schemaExpanded && (
            <div className="px-2.5 py-2 space-y-0.5 bg-muted/10">
              {schema.fields.map((f) => (
                <div key={f.name} className="flex items-center gap-2 text-[10px] font-mono">
                  <span className={cn('truncate', f.required ? 'text-foreground font-semibold' : 'text-muted-foreground')}>
                    {f.name}{f.required ? '*' : ''}
                  </span>
                  <span className="text-muted-foreground/60 truncate">{f.type}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      <div className="rounded-md border border-dashed border-muted-foreground/30 p-2.5 bg-muted/20">
        <p className="text-[10px] text-muted-foreground">
          Connect an LLM node as input. The schema for the target type will be automatically injected into the upstream LLM prompt.
        </p>
      </div>
    </div>
  );
}

// ─── Data Source Node Config ────────────────────────────────────────────────────

function DataSourceNodeConfig({ acceptedTypes, onUpdateAcceptedTypes }: {
  acceptedTypes: string;
  onUpdateAcceptedTypes: (v: string) => void;
}) {
  return (
    <div className="space-y-3">
      <div className="rounded-md border border-dashed border-muted-foreground/30 p-2.5 bg-muted/20">
        <p className="text-[10px] text-muted-foreground">
          This node marks the workflow as data-source-driven. The data source will be selected at run time.
          Workflows with this node will appear in the Data Library's "Run Workflow" action.
        </p>
      </div>
      <div>
        <Label className="text-xs">Accepted Data Type</Label>
        <Select
          value={acceptedTypes || '__any__'}
          onValueChange={(v) => onUpdateAcceptedTypes(v === '__any__' ? '' : v)}
        >
          <SelectTrigger className="h-8 text-sm mt-1">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="__any__">Any data source</SelectItem>
            {DATA_TYPE_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <p className="text-[10px] text-muted-foreground mt-1">
          Filter which data source types this workflow can process.
        </p>
      </div>
    </div>
  );
}
