// ─── WorkflowBuilderTab ───────────────────────────────────────────────────────
//
// Builder tab: create, edit, and manage workflow definitions using the n8n-style editor.

import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import { Plus, Trash2, Pencil, Play, Hammer, CheckCircle2, AlertCircle, Loader2 as Loader2Icon } from 'lucide-react';
import { workflowsApi } from '@/lib/api';
import type { WorkflowDefinition, WorkflowRun } from '@/lib/api';
import { formatDistanceToNow } from 'date-fns';
import { WorkflowEditor, getNodeTypeDef } from '@/components/workflows/WorkflowEditor';
import { WorkflowTriggersPanel } from '@/components/workflows/WorkflowTriggersPanel';
import { WorkflowRunsPanel } from '@/components/workflows/WorkflowRunsPanel';
import { WorkflowDetailPanel } from '../components/WorkflowDetailPanel';
import { RunWorkflowDialog } from '../components/RunWorkflowDialog';

export function WorkflowBuilderTab() {
  const queryClient = useQueryClient();
  const { data: workflows = [], isLoading } = useQuery({
    queryKey: ['workflowDefinitions'],
    queryFn: () => workflowsApi.listDefinitions(),
  });

  // Fetch recent runs across all workflows to show last-run status
  const { data: recentRuns = [] } = useQuery({
    queryKey: ['workflow-runs-builder'],
    queryFn: () => workflowsApi.listRecentRuns({ limit: 50 }),
    staleTime: 30_000,
  });

  // Index last run by workflow_id for quick lookup
  const lastRunByWorkflow = useMemo(() => {
    const map = new Map<string, WorkflowRun>();
    for (const run of recentRuns) {
      if (!map.has(run.workflow_id)) {
        map.set(run.workflow_id, run);
      }
    }
    return map;
  }, [recentRuns]);

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingWorkflow, setEditingWorkflow] = useState<WorkflowDefinition | null>(null);
  const [triggersOpen, setTriggersOpen] = useState(false);
  const [triggersWorkflow, setTriggersWorkflow] = useState<WorkflowDefinition | null>(null);
  const [runsOpen, setRunsOpen] = useState(false);
  const [runsWorkflow, setRunsWorkflow] = useState<WorkflowDefinition | null>(null);
  const [runWorkflow, setRunWorkflow] = useState<WorkflowDefinition | null>(null);
  const [detailWorkflow, setDetailWorkflow] = useState<WorkflowDefinition | null>(null);

  const saveMutation = useMutation({
    mutationFn: async (data: { id: string; name: string; description?: string; nodes: any[]; connections: any[] }) => {
      if (editingWorkflow) {
        return workflowsApi.updateDefinition(data.id, {
          name: data.name,
          description: data.description,
          nodes: data.nodes,
          connections: data.connections,
        });
      } else {
        return workflowsApi.createDefinition(data);
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workflowDefinitions'] });
      setEditorOpen(false);
      setEditingWorkflow(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => workflowsApi.deleteDefinition(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['workflowDefinitions'] });
      setDetailWorkflow(null);
    },
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading workflows...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4 max-w-[1200px] mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Workflow Builder</h2>
          <p className="text-sm text-muted-foreground">
            Create and edit data processing pipelines with the n8n-style node editor.
          </p>
        </div>
        <Button size="sm" variant="outline" className="gap-1.5" onClick={() => { setEditingWorkflow(null); setEditorOpen(true); }}>
          <Plus className="h-3.5 w-3.5" />
          New Workflow
        </Button>
      </div>

      {workflows.length === 0 ? (
        <EmptyState
          icon={Hammer}
          title="No workflows yet"
          description="Create your first workflow to start processing data sources."
          action={{ label: "Create Workflow", onClick: () => { setEditingWorkflow(null); setEditorOpen(true); } }}
        />
      ) : (
        <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
          {workflows.map((wf: WorkflowDefinition) => {
            const nodeCount = wf.nodes?.length ?? 0;
            const isSelected = detailWorkflow?.id === wf.id;
            return (
              <Card
                key={wf.id}
                className={`card-interactive cursor-pointer transition-all ${isSelected ? 'ring-2 ring-primary border-primary' : ''}`}
                onClick={() => setDetailWorkflow(isSelected ? null : wf)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm">{wf.name}</CardTitle>
                    <div className="flex items-center gap-1.5">
                      {wf.is_system && <Badge variant="secondary" className="text-[10px]">System</Badge>}
                      <Badge variant="outline" className="text-[10px]">{nodeCount} node{nodeCount !== 1 ? 's' : ''}</Badge>
                    </div>
                  </div>
                  {wf.description && <CardDescription className="text-xs line-clamp-2">{wf.description}</CardDescription>}
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="flex flex-wrap gap-1">
                      {(wf.nodes ?? []).slice(0, 4).map((node: any) => {
                        const nDef = getNodeTypeDef(node.type);
                        const colorMap: Record<string, string> = {
                          'bg-blue-500': 'bg-blue-500/10 text-blue-700 border-blue-500/20',
                          'bg-purple-500': 'bg-purple-500/10 text-purple-700 border-purple-500/20',
                          'bg-emerald-500': 'bg-emerald-500/10 text-emerald-700 border-emerald-500/20',
                          'bg-amber-500': 'bg-amber-500/10 text-amber-700 border-amber-500/20',
                          'bg-orange-500': 'bg-orange-500/10 text-orange-700 border-orange-500/20',
                          'bg-teal-500': 'bg-teal-500/10 text-teal-700 border-teal-500/20',
                        };
                        const badgeColor = colorMap[nDef?.color ?? ''] ?? 'bg-muted text-muted-foreground';
                        return (
                          <Badge key={node.id} variant="outline" className={`text-[9px] px-1.5 ${badgeColor}`}>{node.name}</Badge>
                        );
                      })}
                      {nodeCount > 4 && <Badge variant="outline" className="text-[9px] px-1.5">+{nodeCount - 4}</Badge>}
                    </div>
                    <div className="flex items-center gap-1">
                      <button
                        className="p-1 rounded hover:bg-primary/10 hover:text-primary transition-colors"
                        title="Run workflow"
                        onClick={(e) => {
                          e.stopPropagation();
                          setRunWorkflow(wf);
                        }}
                      >
                        <Play className="h-3.5 w-3.5" />
                      </button>
                      <button
                        className="p-1 rounded hover:bg-blue-500/10 hover:text-blue-600 transition-colors"
                        title="Edit in visual builder"
                        onClick={(e) => {
                          e.stopPropagation();
                          setEditingWorkflow(wf);
                          setEditorOpen(true);
                        }}
                      >
                        <Pencil className="h-3.5 w-3.5" />
                      </button>
                      {!wf.is_system && (
                        <button
                          className="p-1 rounded hover:bg-destructive/10 hover:text-destructive transition-colors"
                          onClick={(e) => {
                            e.stopPropagation();
                            if (confirm(`Delete "${wf.name}"?`)) deleteMutation.mutate(wf.id);
                          }}
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </button>
                      )}
                    </div>
                  </div>
                  {(() => {
                    const lastRun = lastRunByWorkflow.get(wf.id);
                    if (!lastRun) return <p className="text-[10px] text-muted-foreground/50 mt-1.5">Never run</p>;
                    return (
                      <div className="text-[10px] text-muted-foreground mt-1.5 flex items-center gap-1">
                        Last run: {formatDistanceToNow(new Date(lastRun.created_at), { addSuffix: true })}
                        {lastRun.status === 'completed' && <CheckCircle2 className="h-3 w-3 text-green-500" />}
                        {lastRun.status === 'failed' && <AlertCircle className="h-3 w-3 text-red-500" />}
                        {lastRun.status === 'running' && <Loader2Icon className="h-3 w-3 text-blue-500 animate-spin" />}
                      </div>
                    );
                  })()}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {/* Workflow Detail Panel -- shown inline below the grid when a workflow is selected */}
      {detailWorkflow && (
        <WorkflowDetailPanel
          workflow={detailWorkflow}
          onEdit={() => { setEditingWorkflow(detailWorkflow); setEditorOpen(true); }}
          onRun={() => setRunWorkflow(detailWorkflow)}
          onViewTriggers={() => { setTriggersWorkflow(detailWorkflow); setTriggersOpen(true); }}
          onClose={() => setDetailWorkflow(null)}
        />
      )}

      <WorkflowEditor
        open={editorOpen}
        onOpenChange={(v) => { setEditorOpen(v); if (!v) setEditingWorkflow(null); }}
        workflow={editingWorkflow}
        onSave={(data) => saveMutation.mutate(data)}
        isSaving={saveMutation.isPending}
      />

      <WorkflowTriggersPanel
        open={triggersOpen}
        onOpenChange={(v) => { setTriggersOpen(v); if (!v) setTriggersWorkflow(null); }}
        workflowId={triggersWorkflow?.id ?? ''}
        workflowName={triggersWorkflow?.name}
      />

      <WorkflowRunsPanel
        open={runsOpen}
        onOpenChange={(v) => { setRunsOpen(v); if (!v) setRunsWorkflow(null); }}
        workflowId={runsWorkflow?.id}
      />

      <RunWorkflowDialog
        workflow={runWorkflow}
        onClose={() => setRunWorkflow(null)}
      />
    </div>
  );
}
