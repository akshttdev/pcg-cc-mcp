import { useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  GitBranch,
  Plus,
  Activity,
  ExternalLink,
} from 'lucide-react';
import {
  workflowsApi,
  resolveApiUrl,
  automationsApi,
} from '@/lib/api';
import type { WorkflowDefinition, WorkflowNode, WorkflowConnection } from '@/lib/api';
import { workflowKeys, workflowTemplateKeys } from '@/lib/query-keys';
import { WorkflowEditor as WorkflowEditorComponent } from '@/components/workflows/WorkflowEditor';
import { WorkflowCardGrid } from '@/components/workflows/WorkflowCardGrid';
import { RunWorkflowDialog } from '@/pages/workflows/components/RunWorkflowDialog';

// ── Automation & Template Types ───────────────────────────────────────────────

interface SystemAutomation {
  id: string;
  name: string;
  description?: string;
  schedule?: string;
}

interface TemplateTask {
  title: string;
  task_type?: string;
  agent_role?: string;
  requires_approval?: boolean;
  tags?: string[];
}

interface TemplatePhase {
  name: string;
  description?: string;
  is_recurring?: boolean;
  tasks: TemplateTask[];
}

interface WorkflowTemplate {
  id: string;
  name: string;
  description?: string;
  client_type?: string;
  phases?: TemplatePhase[];
}

// ── Pipeline Types & Constants ───────────────────────────────────────────────

export interface PipelineNode {
  id: string;
  label: string;
  agent?: string;
  type: 'agent' | 'human' | 'parallel' | 'tool';
  description: string;
  tools?: string[];
  parallel?: boolean;
}

export interface PipelineBlueprint {
  id: string;
  name: string;
  description: string;
  category: string;
  color: string;
  nodes: PipelineNode[];
}

const CONFERENCE_PIPELINE: PipelineBlueprint = {
  id: 'conference_research',
  name: 'Conference Research',
  description: 'Full pipeline: conference intel -> speaker/brand/side-event research (parallel) -> article writing -> QA -> social publishing.',
  category: 'Research',
  color: 'blue',
  nodes: [
    { id: 'conf_intel', label: 'Conference Intel', agent: 'Scout', type: 'agent', description: 'Research event, venue, organizers, agenda, and key themes.' },
    { id: 'speaker_res', label: 'Speaker Research', agent: 'Scout', type: 'parallel', description: 'Profile each speaker in parallel -- bio, publications, LinkedIn, social presence.', parallel: true },
    { id: 'brand_res', label: 'Brand Research', agent: 'Scout', type: 'parallel', description: 'Profile sponsors and brands in parallel -- positioning, news, key contacts.', parallel: true },
    { id: 'prod_team', label: 'Production Team', agent: 'Scout', type: 'agent', description: 'Identify AV production companies, photographers, and crew.' },
    { id: 'comp_intel', label: 'Competitive Intel', agent: 'Scout', type: 'agent', description: 'Analyze competing events, positioning, and attendee overlap.' },
    { id: 'side_events', label: 'Side Events Discovery', agent: 'Scout', type: 'parallel', description: 'Discover Lu.ma, Eventbrite, and Partiful side events in parallel.', parallel: true },
    { id: 'articles', label: 'Article Writing', agent: 'Astra', type: 'agent', description: 'Write thought-leadership articles per speaker using research context.' },
    { id: 'qa', label: 'QA Review', agent: 'Astra', type: 'agent', description: 'Quality-check all content for accuracy, tone, and brand alignment.' },
    { id: 'social', label: 'Social Publishing', agent: 'Creative', type: 'agent', description: 'Schedule and publish posts across connected social accounts.' },
  ],
};

