import { useState, useEffect, useMemo } from 'react';
import { useExecutionEvents, ActiveExecution } from '@/hooks/useExecutionEvents';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Input } from '@/components/ui/input';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import {
  GitBranch,
  CheckCircle2,
  AlertCircle,
  Play,
  WifiOff,
  Clock,
  Zap,
  History,
  Microscope,
  Film,
  Calendar,
  Hammer,
  Plus,
  Trash2,
  ClipboardCheck,
  Activity,
  Database,
  Search,
  Loader2,
  FileText,
  Upload,
} from 'lucide-react';
import { agentFlowsApi, wideResearchApi, workflowsApi, dataSourcesApi, stagingApi, resolveApiUrl, DATA_TYPE_OPTIONS } from '@/lib/api';
import type { AgentFlow, WideResearchSession, WorkflowDefinition, WorkflowStagingRecord } from '@/lib/api';
import { WorkflowEditor, getNodeTypeDef } from '@/components/workflows/WorkflowEditor';
import { StagingReviewPanel } from '@/components/workflows/StagingReviewPanel';

interface AutomationDefinition {
  id: string;
  name: string;
  description: string;
  trigger: string;
  action: string;
  schedule: string;
}

interface ConferenceWorkflow {
  id: string;
  conferenceName: string;
  status: string;
  startDate: string;
  endDate: string;
  location: string | null;
  createdAt: string;
}

interface CinematicBrief {
  id: string;
  project_id: string;
  title: string;
  status: string;
  created_at: string;
}

