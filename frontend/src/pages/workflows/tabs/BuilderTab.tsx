// ─── WorkflowBuilderTab ───────────────────────────────────────────────────────
//
// Builder tab: create, edit, and manage workflow definitions using the n8n-style editor.

import { useState, useMemo } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import { Plus } from 'lucide-react';
import { workflowsApi } from '@/lib/api';
import type { WorkflowDefinition, WorkflowRun } from '@/lib/api';
import { workflowKeys } from '@/lib/query-keys';
import { WorkflowEditor } from '@/components/workflows/WorkflowEditor';
import { WorkflowTriggersPanel } from '@/components/workflows/WorkflowTriggersPanel';
import { WorkflowRunsPanel } from '@/components/workflows/WorkflowRunsPanel';
import { WorkflowCardGrid } from '@/components/workflows/WorkflowCardGrid';
import { WorkflowDetailPanel } from '../components/WorkflowDetailPanel';
import { RunWorkflowDialog } from '../components/RunWorkflowDialog';
import { RunAndReviewPanel } from '@/components/workflows/RunAndReviewPanel';
import { CopyWorkflowDialog } from '@/components/workflows/CopyWorkflowDialog';

export function WorkflowBuilderTab() {
  const queryClient = useQueryClient();
  const { data: workflows = [], isLoading } = useQuery({
    queryKey: workflowKeys.definitions(),
    queryFn: () => workflowsApi.listDefinitions(),
  });

  // Fetch recent runs across all workflows to show last-run status
  const { data: recentRuns = [] } = useQuery({
    queryKey: workflowKeys.runsBuilder(),
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
  const [inlineReview, setInlineReview] = useState<{ runId: string; stagedRecords: number; workflowName?: string } | null>(null);
  const [copyWorkflow, setCopyWorkflow] = useState<{ wf: WorkflowDefinition; direction: 'to-org' | 'to-user' } | null>(null);

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
      queryClient.invalidateQueries({ queryKey: workflowKeys.definitions() });
      setEditorOpen(false);
      setEditingWorkflow(null);
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => workflowsApi.deleteDefinition(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: workflowKeys.definitions() });
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

      <WorkflowCardGrid
        workflows={workflows}
        lastRunByWorkflow={lastRunByWorkflow}
        selectedId={detailWorkflow?.id}
        showOwnerBadge
        onSelect={(wf) => setDetailWorkflow(detailWorkflow?.id === wf.id ? null : wf)}
        onEdit={(wf) => { setEditingWorkflow(wf); setEditorOpen(true); }}
        onRun={(wf) => setRunWorkflow(wf)}
        onDelete={(wf) => deleteMutation.mutate(wf.id)}
        onCopyToOrg={(wf) => setCopyWorkflow({ wf, direction: 'to-org' })}
        onCopyToUser={(wf) => setCopyWorkflow({ wf, direction: 'to-user' })}
        onCreateNew={() => { setEditingWorkflow(null); setEditorOpen(true); }}
      />

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

      {/* Inline staging review -- shown after a successful run */}
      {inlineReview && (
        <RunAndReviewPanel
          runId={inlineReview.runId}
          stagedRecords={inlineReview.stagedRecords}
          workflowName={inlineReview.workflowName}
          onRunAnother={() => {
            setInlineReview(null);
            setRunWorkflow(runWorkflow);
          }}
          onClose={() => setInlineReview(null)}
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
        onRunComplete={(runId, stagedRecords) => {
          setInlineReview({ runId, stagedRecords, workflowName: runWorkflow?.name });
        }}
      />

      <CopyWorkflowDialog
        workflow={copyWorkflow?.wf ?? null}
        direction={copyWorkflow?.direction ?? 'to-org'}
        onClose={() => setCopyWorkflow(null)}
      />
    </div>
  );
}