const EDITRON_PIPELINE: PipelineBlueprint = {
  id: 'editron',
  name: 'Editron Production',
  description: 'Video production pipeline: intake -> scene detection (Maci) -> audio/music (Sonix) -> colour -> assembly -> review -> export.',
  category: 'Production',
  color: 'amber',
  nodes: [
    { id: 'intake', label: 'Intake & Indexing', agent: 'Nora', type: 'agent', description: 'Receive footage from Nora task, generate proxy files for fast editing.', tools: ['FFmpeg', 'Proxy Manager'] },
    { id: 'scene', label: 'Scene Detection', agent: 'Maci', type: 'agent', description: 'Shot selection via visual QC -- detect scenes, label content, rank clips by quality.', tools: ['Maci', 'Visual QC'] },
    { id: 'music', label: 'Music & Sound', agent: 'Sonix', type: 'tool', description: 'Audio engineering: music recommendations, loudness normalization, compression. Libraries: Artlist, Epidemic Sound, Soundstripe.', tools: ['Sonix', 'Artlist', 'Epidemic Sound', 'Soundstripe'] },
    { id: 'color', label: 'Colour Grading', agent: 'Editron', type: 'tool', description: 'Apply LUT and colour grade presets matched to project brand guide.', tools: ['Colour Engine', 'LUTs'] },
    { id: 'assembly', label: 'Edit Assembly', agent: 'Editron', type: 'agent', description: 'Assemble timeline -- clips, transitions, music sync, markers. Output Premiere .prproj.', tools: ['Edit Assembly', 'Premiere Bridge'] },
    { id: 'review', label: 'Human Review', agent: undefined, type: 'human', description: 'Creative director reviews cut, provides revision notes.' },
    { id: 'export', label: 'Export & Deliver', agent: 'Editron', type: 'tool', description: 'Final encode via Media Encoder or FFmpeg. Deliver to client asset folder.', tools: ['Media Encoder', 'FFmpeg'] },
  ],
};

export const STATIC_PIPELINES = [CONFERENCE_PIPELINE, EDITRON_PIPELINE];

export const AGENT_COLORS: Record<string, string> = {
  Scout: 'bg-blue-500/15 border-blue-500/40 text-blue-400',
  Astra: 'bg-purple-500/15 border-purple-500/40 text-purple-400',
  Creative: 'bg-pink-500/15 border-pink-500/40 text-pink-400',
  Maci: 'bg-orange-500/15 border-orange-500/40 text-orange-400',
  Sonix: 'bg-green-500/15 border-green-500/40 text-green-400',
  Nora: 'bg-indigo-500/15 border-indigo-500/40 text-indigo-400',
  Editron: 'bg-amber-500/15 border-amber-500/40 text-amber-400',
  human: 'bg-emerald-500/15 border-emerald-500/40 text-emerald-400',
};

// ── Pipeline Node Card ───────────────────────────────────────────────────────

export function PipelineNodeCard({ node, isLast }: { node: PipelineNode; isLast: boolean }) {
  const agentKey = node.type === 'human' ? 'human' : (node.agent ?? '');
  const colorClass = AGENT_COLORS[agentKey] ?? 'bg-muted/50 border-border text-muted-foreground';

  return (
    <div className="flex items-start gap-0">
      <div className={`relative border rounded-lg p-3 w-44 shrink-0 ${colorClass}`}>
        {node.parallel && (
          <div className="absolute -top-2 right-2">
            <Badge variant="outline" className="text-[10px] px-1 py-0">parallel</Badge>
          </div>
        )}
        <div className="font-medium text-sm leading-tight mb-1">{node.label}</div>
        {node.agent && (
          <div className="text-[10px] opacity-70 mb-1.5">{node.agent}</div>
        )}
        {node.type === 'human' && (
          <div className="text-[10px] opacity-70 mb-1.5">Human Gate</div>
        )}
        <div className="text-[10px] opacity-60 leading-snug line-clamp-3">{node.description}</div>
        {node.tools && node.tools.length > 0 && (
          <div className="flex flex-wrap gap-1 mt-2">
            {node.tools.map((t) => (
              <span key={t} className="text-[9px] bg-black/20 rounded px-1 py-0.5">{t}</span>
            ))}
          </div>
        )}
      </div>
      {!isLast && (
        <div className="flex items-center self-center shrink-0 px-1">
          <div className="w-6 h-px bg-border" />
          <svg width="8" height="8" viewBox="0 0 8 8" className="text-muted-foreground shrink-0">
            <path d="M0 4h6M3 1l3 3-3 3" stroke="currentColor" strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
          </svg>
        </div>
      )}
    </div>
  );
}

// ── Editable Workflows View ──────────────────────────────────────────────────