export function WorkflowsPage() {
  const [selectedExecution, setSelectedExecution] = useState<ActiveExecution | null>(null);
  const [activeTab, setActiveTab] = useState('builder');

  const [agentFlows, setAgentFlows] = useState<AgentFlow[]>([]);
  const [automations, setAutomations] = useState<AutomationDefinition[]>([]);
  const [conferences, setConferences] = useState<ConferenceWorkflow[]>([]);
  const [researchSessions, setResearchSessions] = useState<WideResearchSession[]>([]);
  const [cinematicBriefs, setCinematicBriefs] = useState<CinematicBrief[]>([]);

  const {
    activeExecutions,
    completedExecutions,
    connected,
    connectionMode,
  } = useExecutionEvents({ maxHistory: 50 });

  useEffect(() => {
    agentFlowsApi.list().then((flows) => {
      if (Array.isArray(flows)) setAgentFlows(flows);
    }).catch(() => {});

    wideResearchApi.list().then((sessions) => {
      if (Array.isArray(sessions)) setResearchSessions(sessions);
    }).catch(() => {});

    fetch(resolveApiUrl('/api/automations'), { credentials: 'include' })
      .then((r) => r.json())
      .then((res) => { if (res?.data) setAutomations(res.data); })
      .catch(() => {});

    fetch(resolveApiUrl('/api/nora/workflows'), { credentials: 'include' })
      .then((r) => r.json())
      .then((data: ConferenceWorkflow[]) => { if (Array.isArray(data)) setConferences(data); })
      .catch(() => {});

    fetch(resolveApiUrl('/api/nora/cinematics/briefs'), { credentials: 'include' })
      .then((r) => r.json())
      .then((res) => { if (Array.isArray(res?.data ?? res)) setCinematicBriefs(res?.data ?? res); })
      .catch(() => {});
  }, []);

  const stats = {
    active: activeExecutions.length,
    completed: completedExecutions.filter((e) => e.status === 'completed').length,
    failed: completedExecutions.filter((e) => e.status === 'failed').length,
  };

  const statusColor = (status: string) => {
    if (status === 'completed' || status === 'done' || status === 'ready') return 'border-l-green-500';
    if (status === 'failed' || status === 'error') return 'border-l-red-500';
    if (status === 'running' || status === 'active' || status === 'processing') return 'border-l-yellow-500';
    return 'border-l-blue-500';
  };

  const statusBadge = (status: string) => {
    if (status === 'completed' || status === 'done' || status === 'ready') return 'default';
    if (status === 'failed' || status === 'error') return 'destructive';
    return 'outline';
  };

  const formatDuration = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  };

  const formatTime = (ts?: string) => {
    if (!ts) return '-';
    return new Date(ts).toLocaleTimeString();
  };

  const formatDate = (ts?: string) => {
    if (!ts) return '-';
    return new Date(ts).toLocaleDateString(undefined, { month: 'short', day: 'numeric', year: 'numeric' });
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="relative border-b border-border/40 bg-card/50 backdrop-blur-sm overflow-hidden">
        <div className="ambient-glow -top-48 -right-32" />
        <div className="page-header px-4 sm:px-6 lg:px-8 py-4 max-w-[1600px] mx-auto relative">
          <div className="flex items-center gap-3">
            <div className="section-header-icon">
              <GitBranch className="h-5 w-5" />
            </div>
            <div>
              <h1 className="page-title">Workflows</h1>
              <p className="page-description">
                All platform workflow systems — live executions, agent flows, research, conference, and Editron pipelines
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2 sm:gap-3 flex-wrap">
            {connected ? (
              <Badge variant="success" className="gap-1.5">
                <div className="status-dot status-dot-online" />
                Live ({connectionMode})
              </Badge>
            ) : (
              <Badge variant="destructive" className="gap-1.5">
                <WifiOff className="h-3 w-3" />
                Disconnected
              </Badge>
            )}
            <div className="flex items-center gap-1.5">
              <Badge variant="warning" className="gap-1">
                <Play className="h-3 w-3" />
                {stats.active} Active
              </Badge>
              <Badge variant="success" className="gap-1">
                <CheckCircle2 className="h-3 w-3" />
                {stats.completed}
              </Badge>
              {stats.failed > 0 && (
                <Badge variant="destructive" className="gap-1">
                  <AlertCircle className="h-3 w-3" />
                  {stats.failed}
                </Badge>
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-hidden">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
          <div className="border-b px-4 sm:px-6">
            <TabsList className="tab-grid-10">
              <TabsTrigger value="builder">
                <Hammer className="h-3.5 w-3.5 mr-1" />
                Builder
              </TabsTrigger>
              <TabsTrigger value="staging">
                <ClipboardCheck className="h-3.5 w-3.5 mr-1" />
                Staging
              </TabsTrigger>
              <TabsTrigger value="runs">
                <Activity className="h-3.5 w-3.5 mr-1" />
                Runs
              </TabsTrigger>
              <TabsTrigger value="active">
                Active
                {activeExecutions.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 animate-pulse text-xs">
                    {activeExecutions.length}
                  </Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="recent">
                Recent
                {completedExecutions.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 text-xs">{completedExecutions.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="flows">
                <History className="h-3.5 w-3.5 mr-1" />
                Agent Flows
                {agentFlows.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 text-xs">{agentFlows.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="research">
                <Microscope className="h-3.5 w-3.5 mr-1" />
                Research
                {researchSessions.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 text-xs">{researchSessions.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="conference">
                <Calendar className="h-3.5 w-3.5 mr-1" />
                Conference
                {conferences.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 text-xs">{conferences.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="editron">
                <Film className="h-3.5 w-3.5 mr-1" />
                Editron
                {cinematicBriefs.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 text-xs">{cinematicBriefs.length}</Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="automations">
                <Zap className="h-3.5 w-3.5 mr-1" />
                Automations
                {automations.length > 0 && (
                  <Badge variant="secondary" className="ml-1.5 text-xs">{automations.length}</Badge>
                )}
              </TabsTrigger>
            </TabsList>
          </div>

          {/* Builder */}
          <TabsContent value="builder" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            <WorkflowBuilderTab />
          </TabsContent>

          {/* Staging Review */}
          <TabsContent value="staging" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            <StagingTab />
          </TabsContent>

          {/* Workflow Runs */}
          <TabsContent value="runs" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            <RunsTab />
          </TabsContent>

          {/* Active */}
          <TabsContent value="active" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {activeExecutions.length === 0 ? (
              <div className="empty-state h-full">
                <GitBranch className="empty-state-icon" />
                <p className="empty-state-title">No Active Workflows</p>
                <p className="empty-state-description">
                  Workflows appear here when agents execute tasks. Try asking Nora to run a workflow.
                </p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {activeExecutions.map((exec) => (
                  <Card
                    key={exec.executionId}
                    className="card-interactive border-l-4 border-l-yellow-500"
                    onClick={() => setSelectedExecution(exec)}
                  >
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-base">{exec.agentCodename}</CardTitle>
                        <Badge variant="outline" className="text-yellow-600">
                          <div className="status-dot status-dot-warning mr-1.5" />
                          Running
                        </Badge>
                      </div>
                      <p className="text-sm text-muted-foreground">{exec.workflowName}</p>
                    </CardHeader>
                    <CardContent>
                      <div className="space-y-2">
                        <div className="flex items-center justify-between text-sm">
                          <span className="text-muted-foreground">Stage:</span>
                          <span>{exec.stageName}</span>
                        </div>
                        <Progress
                          value={exec.totalStages > 0 ? (exec.currentStage / exec.totalStages) * 100 : 0}
                          className="h-1.5"
                        />
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <span>Stage {exec.currentStage} of {exec.totalStages || '?'}</span>
                          <span>Started: {formatTime(exec.startedAt)}</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Recent */}
          <TabsContent value="recent" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {completedExecutions.length === 0 ? (
              <div className="empty-state h-full">
                <Clock className="empty-state-icon" />
                <p className="empty-state-title">No Recent Workflows</p>
                <p className="empty-state-description">Completed workflows will appear here</p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {completedExecutions.map((exec) => (
                  <Card
                    key={exec.executionId}
                    className={`card-interactive border-l-4 ${exec.status === 'completed' ? 'border-l-green-500' : 'border-l-red-500'}`}
                    onClick={() => setSelectedExecution(exec)}
                  >
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-base">{exec.agentCodename}</CardTitle>
                        <Badge variant={exec.status === 'completed' ? 'default' : 'destructive'}>
                          {exec.status === 'completed' ? 'Completed' : 'Failed'}
                        </Badge>
                      </div>
                      <p className="text-sm text-muted-foreground">{exec.workflowName}</p>
                    </CardHeader>
                    <CardContent>
                      <div className="space-y-2">
                        {exec.status === 'completed' ? (
                          <>
                            <div className="flex items-center justify-between text-sm">
                              <span className="text-muted-foreground">Tasks Created:</span>
                              <span>{exec.tasksCreated || 0}</span>
                            </div>
                            <div className="flex items-center justify-between text-sm">
                              <span className="text-muted-foreground">Artifacts:</span>
                              <span>{exec.artifactsCount || 0}</span>
                            </div>
                          </>
                        ) : (
                          <div className="text-sm text-destructive">{exec.error || 'Execution failed'}</div>
                        )}
                        <div className="flex items-center justify-between text-xs text-muted-foreground pt-2 border-t border-border/30">
                          <span>Duration: {formatDuration(exec.durationMs)}</span>
                          <span>{formatTime(exec.completedAt)}</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Agent Flows */}
          <TabsContent value="flows" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {agentFlows.length === 0 ? (
              <div className="empty-state h-full">
                <History className="empty-state-icon" />
                <p className="empty-state-title">No Agent Flows</p>
                <p className="empty-state-description">Historical agent execution flows appear here</p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {agentFlows.map((flow) => (
                  <Card key={flow.id} className={`border-l-4 ${statusColor(flow.status)}`}>
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-sm capitalize">{flow.flow_type} flow</CardTitle>
                        <Badge variant={statusBadge(flow.status)} className="text-xs capitalize">
                          {flow.status}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <div className="space-y-1 text-sm">
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">Phase</span>
                          <span className="capitalize">{flow.current_phase}</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">Started</span>
                          <span>{formatDate(flow.created_at)}</span>
                        </div>
                        <div className="text-xs text-muted-foreground pt-1 truncate font-mono">{flow.id}</div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Research */}
          <TabsContent value="research" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {researchSessions.length === 0 ? (
              <div className="empty-state h-full">
                <Microscope className="empty-state-icon" />
                <p className="empty-state-title">No Research Sessions</p>
                <p className="empty-state-description">
                  Wide research sessions (multi-agent parallel research) appear here
                </p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {researchSessions.map((session) => (
                  <Card key={session.id} className={`border-l-4 ${statusColor(session.status)}`}>
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-sm line-clamp-1">{session.task_description}</CardTitle>
                        <Badge variant={statusBadge(session.status)} className="text-xs capitalize ml-2 shrink-0">
                          {session.status}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <div className="space-y-2 text-sm">
                        <Progress
                          value={session.total_subagents > 0 ? (session.completed_count / session.total_subagents) * 100 : 0}
                          className="h-1.5"
                        />
                        <div className="flex justify-between text-xs text-muted-foreground">
                          <span>{session.completed_count} / {session.total_subagents} subagents</span>
                          {session.failed_count > 0 && (
                            <span className="text-destructive">{session.failed_count} failed</span>
                          )}
                        </div>
                        <div className="flex justify-between text-xs text-muted-foreground">
                          <span>Parallelism: {session.parallelism_limit}</span>
                          <span>{formatDate(session.created_at)}</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Conference */}
          <TabsContent value="conference" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {conferences.length === 0 ? (
              <div className="empty-state h-full">
                <Calendar className="empty-state-icon" />
                <p className="empty-state-title">No Conference Workflows</p>
                <p className="empty-state-description">
                  Conference research, content, and social publishing pipelines appear here
                </p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {conferences.map((conf) => (
                  <Card key={conf.id} className={`border-l-4 ${statusColor(conf.status)}`}>
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-sm">{conf.conferenceName}</CardTitle>
                        <Badge variant={statusBadge(conf.status)} className="text-xs capitalize">
                          {conf.status}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <div className="space-y-1 text-sm">
                        {conf.location && (
                          <div className="flex justify-between">
                            <span className="text-muted-foreground">Location</span>
                            <span>{conf.location}</span>
                          </div>
                        )}
                        <div className="flex justify-between">
                          <span className="text-muted-foreground">Dates</span>
                          <span>{formatDate(conf.startDate)} – {formatDate(conf.endDate)}</span>
                        </div>
                        <div className="flex justify-between text-xs text-muted-foreground">
                          <span>Created</span>
                          <span>{formatDate(conf.createdAt)}</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Editron */}
          <TabsContent value="editron" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {cinematicBriefs.length === 0 ? (
              <div className="empty-state h-full">
                <Film className="empty-state-icon" />
                <p className="empty-state-title">No Editron Briefs</p>
                <p className="empty-state-description">
                  Cinematic briefs and render pipelines from Editron Pro appear here
                </p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {cinematicBriefs.map((brief) => (
                  <Card key={brief.id} className={`border-l-4 ${statusColor(brief.status)}`}>
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-sm">{brief.title}</CardTitle>
                        <Badge variant={statusBadge(brief.status)} className="text-xs capitalize">
                          {brief.status}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <div className="text-xs text-muted-foreground">{formatDate(brief.created_at)}</div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Automations */}
          <TabsContent value="automations" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {automations.length === 0 ? (
              <div className="empty-state h-full">
                <Zap className="empty-state-icon" />
                <p className="empty-state-title">No Automations</p>
                <p className="empty-state-description">CRM automations are loaded from the server</p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {automations.map((automation) => (
                  <Card key={automation.id} className="border-l-4 border-l-blue-500">
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-sm">{automation.name}</CardTitle>
                        <Badge variant="outline" className="text-xs gap-1 shrink-0 ml-2">
                          <Clock className="h-3 w-3" />
                          {automation.schedule}
                        </Badge>
                      </div>
                    </CardHeader>
                    <CardContent>
                      <p className="text-sm text-muted-foreground mb-3">{automation.description}</p>
                      <div className="space-y-1.5 text-xs">
                        <div className="flex gap-2">
                          <span className="text-muted-foreground shrink-0">Trigger:</span>
                          <span>{automation.trigger}</span>
                        </div>
                        <div className="flex gap-2">
                          <span className="text-muted-foreground shrink-0">Action:</span>
                          <span>{automation.action}</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>
        </Tabs>
      </div>

      {/* Detail Sheet */}
      <Sheet open={!!selectedExecution} onOpenChange={() => setSelectedExecution(null)}>
        <SheetContent className="w-[500px] sm:max-w-[600px]">
          <SheetHeader>
            <SheetTitle>{selectedExecution?.agentCodename} Execution</SheetTitle>
            <SheetDescription>{selectedExecution?.workflowName}</SheetDescription>
          </SheetHeader>
          {selectedExecution && (
            <div className="mt-6 space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="stat-card">
                  <div className="stat-card-label">Status</div>
                  <div className="font-medium capitalize">{selectedExecution.status}</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-label">Duration</div>
                  <div className="font-medium">{formatDuration(selectedExecution.durationMs)}</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-label">Tasks Created</div>
                  <div className="font-medium">{selectedExecution.tasksCreated || 0}</div>
                </div>
                <div className="stat-card">
                  <div className="stat-card-label">Artifacts</div>
                  <div className="font-medium">{selectedExecution.artifactsCount || 0}</div>
                </div>
              </div>
              <div className="card-inset p-3">
                <div className="text-xs text-muted-foreground mb-1">Execution ID</div>
                <code className="text-xs">{selectedExecution.executionId}</code>
              </div>
              {selectedExecution.error && (
                <div className="p-3 border border-destructive/30 rounded-lg bg-destructive/5">
                  <div className="text-xs text-destructive mb-1">Error</div>
                  <div className="text-sm">{selectedExecution.error}</div>
                </div>
              )}
              <div className="card-inset p-3">
                <div className="text-xs text-muted-foreground mb-2">Timeline</div>
                <div className="space-y-2 text-sm">
                  <div className="flex justify-between">
                    <span>Started</span>
                    <span>{formatTime(selectedExecution.startedAt)}</span>
                  </div>
                  {selectedExecution.completedAt && (
                    <div className="flex justify-between">
                      <span>Completed</span>
                      <span>{formatTime(selectedExecution.completedAt)}</span>
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
}

function WorkflowBuilderTab() {
  const queryClient = useQueryClient();
  const { data: workflows = [], isLoading } = useQuery({
    queryKey: ['workflowDefinitions'],
    queryFn: () => workflowsApi.listDefinitions(),
  });

  const [editorOpen, setEditorOpen] = useState(false);
  const [editingWorkflow, setEditingWorkflow] = useState<WorkflowDefinition | null>(null);
  const [runWorkflow, setRunWorkflow] = useState<WorkflowDefinition | null>(null);

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
        <div className="text-center py-16 text-muted-foreground">
          <Hammer className="h-10 w-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm font-medium">No workflows yet</p>
          <p className="text-xs mt-1">Create your first workflow to start processing data sources.</p>
          <Button size="sm" variant="outline" className="mt-4 gap-1.5" onClick={() => { setEditingWorkflow(null); setEditorOpen(true); }}>
            <Plus className="h-3.5 w-3.5" />
            Create Workflow
          </Button>
        </div>
      ) : (
        <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
          {workflows.map((wf: WorkflowDefinition) => {
            const nodeCount = wf.nodes?.length ?? 0;
            return (
              <Card
                key={wf.id}
                className="card-interactive cursor-pointer"
                onClick={() => { setEditingWorkflow(wf); setEditorOpen(true); }}
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
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      <WorkflowEditor
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

function RunWorkflowDialog({ workflow, onClose }: { workflow: WorkflowDefinition | null; onClose: () => void }) {
  const orgId = '01010101-0101-0101-0101-010101010101';
  const [selectedDataSourceId, setSelectedDataSourceId] = useState<string>('');
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [searchFilter, setSearchFilter] = useState('');
  const [dataTypeFilter, setDataTypeFilter] = useState<string>('__all__');
  const [reviewRunId, setReviewRunId] = useState<string | null>(null);

  const { data: dataSources = [] } = useQuery({
    queryKey: ['orgDataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    enabled: !!workflow,
  });

  const { data: availableModels } = useQuery({
    queryKey: ['workflowModels'],
    queryFn: () => workflowsApi.listAvailableModels(),
    staleTime: 60 * 60 * 1000,
    enabled: !!workflow,
  });

  const effectiveModel = selectedModel || availableModels?.find((m) => m.is_default)?.id || '';

  const filteredSources = useMemo(() => {
    let result = dataSources.filter((ds) => ds.status === 'ready');
    if (searchFilter) {
      const lower = searchFilter.toLowerCase();
      result = result.filter((ds) =>
        ds.title.toLowerCase().includes(lower) ||
        ds.description?.toLowerCase().includes(lower)
      );
    }
    if (dataTypeFilter && dataTypeFilter !== '__all__') {
      result = result.filter((ds) => ds.data_type === dataTypeFilter);
    }
    return result;
  }, [dataSources, searchFilter, dataTypeFilter]);

  const runMutation = useMutation({
    mutationFn: () => dataSourcesApi.runWorkflow(selectedDataSourceId, workflow!.id, effectiveModel || undefined),
    onSuccess: (data) => {
      if (data.workflow_run_id && data.staged_records > 0) {
        setReviewRunId(data.workflow_run_id);
      }
    },
  });

  const handleClose = () => {
    setSelectedDataSourceId('');
    setSelectedModel('');
    setSearchFilter('');
    setDataTypeFilter('__all__');
    setReviewRunId(null);
    runMutation.reset();
    onClose();
  };

  const sourceTypeIcon = (st: string) => {
    if (st === 'file') return <Upload className="h-3.5 w-3.5 text-muted-foreground" />;
    if (st === 'text') return <FileText className="h-3.5 w-3.5 text-muted-foreground" />;
    return <Database className="h-3.5 w-3.5 text-muted-foreground" />;
  };

  return (
    <>
      <Dialog open={!!workflow && !reviewRunId} onOpenChange={(open) => { if (!open) handleClose(); }}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Play className="h-4 w-4" />
              Run: {workflow?.name}
            </DialogTitle>
          </DialogHeader>

          <div className="space-y-4">
            {/* Filters */}
            <div className="flex items-center gap-2">
              <div className="relative flex-1">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
                <Input
                  placeholder="Search data sources..."
                  className="pl-8 h-8 text-sm"
                  value={searchFilter}
                  onChange={(e) => setSearchFilter(e.target.value)}
                />
              </div>
              <Select value={dataTypeFilter} onValueChange={setDataTypeFilter}>
                <SelectTrigger className="h-8 w-[140px] text-xs">
                  <SelectValue placeholder="All types" />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="__all__">All types</SelectItem>
                  {DATA_TYPE_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            {/* Data source list */}
            <div className="border rounded-md max-h-64 overflow-auto">
              {filteredSources.length === 0 ? (
                <div className="py-8 text-center text-sm text-muted-foreground">
                  <Database className="h-6 w-6 mx-auto mb-2 opacity-40" />
                  {dataSources.length === 0
                    ? 'No data sources available. Add data sources in the Knowledge tab.'
                    : 'No data sources match your filters.'}
                </div>
              ) : (
                filteredSources.map((ds) => (
                  <button
                    key={ds.id}
                    className={`flex items-center gap-3 w-full text-left px-3 py-2.5 border-b last:border-b-0 transition-colors ${
                      selectedDataSourceId === ds.id
                        ? 'bg-primary/10 border-l-2 border-l-primary'
                        : 'hover:bg-muted/50'
                    }`}
                    onClick={() => setSelectedDataSourceId(ds.id)}
                  >
                    {sourceTypeIcon(ds.source_type)}
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium truncate">{ds.title}</div>
                      {ds.description && (
                        <div className="text-xs text-muted-foreground truncate">{ds.description}</div>
                      )}
                    </div>
                    <Badge variant="outline" className="text-[9px] shrink-0">
                      {DATA_TYPE_OPTIONS.find((o) => o.value === ds.data_type)?.label ?? ds.data_type}
                    </Badge>
                  </button>
                ))
              )}
            </div>

            {/* Model selector */}
            {Array.isArray(availableModels) && availableModels.length > 0 && (
              <div className="flex items-center gap-2">
                <span className="text-xs text-muted-foreground shrink-0">Model:</span>
                <Select value={effectiveModel} onValueChange={setSelectedModel}>
                  <SelectTrigger className="h-8 text-xs">
                    <SelectValue placeholder="Use workflow default" />
                  </SelectTrigger>
                  <SelectContent>
                    {availableModels.map((m) => (
                      <SelectItem key={m.id} value={m.id}>{m.label}</SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            )}

            {/* Run result */}
            {runMutation.isError && (
              <div className="p-3 rounded-md bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 text-sm text-red-700 dark:text-red-300">
                Failed: {(runMutation.error as Error)?.message ?? 'Unknown error'}
              </div>
            )}

            {runMutation.isSuccess && (
              <div className="p-3 rounded-md bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 text-sm text-green-700 dark:text-green-300">
                Workflow completed. {runMutation.data?.staged_records ?? 0} records staged.
              </div>
            )}

            {/* Actions */}
            <div className="flex items-center justify-end gap-2">
              <Button variant="outline" size="sm" onClick={handleClose}>Cancel</Button>
              <Button
                size="sm"
                className="gap-1.5"
                onClick={() => runMutation.mutate()}
                disabled={!selectedDataSourceId || runMutation.isPending}
              >
                {runMutation.isPending ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : (
                  <Play className="h-3.5 w-3.5" />
                )}
                Run Workflow
              </Button>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      {reviewRunId && (
        <StagingReviewPanel
          open
          onOpenChange={(open) => { if (!open) { setReviewRunId(null); handleClose(); } }}
          workflowRunId={reviewRunId}
          workflowName={workflow?.name}
        />
      )}
    </>
  );
}

function StagingTab() {
  const [selectedRunId, setSelectedRunId] = useState<string | null>(null);
  const orgId = '01010101-0101-0101-0101-010101010101';

  const { data: pendingRecords = [], isLoading } = useQuery({
    queryKey: ['stagingPending', orgId],
    queryFn: () => stagingApi.listPending(orgId),
    refetchInterval: 15000,
  });

  const grouped = useMemo(() => {
    const byRun: Record<string, { runId: string; records: WorkflowStagingRecord[]; workflowName?: string }> = {};
    for (const r of pendingRecords) {
      const rid = r.workflow_run_id;
      if (!byRun[rid]) byRun[rid] = { runId: rid, records: [], workflowName: undefined };
      byRun[rid].records.push(r);
    }
    return Object.values(byRun);
  }, [pendingRecords]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading staging records...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4 max-w-[1200px] mx-auto">
      <div>
        <h2 className="text-lg font-semibold">Staging Review</h2>
        <p className="text-sm text-muted-foreground">
          Review and approve records extracted by workflows before they are committed to the CRM.
        </p>
      </div>

      {grouped.length === 0 ? (
        <div className="text-center py-16 text-muted-foreground">
          <ClipboardCheck className="h-10 w-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm font-medium">No pending records</p>
          <p className="text-xs mt-1">Records extracted by workflow runs will appear here for review.</p>
        </div>
      ) : (
        <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
          {grouped.map((group) => {
            const statuses = group.records.reduce<Record<string, number>>((acc, r) => {
              acc[r.status] = (acc[r.status] || 0) + 1;
              return acc;
            }, {});
            return (
              <Card
                key={group.runId}
                className="card-interactive cursor-pointer"
                onClick={() => setSelectedRunId(group.runId)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm">{group.records.length} Records</CardTitle>
                    <Badge variant="outline" className="text-[10px]">Pending Review</Badge>
                  </div>
                  <p className="text-xs text-muted-foreground font-mono truncate">{group.runId}</p>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-1">
                    {Object.entries(statuses).map(([status, count]) => (
                      <Badge key={status} variant="secondary" className="text-[9px]">
                        {status}: {count}
                      </Badge>
                    ))}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {selectedRunId && (
        <StagingReviewPanel
          open
          onOpenChange={(open) => { if (!open) setSelectedRunId(null); }}
          workflowRunId={selectedRunId}
        />
      )}
    </div>
  );
}

function RunsTab() {
  const { data: recentRuns = [], isLoading } = useQuery({
    queryKey: ['workflowRuns'],
    queryFn: () => workflowsApi.listRecentRuns({ limit: 50 }),
    refetchInterval: 10000,
  });

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading workflow runs...</div>
      </div>
    );
  }

  const formatDurationShort = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
  };

  return (
    <div className="space-y-4 max-w-[1200px] mx-auto">
      <div>
        <h2 className="text-lg font-semibold">Workflow Runs</h2>
        <p className="text-sm text-muted-foreground">
          History of workflow executions with token usage, cost, and output metrics.
        </p>
      </div>

      {recentRuns.length === 0 ? (
        <div className="text-center py-16 text-muted-foreground">
          <Activity className="h-10 w-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm font-medium">No workflow runs yet</p>
          <p className="text-xs mt-1">Run a workflow from a data source to see execution history here.</p>
        </div>
      ) : (
        <div className="space-y-2">
          {recentRuns.map((run: any) => (
            <Card key={run.id} className={`border-l-4 ${run.status === 'completed' ? 'border-l-green-500' : run.status === 'failed' ? 'border-l-red-500' : 'border-l-yellow-500'}`}>
              <CardContent className="py-3 px-4">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Badge variant={run.status === 'completed' ? 'default' : run.status === 'failed' ? 'destructive' : 'outline'} className="text-[10px] capitalize">
                      {run.status}
                    </Badge>
                    <span className="text-sm font-medium">{run.workflow_name || run.workflow_id}</span>
                  </div>
                  <div className="flex items-center gap-4 text-xs text-muted-foreground">
                    {run.duration_ms && <span>{formatDurationShort(run.duration_ms)}</span>}
                    {run.total_cost_micros != null && <span>${(run.total_cost_micros / 1_000_000).toFixed(4)}</span>}
                    {run.records_staged != null && <span>{run.records_staged} records</span>}
                    <span>{new Date(run.started_at).toLocaleString()}</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

export default WorkflowsPage;
