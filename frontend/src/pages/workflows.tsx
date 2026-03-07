import { useState, useEffect } from 'react';
import { useExecutionEvents, ActiveExecution } from '@/hooks/useExecutionEvents';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
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
} from 'lucide-react';
import { agentFlowsApi, wideResearchApi, resolveApiUrl } from '@/lib/api';
import type { AgentFlow, WideResearchSession } from '@/lib/api';

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
  const [activeTab, setActiveTab] = useState('active');

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
            <TabsList className="tab-grid-7">
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

export default WorkflowsPage;
