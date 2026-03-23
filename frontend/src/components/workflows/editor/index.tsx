import { useState, useCallback, useEffect, useMemo, useRef } from 'react';
import { useQuery } from '@tanstack/react-query';
import { workflowKeys } from '@/lib/query-keys';
import {
  Dialog,
  DialogContent,
} from '@/components/ui/dialog';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Settings2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { WorkflowTriggersPanel } from '../WorkflowTriggersPanel';
import { workflowsApi } from '@/lib/api';
import type {
  WorkflowNode,
  WorkflowConnection,
  WorkflowDefinition,
  PreviewNodeResult,
} from '@/lib/api';

import { getNodeTypeDef } from './node-types';
import { WorkflowGraphView } from './WorkflowGraphView';
import { NodeConfigPanel } from './NodeConfigPanel';
import { MetadataPanel } from './MetadataPanel';
import { NodeListPanel } from './NodeListPanel';
import { PreviewPanel } from './PreviewPanel';
import { DeleteNodeDialog } from './DeleteNodeDialog';
import { EditorHeader } from './EditorHeader';
import { computePreviewWarnings } from './computePreviewWarnings';

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

  const previewWarningNodes = useMemo(
    () => computePreviewWarnings(previewResults),
    [previewResults]
  );

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
        <EditorHeader
          isNew={isNew}
          workflow={workflow}
          showGraph={showGraph}
          setShowGraph={setShowGraph}
          nodesCount={nodes.length}
          isPreviewing={isPreviewing}
          handlePreview={handlePreview}
          setTriggersOpen={setTriggersOpen}
          onOpenChange={onOpenChange}
          handleSave={handleSave}
          canSave={!!canSave}
          isSaving={isSaving}
        />

        {/* Body: two-panel layout like n8n */}
        <div ref={bodyRef} className="flex flex-1 min-h-0">
          {/* Left: Canvas / Node list */}
          <div
            className="flex flex-col shrink-0"
            style={{ width: selectedNodeId ? leftPanelWidth : undefined, flex: selectedNodeId ? undefined : 1 }}
          >
            <MetadataPanel
              name={name}
              setName={setName}
              description={description}
              setDescription={setDescription}
              id={id}
              setId={setId}
              idLocked={idLocked}
              setIdLocked={setIdLocked}
              isNew={isNew}
              defaultModel={defaultModel}
              setDefaultModel={setDefaultModel}
              availableModels={availableModels}
              metadataExpanded={metadataExpanded}
              setMetadataExpanded={setMetadataExpanded}
            />

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
              <NodeListPanel
                nodes={nodes}
                selectedNodeId={selectedNodeId}
                setSelectedNodeId={setSelectedNodeId}
                showNodePicker={showNodePicker}
                setShowNodePicker={setShowNodePicker}
                addNode={addNode}
                getNodeInputs={getNodeInputs}
                previewWarningNodes={previewWarningNodes}
                setPendingDeleteNodeId={setPendingDeleteNodeId}
              />
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
        <PreviewPanel
          showPreview={showPreview}
          setShowPreview={setShowPreview}
          isPreviewing={isPreviewing}
          previewResults={previewResults}
          previewWarningNodes={previewWarningNodes}
        />

      </DialogContent>

      {/* Delete confirmation modal */}
      <DeleteNodeDialog
        pendingDeleteNodeId={pendingDeleteNodeId}
        setPendingDeleteNodeId={setPendingDeleteNodeId}
        nodes={nodes}
        deleteNode={deleteNode}
      />

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