export function EditableWorkflowsView({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const { data: workflows = [], isLoading } = useQuery({
    queryKey: workflowKeys.definitions(),
    queryFn: () => workflowsApi.listDefinitions(),
  });

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingWorkflow, setEditingWorkflow] = useState<WorkflowDefinition | null>(null);
  const [runWorkflow, setRunWorkflow] = useState<WorkflowDefinition | null>(null);

  const saveMutation = useMutation({
    mutationFn: async (data: { id: string; name: string; description?: string; nodes: WorkflowNode[]; connections: WorkflowConnection[] }) => {
      if (editingWorkflow) {
        return workflowsApi.updateDefinition(data.id, {
          name: data.name,
          description: data.description,
          nodes: data.nodes,
          connections: data.connections,
        });
      } else {
        return workflowsApi.createDefinition({
          ...data,
          owner_type: 'organization',
          owner_id: orgId,
        });
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
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <GitBranch className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Workflows</h2>
          <Badge variant="secondary">{workflows.length}</Badge>
        </div>
        <Button size="sm" variant="outline" className="gap-1.5" onClick={() => { setEditingWorkflow(null); setEditorOpen(true); }}>
          <Plus className="h-3.5 w-3.5" />
          New Workflow
        </Button>
      </div>

      <p className="text-sm text-muted-foreground">
        Workflows are data processing pipelines that extract structured information from data sources.
        Run them from any data source detail page.
      </p>

      <WorkflowCardGrid
        workflows={workflows}
        showOwnerBadge
        onSelect={(wf) => { setEditingWorkflow(wf); setEditorOpen(true); }}
        onEdit={(wf) => { setEditingWorkflow(wf); setEditorOpen(true); }}
        onRun={(wf) => setRunWorkflow(wf)}
        onDelete={(wf) => deleteMutation.mutate(wf.id)}
        onCreateNew={() => { setEditingWorkflow(null); setEditorOpen(true); }}
        emptyIcon={GitBranch}
        emptyTitle="No workflows defined yet"
        emptyDescription="Create your first workflow to start processing data sources."
      />

      <WorkflowEditorComponent
        open={editorOpen}
        onOpenChange={(v) => { setEditorOpen(v); if (!v) setEditingWorkflow(null); }}
        workflow={editingWorkflow}
        onSave={(data) => saveMutation.mutate(data)}
        isSaving={saveMutation.isPending}
      />

      <RunWorkflowDialog
        workflow={runWorkflow}
        onClose={() => setRunWorkflow(null)}
      />
    </div>
  );
}

// ── Pipeline View ────────────────────────────────────────────────────────────

export function PipelineView({ pipeline }: { pipeline: PipelineBlueprint }) {
  return (
    <div className="space-y-3">
      <div>
        <p className="text-sm text-muted-foreground">{pipeline.description}</p>
        <div className="flex gap-3 mt-2 flex-wrap text-xs text-muted-foreground">
          {['Scout', 'Astra', 'Maci', 'Sonix', 'Editron', 'Creative', 'Nora'].map((agent) => {
            if (!pipeline.nodes.some((n) => n.agent === agent)) return null;
            const c = AGENT_COLORS[agent] ?? '';
            return (
              <span key={agent} className={`inline-flex items-center gap-1 border rounded px-1.5 py-0.5 ${c}`}>
                <span className="w-1.5 h-1.5 rounded-full bg-current opacity-60" />
                {agent}
              </span>
            );
          })}
          {pipeline.nodes.some((n) => n.type === 'human') && (
            <span className={`inline-flex items-center gap-1 border rounded px-1.5 py-0.5 ${AGENT_COLORS.human}`}>
              <span className="w-1.5 h-1.5 rounded-full bg-current opacity-60" />
              Human Gate
            </span>
          )}
        </div>
      </div>
      <div className="overflow-x-auto pb-2">
        <div className="flex items-start gap-0 min-w-max">
          {pipeline.nodes.map((node, i) => (
            <PipelineNodeCard key={node.id} node={node} isLast={i === pipeline.nodes.length - 1} />
          ))}
        </div>
      </div>
    </div>
  );
}

// ── Template Pipeline View ───────────────────────────────────────────────────

export function TemplatePipelineView({ template }: { template: WorkflowTemplate }) {
  return (
    <div className="space-y-4">
      <p className="text-sm text-muted-foreground">{template.description}</p>
      <div className="space-y-6">
        {(template.phases ?? []).map((phase: TemplatePhase, pi: number) => (
          <div key={phase.name}>
            <div className="flex items-center gap-2 mb-3">
              <div className="w-6 h-6 rounded-full bg-muted flex items-center justify-center text-xs font-medium shrink-0">{pi + 1}</div>
              <div>
                <div className="font-medium text-sm">{phase.name}</div>
                <div className="text-xs text-muted-foreground">{phase.description}</div>
              </div>
              {phase.is_recurring && <Badge variant="outline" className="text-xs ml-auto">Recurring</Badge>}
            </div>
            <div className="overflow-x-auto pb-1 pl-8">
              <div className="flex items-start gap-0 min-w-max">
                {(phase.tasks ?? []).map((task: TemplateTask, ti: number) => {
                  const agentKey = task.task_type === 'human_review' ? 'human' : (task.agent_role ?? '');
                  const colorClass = AGENT_COLORS[agentKey] ?? (task.task_type === 'human_review' ? AGENT_COLORS.human : 'bg-muted/50 border-border text-muted-foreground');
                  const isLast = ti === phase.tasks.length - 1;
                  return (
                    <div key={task.title} className="flex items-start gap-0">
                      <div className={`border rounded-lg p-2.5 w-40 shrink-0 ${colorClass}`}>
                        {task.requires_approval && (
                          <div className="text-[9px] mb-1 opacity-60">Gate</div>
                        )}
                        <div className="font-medium text-xs leading-tight mb-1">{task.title}</div>
                        {task.agent_role && (
                          <div className="text-[10px] opacity-60 capitalize">{task.agent_role}</div>
                        )}
                        {task.task_type === 'human_review' && (
                          <div className="text-[10px] opacity-60">Human Review</div>
                        )}
                        {(task.tags?.length ?? 0) > 0 && (
                          <div className="flex flex-wrap gap-0.5 mt-1.5">
                            {task.tags!.slice(0, 2).map((t: string) => (
                              <span key={t} className="text-[9px] bg-black/20 rounded px-1">{t}</span>
                            ))}
                          </div>
                        )}
                      </div>
                      {!isLast && (
                        <div className="flex items-center self-center shrink-0 px-1">
                          <div className="w-5 h-px bg-border" />
                          <svg width="8" height="8" viewBox="0 0 8 8" className="text-muted-foreground shrink-0">
                            <path d="M0 4h6M3 1l3 3-3 3" stroke="currentColor" strokeWidth="1.5" fill="none" strokeLinecap="round" strokeLinejoin="round"/>
                          </svg>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

// ── Legacy Pipelines View ────────────────────────────────────────────────────

export function LegacyPipelinesView({ orgId: _orgId }: { orgId: string }) {
  const [selected, setSelected] = useState<string | null>(null);

  const { data: templates = [] } = useQuery({
    queryKey: workflowTemplateKeys.all(),
    queryFn: () =>
      fetch(resolveApiUrl('/api/workflow-templates'), { credentials: 'include' })
        .then((r) => r.json())
        .then((res) => res?.data ?? []),
  });

  const allPipelines: Array<{ id: string; name: string; category: string; source: 'template' | 'static'; data: WorkflowTemplate | PipelineBlueprint }> = [
    ...(templates as WorkflowTemplate[]).map((t: WorkflowTemplate) => ({
      id: t.id,
      name: t.name,
      category: t.client_type === 'foundation_build' ? 'Client Engagement' : 'Client Engagement',
      source: 'template' as const,
      data: t,
    })),
    ...STATIC_PIPELINES.map((p) => ({
      id: p.id,
      name: p.name,
      category: p.category,
      source: 'static' as const,
      data: p,
    })),
  ];

  const selectedPipeline = allPipelines.find((p) => p.id === selected) ?? allPipelines[0] ?? null;

  return (
    <div className="flex gap-4 h-full min-h-[500px]">
      {/* Sidebar */}
      <div className="w-52 shrink-0 space-y-1">
        <div className="text-xs font-medium text-muted-foreground uppercase tracking-wide mb-2 px-1">Workflows</div>
        {allPipelines.map((p) => (
          <button
            key={p.id}
            onClick={() => setSelected(p.id)}
            className={`w-full text-left px-3 py-2 rounded-lg text-sm transition-colors ${
              (selectedPipeline?.id === p.id)
                ? 'bg-accent text-accent-foreground'
                : 'text-muted-foreground hover:bg-muted/50 hover:text-foreground'
            }`}
          >
            <div className="font-medium leading-tight">{p.name}</div>
            <div className="text-[10px] opacity-60 mt-0.5">{p.category}</div>
          </button>
        ))}
      </div>

      {/* Pipeline canvas */}
      <div className="flex-1 min-w-0 border border-border/50 rounded-xl bg-card/40 p-4 overflow-auto">
        {selectedPipeline ? (
          <div className="space-y-4">
            <div className="flex items-center gap-3">
              <GitBranch className="h-5 w-5 text-muted-foreground shrink-0" />
              <div>
                <h3 className="font-semibold">{selectedPipeline.name}</h3>
                <div className="text-xs text-muted-foreground">{selectedPipeline.category}</div>
              </div>
            </div>
            {selectedPipeline.source === 'static'
              ? <PipelineView pipeline={selectedPipeline.data as PipelineBlueprint} />
              : <TemplatePipelineView template={selectedPipeline.data as WorkflowTemplate} />
            }
          </div>
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            <div className="text-center">
              <GitBranch className="h-8 w-8 mx-auto mb-2 opacity-40" />
              <p className="text-sm">Select a workflow to view its pipeline</p>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ── System Automations Section ───────────────────────────────────────────────

export function SystemAutomationsSection() {
  const { data: automations = [] } = useQuery({
    queryKey: workflowKeys.systemAutomations(),
    queryFn: () => automationsApi.list(),
    staleTime: 5 * 60_000,
  });

  if (automations.length === 0) return null;

  return (
    <div className="border-t pt-6">
      <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">
        System Automations ({automations.length} active)
      </p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {(automations as SystemAutomation[]).map((a) => (
          <Card key={a.id} className="bg-card/80 border-border/50">
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <Activity className="h-4 w-4 text-green-500 shrink-0 mt-0.5" />
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{a.name}</p>
                  {a.description && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{a.description}</p>}
                  <div className="flex items-center gap-1.5 mt-1">
                    <Badge variant="default" className="text-[9px] bg-green-500/10 text-green-700 border-green-200">Active</Badge>
                    {a.schedule && <span className="text-[9px] text-muted-foreground">{a.schedule}</span>}
                  </div>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}

// ── Workflows Intel View ─────────────────────────────────────────────────────

export function WorkflowsIntelView({ orgId }: { orgId: string }) {
  const { data: automations = [] } = useQuery({
    queryKey: workflowKeys.systemAutomations(),
    queryFn: () => automationsApi.list(),
    staleTime: 5 * 60_000,
  });

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-muted-foreground">Workflows & Automations</h3>
        <Link
          to="/workflows"
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
        >
          <GitBranch className="h-3.5 w-3.5" />
          Live Workflow Monitor
          <ExternalLink className="h-3 w-3" />
        </Link>
      </div>

      {/* System Automations */}
      {automations.length > 0 && (
        <div>
          <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-2">System Automations ({automations.length} active)</p>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
            {(automations as SystemAutomation[]).map((a) => (
              <Card key={a.id} className="bg-card/80 border-border/50">
                <CardContent className="pt-4 pb-4">
                  <div className="flex items-start gap-2">
                    <Activity className="h-4 w-4 text-green-500 shrink-0 mt-0.5" />
                    <div className="min-w-0">
                      <p className="text-sm font-medium truncate">{a.name}</p>
                      {a.description && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{a.description}</p>}
                      <div className="flex items-center gap-1.5 mt-1">
                        <Badge variant="default" className="text-[9px] bg-green-500/10 text-green-700 border-green-200">Active</Badge>
                        {a.schedule && <span className="text-[9px] text-muted-foreground">{a.schedule}</span>}
                      </div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      )}

      {/* Pipeline Blueprints -- Conference, Editron, and API templates */}
      <div>
        <p className="text-xs font-medium text-muted-foreground uppercase tracking-wider mb-3">Pipeline Blueprints</p>
        <LegacyPipelinesView orgId={orgId} />
      </div>
    </div>
  );
}
