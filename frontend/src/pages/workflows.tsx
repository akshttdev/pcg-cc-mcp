import { useState, useEffect, useMemo, useCallback } from 'react';
import { useSearchParams, useNavigate } from 'react-router-dom';
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
  ChevronDown,
  Pencil,
  ArrowRight,
  CheckCheck,
  XCircle,
  LayoutGrid,
  TableProperties,
  ChevronRight,
  AlertTriangle,
  Users,
  Building2,
  Handshake,
  ListTodo,
  Check,
  RotateCcw,
  X,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { agentFlowsApi, wideResearchApi, workflowsApi, dataSourcesApi, stagingApi, resolveApiUrl, DATA_TYPE_OPTIONS } from '@/lib/api';
import type { AgentFlow, WideResearchSession, WorkflowDefinition, WorkflowStagingRecord } from '@/lib/api';
import { WorkflowEditor, getNodeTypeDef } from '@/components/workflows/WorkflowEditor';
import { WorkflowTriggersPanel } from '@/components/workflows/WorkflowTriggersPanel';
import { WorkflowRunsPanel } from '@/components/workflows/WorkflowRunsPanel';
import { StagingReviewContent } from '@/components/workflows/StagingReviewPanel';
import { useAuth } from '@/contexts/AuthContext';
import { cn } from '@/lib/utils';

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
  const [searchParams, setSearchParams] = useSearchParams();
  const [selectedExecution, setSelectedExecution] = useState<ActiveExecution | null>(null);
  const [activeTab, setActiveTab] = useState(() => searchParams.get('tab') || 'builder');

  // Respond to URL param changes for tab switching
  const tabParam = searchParams.get('tab');
  useEffect(() => {
    if (tabParam) {
      setActiveTab(tabParam);
    }
  }, [tabParam]);

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
        <Tabs value={activeTab} onValueChange={(tab) => { setActiveTab(tab); setSearchParams({ tab }, { replace: true }); }} className="h-full flex flex-col">
          <div className="border-b px-4 sm:px-6">
            <div className="flex items-center gap-1">
              <TabsList>
                <TabsTrigger value="builder">
                  <Hammer className="h-3.5 w-3.5 mr-1" />
                  Builder
                </TabsTrigger>
                <TabsTrigger value="runs">
                  <Activity className="h-3.5 w-3.5 mr-1" />
                  Runs
                  {activeExecutions.length > 0 && (
                    <Badge variant="secondary" className="ml-1.5 animate-pulse text-xs">
                      {activeExecutions.length}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger value="staging">
                  <ClipboardCheck className="h-3.5 w-3.5 mr-1" />
                  Staging
                </TabsTrigger>
                <TabsTrigger value="automations">
                  <Zap className="h-3.5 w-3.5 mr-1" />
                  Automations
                </TabsTrigger>
              </TabsList>

              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm" className="h-8 gap-1 text-xs text-muted-foreground ml-1">
                    More
                    <ChevronDown className="h-3 w-3" />
                    {(activeExecutions.length + completedExecutions.length + agentFlows.length + researchSessions.length + conferences.length + cinematicBriefs.length) > 0 && (
                      <Badge variant="secondary" className="text-[9px] px-1 h-4 ml-0.5">
                        {activeExecutions.length + completedExecutions.length + agentFlows.length + researchSessions.length + conferences.length + cinematicBriefs.length}
                      </Badge>
                    )}
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="start">
                  <DropdownMenuItem onClick={() => setActiveTab('active')}>
                    <Play className="h-3.5 w-3.5 mr-2" />
                    Active Executions
                    {activeExecutions.length > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[9px]">{activeExecutions.length}</Badge>
                    )}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setActiveTab('recent')}>
                    <Clock className="h-3.5 w-3.5 mr-2" />
                    Recent Executions
                    {completedExecutions.length > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[9px]">{completedExecutions.length}</Badge>
                    )}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setActiveTab('flows')}>
                    <History className="h-3.5 w-3.5 mr-2" />
                    Agent Flows
                    {agentFlows.length > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[9px]">{agentFlows.length}</Badge>
                    )}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setActiveTab('research')}>
                    <Microscope className="h-3.5 w-3.5 mr-2" />
                    Research
                    {researchSessions.length > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[9px]">{researchSessions.length}</Badge>
                    )}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setActiveTab('conference')}>
                    <Calendar className="h-3.5 w-3.5 mr-2" />
                    Conference
                    {conferences.length > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[9px]">{conferences.length}</Badge>
                    )}
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => setActiveTab('editron')}>
                    <Film className="h-3.5 w-3.5 mr-2" />
                    Editron
                    {cinematicBriefs.length > 0 && (
                      <Badge variant="secondary" className="ml-auto text-[9px]">{cinematicBriefs.length}</Badge>
                    )}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
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
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {/* Workflow Detail Panel — shown inline below the grid when a workflow is selected */}
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

/** Inline detail panel shown below the workflow grid when a workflow card is clicked */
function WorkflowDetailPanel({
  workflow,
  onEdit,
  onRun,
  onViewTriggers,
  onClose,
}: {
  workflow: WorkflowDefinition;
  onEdit: () => void;
  onRun: () => void;
  onViewTriggers: () => void;
  onClose: () => void;
}) {
  const navigate = useNavigate();
  const [detailTab, setDetailTab] = useState<'overview' | 'runs' | 'staging'>('overview');

  const { data: recentRuns = [] } = useQuery({
    queryKey: ['workflowRunsByWf', workflow.id],
    queryFn: () => workflowsApi.listRecentRuns({ workflow_id: workflow.id, limit: 10 }),
    refetchInterval: 15000,
  });

  const { user } = useAuth();
  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;
  const { data: pendingRecords = [] } = useQuery({
    queryKey: ['stagingPendingByWf', workflow.id],
    queryFn: () => stagingApi.listPending(orgId!),
    enabled: !!orgId,
  });

  // Filter pending records to this workflow's runs
  const workflowPending = useMemo(() => {
    const runIds = new Set(recentRuns.map((r: any) => r.id));
    return pendingRecords.filter((r) => runIds.has(r.workflow_run_id));
  }, [pendingRecords, recentRuns]);

  const pendingByRun = useMemo(() => {
    const byRun: Record<string, WorkflowStagingRecord[]> = {};
    for (const r of workflowPending) {
      if (!byRun[r.workflow_run_id]) byRun[r.workflow_run_id] = [];
      byRun[r.workflow_run_id].push(r);
    }
    return byRun;
  }, [workflowPending]);

  const formatDurationShort = (ms?: number) => {
    if (!ms) return '-';
    if (ms < 1000) return `${ms}ms`;
    if (ms < 60000) return `${(ms / 1000).toFixed(1)}s`;
    return `${Math.floor(ms / 60000)}m ${Math.round((ms % 60000) / 1000)}s`;
  };

  const nodeCount = workflow.nodes?.length ?? 0;
  const outputNodes = (workflow.nodes ?? []).filter((n: any) =>
    ['output_crm_contacts', 'output_crm_companies', 'output_crm_deals', 'output_tasks'].includes(n.type)
  );

  const totalRuns = recentRuns.length;
  const successRuns = recentRuns.filter((r: any) => r.status === 'completed').length;
  const totalRecordsStaged = recentRuns.reduce((sum: number, r: any) => sum + (r.records_staged || 0), 0);

  return (
    <Card className="border-primary/30 bg-card/80">
      <CardHeader className="pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <CardTitle className="text-base">{workflow.name}</CardTitle>
            {workflow.is_system && <Badge variant="secondary" className="text-[10px]">System</Badge>}
          </div>
          <div className="flex items-center gap-1.5">
            <Button size="sm" variant="outline" className="gap-1.5 h-7 text-xs" onClick={onRun}>
              <Play className="h-3 w-3" />
              Run
            </Button>
            <Button size="sm" variant="outline" className="gap-1.5 h-7 text-xs" onClick={onEdit}>
              <Pencil className="h-3 w-3" />
              Edit
            </Button>
            <Button size="sm" variant="outline" className="gap-1.5 h-7 text-xs" onClick={onViewTriggers}>
              <Zap className="h-3 w-3" />
              Triggers
            </Button>
            <Button size="sm" variant="ghost" className="h-7 w-7 p-0" onClick={onClose}>
              <span className="sr-only">Close</span>
              &times;
            </Button>
          </div>
        </div>
        {workflow.description && (
          <p className="text-sm text-muted-foreground mt-1">{workflow.description}</p>
        )}
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Quick stats */}
        <div className="grid grid-cols-4 gap-3">
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold">{nodeCount}</div>
            <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Nodes</div>
          </div>
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold">{totalRuns}</div>
            <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Runs</div>
          </div>
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold text-green-600">{successRuns}</div>
            <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Successful</div>
          </div>
          <div className="rounded-md border p-2.5 text-center">
            <div className="text-lg font-semibold">{totalRecordsStaged}</div>
            <div className="text-[10px] text-muted-foreground uppercase tracking-wide">Records</div>
          </div>
        </div>

        {/* Output types */}
        {outputNodes.length > 0 && (
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted-foreground">Outputs:</span>
            {outputNodes.map((node: any) => {
              const outputLabels: Record<string, string> = {
                output_crm_contacts: 'Contacts',
                output_crm_companies: 'Companies',
                output_crm_deals: 'Deals',
                output_tasks: 'Tasks',
              };
              return (
                <Badge key={node.id} variant="secondary" className="text-[10px]">
                  {outputLabels[node.type] ?? node.type}
                </Badge>
              );
            })}
          </div>
        )}

        {/* Sub-tabs: Overview / Runs / Staging */}
        <div className="border-t pt-3">
          <div className="flex gap-1 mb-3">
            {(['overview', 'runs', 'staging'] as const).map((tab) => (
              <Button
                key={tab}
                size="sm"
                variant={detailTab === tab ? 'default' : 'ghost'}
                className="h-7 text-xs capitalize"
                onClick={() => setDetailTab(tab)}
              >
                {tab}
                {tab === 'staging' && workflowPending.length > 0 && (
                  <Badge variant="secondary" className="ml-1 text-[9px] h-4 px-1">{workflowPending.length}</Badge>
                )}
                {tab === 'runs' && totalRuns > 0 && (
                  <Badge variant="secondary" className="ml-1 text-[9px] h-4 px-1">{totalRuns}</Badge>
                )}
              </Button>
            ))}
          </div>

          {detailTab === 'overview' && (
            <div className="space-y-2">
              <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">Pipeline</p>
              <div className="flex flex-wrap items-center gap-1.5">
                {(workflow.nodes ?? []).map((node: any, idx: number) => (
                    <div key={node.id} className="flex items-center gap-1.5">
                      {idx > 0 && <ArrowRight className="h-3 w-3 text-muted-foreground" />}
                      <Badge variant="outline" className="text-[10px]">
                        {node.name}
                      </Badge>
                    </div>
                  ))}
              </div>
            </div>
          )}

          {detailTab === 'runs' && (
            <div className="space-y-1.5 max-h-48 overflow-auto">
              {recentRuns.length === 0 ? (
                <p className="text-xs text-muted-foreground py-4 text-center">No runs yet. Click "Run" to execute this workflow.</p>
              ) : (
                recentRuns.map((run: any) => (
                  <div
                    key={run.id}
                    className={`flex items-center justify-between p-2 rounded-md border text-xs cursor-pointer hover:bg-muted/50 ${
                      run.status === 'completed' ? 'border-l-2 border-l-green-500' : run.status === 'failed' ? 'border-l-2 border-l-red-500' : 'border-l-2 border-l-yellow-500'
                    }`}
                    onClick={() => {
                      if (run.records_staged > 0) navigate(`/workflows?tab=staging&run=${run.id}`);
                    }}
                  >
                    <div className="flex items-center gap-2">
                      <Badge variant={run.status === 'completed' ? 'default' : run.status === 'failed' ? 'destructive' : 'outline'} className="text-[9px] capitalize">
                        {run.status}
                      </Badge>
                      <span className="text-muted-foreground">{formatDurationShort(run.duration_ms)}</span>
                      {run.records_staged > 0 && (
                        <span>{run.records_staged} records</span>
                      )}
                    </div>
                    <span className="text-muted-foreground">
                      {new Date(run.started_at).toLocaleDateString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                    </span>
                  </div>
                ))
              )}
            </div>
          )}

          {detailTab === 'staging' && (
            <div className="space-y-1.5 max-h-48 overflow-auto">
              {workflowPending.length === 0 ? (
                <p className="text-xs text-muted-foreground py-4 text-center">No pending records for this workflow.</p>
              ) : (
                Object.entries(pendingByRun).map(([runId, records]) => (
                  <div
                    key={runId}
                    className="flex items-center justify-between p-2 rounded-md border text-xs cursor-pointer hover:bg-muted/50"
                    onClick={() => navigate(`/workflows?tab=staging&run=${runId}`)}
                  >
                    <div className="flex items-center gap-2">
                      <Badge variant="outline" className="text-[9px]">Pending</Badge>
                      <span>{records.length} record{records.length !== 1 ? 's' : ''}</span>
                    </div>
                    <div className="flex items-center gap-1.5">
                      {(() => {
                        const types: Record<string, number> = {};
                        for (const r of records) types[r.target_type] = (types[r.target_type] || 0) + 1;
                        return Object.entries(types).map(([t, c]) => (
                          <Badge key={t} variant="secondary" className="text-[9px]">{c} {t.replace('crm_', '')}</Badge>
                        ));
                      })()}
                    </div>
                  </div>
                ))
              )}
            </div>
          )}
        </div>
      </CardContent>

    </Card>
  );
}

function RunWorkflowDialog({ workflow, onClose }: { workflow: WorkflowDefinition | null; onClose: () => void }) {
  const navigate = useNavigate();
  const { user } = useAuth();
  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;
  const [selectedDataSourceId, setSelectedDataSourceId] = useState<string>('');
  const [selectedModel, setSelectedModel] = useState<string>('');
  const [searchFilter, setSearchFilter] = useState('');
  const [dataTypeFilter, setDataTypeFilter] = useState<string>('__all__');
  const { data: dataSources = [] } = useQuery({
    queryKey: ['orgDataSources', orgId],
    queryFn: () => dataSourcesApi.listByOrganization(orgId!),
    enabled: !!workflow && !!orgId,
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
        handleClose();
        navigate('/workflows?tab=staging&run=' + data.workflow_run_id);
      }
    },
  });

  const handleClose = () => {
    setSelectedDataSourceId('');
    setSelectedModel('');
    setSearchFilter('');
    setDataTypeFilter('__all__');
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
      <Dialog open={!!workflow} onOpenChange={(open) => { if (!open) handleClose(); }}>
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

    </>
  );
}

const STAGING_TARGET_CONFIG: Record<string, { label: string; icon: typeof Users; color: string }> = {
  crm_contact: { label: 'Contacts', icon: Users, color: 'text-blue-500' },
  company: { label: 'Companies', icon: Building2, color: 'text-purple-500' },
  crm_deal: { label: 'Deals', icon: Handshake, color: 'text-green-500' },
  task: { label: 'Tasks', icon: ListTodo, color: 'text-orange-500' },
};

// Inline editable field for expanded row detail panel
function InlineEditField({
  fieldKey,
  value,
  recordId,
  onSave,
  editable,
}: {
  fieldKey: string;
  value: string;
  recordId: string;
  onSave: (recordId: string, key: string, value: string) => void;
  editable: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const [editValue, setEditValue] = useState(value);
  const inputRef = useCallback((el: HTMLInputElement | HTMLTextAreaElement | null) => {
    if (el) el.focus();
  }, []);

  const handleSave = useCallback(() => {
    if (editValue !== value) {
      onSave(recordId, fieldKey, editValue);
    }
    setEditing(false);
  }, [editValue, value, onSave, recordId, fieldKey]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSave();
    }
    if (e.key === 'Escape') {
      setEditValue(value);
      setEditing(false);
    }
  }, [handleSave, value]);

  const handleStartEdit = useCallback(() => {
    if (!editable) return;
    setEditValue(value);
    setEditing(true);
  }, [editable, value]);

  const isLong = value.length > 80;

  if (editing) {
    return isLong ? (
      <textarea
        ref={inputRef as React.Ref<HTMLTextAreaElement>}
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={handleSave}
        onKeyDown={handleKeyDown}
        className="w-full text-xs bg-background border rounded px-1.5 py-1 focus:outline-none focus:ring-1 focus:ring-ring min-h-[60px] resize-y"
      />
    ) : (
      <input
        ref={inputRef as React.Ref<HTMLInputElement>}
        value={editValue}
        onChange={(e) => setEditValue(e.target.value)}
        onBlur={handleSave}
        onKeyDown={handleKeyDown}
        className="w-full text-xs bg-background border rounded px-1.5 py-0.5 focus:outline-none focus:ring-1 focus:ring-ring h-6"
      />
    );
  }

  return (
    <dd
      onClick={editable ? handleStartEdit : undefined}
      className={cn(
        'text-xs mt-0.5',
        isLong ? 'whitespace-pre-wrap break-words' : 'truncate',
        editable && 'cursor-text hover:bg-muted/40 rounded px-1 -mx-1 transition-colors',
      )}
      title={editable ? 'Click to edit' : undefined}
    >
      {value || <span className="text-muted-foreground italic">empty</span>}
    </dd>
  );
}

type GlobalFilterType = 'all' | 'valid' | 'duplicates' | 'approved' | 'rejected' | 'issues' | 'error';
const VALID_GLOBAL_FILTERS: GlobalFilterType[] = ['all', 'valid', 'duplicates', 'approved', 'rejected', 'issues', 'error'];

function StagingTab() {
  const [searchParams, setSearchParams] = useSearchParams();
  const { user } = useAuth();
  const orgId = user?.home_organization_id ?? user?.organizations?.[0]?.id;
  const [expandedId, setExpandedId] = useState<string | null>(null);

  // Persist view/filter state in URL params
  const activeRunId = searchParams.get('run');
  const viewMode = (searchParams.get('view') === 'table' ? 'table' : 'cards') as 'cards' | 'table';
  const globalFilter = (VALID_GLOBAL_FILTERS.includes(searchParams.get('filter') as GlobalFilterType)
    ? searchParams.get('filter') as GlobalFilterType : 'all');
  const typeFilter = searchParams.get('type') || 'all';
  const workflowFilter = searchParams.get('wf') || 'all';

  // Helper to update search params preserving existing ones
  const updateParams = useCallback((updates: Record<string, string | null>) => {
    setSearchParams(prev => {
      const next = new URLSearchParams(prev);
      for (const [key, value] of Object.entries(updates)) {
        if (value === null || value === 'all' || value === 'cards') {
          next.delete(key);
        } else {
          next.set(key, value);
        }
      }
      // Always keep tab=staging
      next.set('tab', 'staging');
      return next;
    }, { replace: true });
  }, [setSearchParams]);

  const { data: pendingRecords = [], isLoading } = useQuery({
    queryKey: ['stagingPending', orgId],
    queryFn: () => stagingApi.listPending(orgId!),
    enabled: !!orgId,
    refetchInterval: 15000,
  });

  // Fetch recent runs to map run IDs to workflow names
  const { data: recentRuns = [] } = useQuery({
    queryKey: ['workflowRuns'],
    queryFn: () => workflowsApi.listRecentRuns({ limit: 100 }),
    staleTime: 30000,
  });

  const runNameMap = useMemo(() => {
    const m: Record<string, string> = {};
    for (const r of recentRuns as any[]) {
      if (r.id && r.workflow_name) m[r.id] = r.workflow_name;
    }
    return m;
  }, [recentRuns]);

  const grouped = useMemo(() => {
    const byRun: Record<string, { runId: string; records: WorkflowStagingRecord[]; workflowName?: string }> = {};
    for (const r of pendingRecords) {
      const rid = r.workflow_run_id;
      if (!byRun[rid]) byRun[rid] = { runId: rid, records: [], workflowName: runNameMap[rid] };
      byRun[rid].records.push(r);
    }
    return Object.values(byRun);
  }, [pendingRecords, runNameMap]);

  const queryClient = useQueryClient();

  // Global counts (always computed, even if not rendered)
  const totalPending = pendingRecords.filter(r => r.status === 'pending_review').length;
  const totalApproved = pendingRecords.filter(r => r.status === 'approved').length;
  const totalDuplicates = pendingRecords.filter(r => r.duplicate_of_id != null).length;
  const pendingNonDuplicate = useMemo(
    () => pendingRecords.filter(r => r.status === 'pending_review' && !r.duplicate_of_id),
    [pendingRecords]
  );

  // Global batch mutations — must be declared before early returns
  const batchApproveMutation = useMutation({
    mutationFn: async () => {
      await stagingApi.batchAction(pendingNonDuplicate.map(r => r.id), 'approve');
      // Auto-commit after approving: commit all runs that have approved records
      const runIds = [...new Set(pendingNonDuplicate.map(r => r.workflow_run_id))];
      return Promise.all(runIds.map(rid => stagingApi.batchCommit(rid)));
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['stagingPending'] }),
  });

  const batchRejectDupsMutation = useMutation({
    mutationFn: () => {
      const dupIds = pendingRecords.filter(r => r.status === 'pending_review' && r.duplicate_of_id != null).map(r => r.id);
      return stagingApi.batchAction(dupIds, 'reject');
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['stagingPending'] }),
  });

  // Per-record mutations for table view actions
  const updateMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: { status?: string; record_data?: any } }) =>
      stagingApi.update(id, data),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['stagingPending'] }),
  });

  const commitMutation = useMutation({
    mutationFn: (id: string) => stagingApi.commit(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['stagingPending'] }),
  });

  const retryMutation = useMutation({
    mutationFn: async (id: string) => {
      await stagingApi.update(id, { status: 'approved' });
      return stagingApi.commit(id);
    },
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['stagingPending'] }),
  });

  const handleApproveRecord = useCallback((id: string) => {
    // Auto-commit: approve then immediately commit to CRM
    updateMutation.mutate({ id, data: { status: 'approved' } }, {
      onSuccess: () => {
        commitMutation.mutate(id);
      },
    });
  }, [updateMutation, commitMutation]);

  const handleRejectRecord = useCallback((id: string) => {
    updateMutation.mutate({ id, data: { status: 'rejected' } });
  }, [updateMutation]);

  const handleRetryRecord = useCallback((id: string) => {
    retryMutation.mutate(id);
  }, [retryMutation]);

  const handleToggleExpand = useCallback((id: string) => {
    setExpandedId(prev => prev === id ? null : id);
  }, []);

  const handleStopPropagation = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
  }, []);

  // Inline field edit: update a single field in record_data
  const handleInlineFieldSave = useCallback((recordId: string, fieldKey: string, newValue: string) => {
    const record = pendingRecords.find(r => r.id === recordId);
    if (!record) return;
    try {
      const data = JSON.parse(record.record_data);
      data[fieldKey] = newValue;
      updateMutation.mutate({ id: recordId, data: { record_data: data } });
    } catch {}
  }, [pendingRecords, updateMutation]);

  const handleRejectDups = useCallback(() => {
    batchRejectDupsMutation.mutate();
  }, [batchRejectDupsMutation]);

  const handleApproveAll = useCallback(() => {
    batchApproveMutation.mutate();
  }, [batchApproveMutation]);

  const selectRun = useCallback((runId: string) => {
    updateParams({ run: runId });
  }, [updateParams]);

  const clearRun = useCallback(() => {
    updateParams({ run: null });
  }, [updateParams]);

  const setCardsView = useCallback(() => updateParams({ view: null }), [updateParams]);
  const setTableView = useCallback(() => updateParams({ view: 'table' }), [updateParams]);

  // Sorting state for global table
  const [sortField, setSortField] = useState<'name' | 'type' | 'workflow' | 'status' | 'confidence'>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  const handleSort = useCallback((field: typeof sortField) => {
    setSortDir(prev => sortField === field ? (prev === 'asc' ? 'desc' : 'asc') : 'asc');
    setSortField(field);
  }, [sortField]);

  // Derived filter counts
  const totalRejected = pendingRecords.filter(r => r.status === 'rejected').length;
  const totalError = pendingRecords.filter(r => r.status === 'error').length;
  const totalWithIssues = useMemo(() => pendingRecords.filter(r => {
    if (!r.validation_errors) return false;
    try { const e = JSON.parse(r.validation_errors); return Array.isArray(e) && e.length > 0; } catch { return false; }
  }).length, [pendingRecords]);

  // Unique target types and workflows for filter dropdowns
  const uniqueTypes = useMemo(() => {
    const types = new Set(pendingRecords.map(r => r.target_type));
    return Array.from(types);
  }, [pendingRecords]);

  const uniqueWorkflows = useMemo(() => {
    const wfs: Record<string, string> = {};
    for (const r of pendingRecords) {
      if (!wfs[r.workflow_run_id]) wfs[r.workflow_run_id] = runNameMap[r.workflow_run_id] || r.workflow_run_id.slice(0, 8);
    }
    return Object.entries(wfs);
  }, [pendingRecords, runNameMap]);

  // Detect common warnings that appear on many records — collapse into batch banner
  const commonWarnings = useMemo(() => {
    const warnCounts: Record<string, number> = {};
    for (const r of pendingRecords) {
      if (!r.validation_errors) continue;
      try {
        const errs = JSON.parse(r.validation_errors);
        if (Array.isArray(errs)) {
          for (const e of errs) warnCounts[e] = (warnCounts[e] || 0) + 1;
        }
      } catch {}
    }
    // Warnings appearing on more than half of records are "common"
    const threshold = Math.max(2, Math.floor(pendingRecords.length * 0.5));
    return Object.entries(warnCounts)
      .filter(([, count]) => count >= threshold)
      .map(([msg, count]) => ({ msg, count }));
  }, [pendingRecords]);

  const commonWarningSet = useMemo(() => new Set(commonWarnings.map(w => w.msg)), [commonWarnings]);

  // Filtered records for global table view
  const filteredRecords = useMemo(() => {
    let result = pendingRecords;

    // Status/category filter
    switch (globalFilter) {
      case 'valid':
        result = result.filter(r => r.status === 'pending_review' && !r.duplicate_of_id);
        break;
      case 'duplicates':
        result = result.filter(r => r.duplicate_of_id != null);
        break;
      case 'approved':
        result = result.filter(r => r.status === 'approved');
        break;
      case 'rejected':
        result = result.filter(r => r.status === 'rejected');
        break;
      case 'error':
        result = result.filter(r => r.status === 'error');
        break;
      case 'issues':
        result = result.filter(r => {
          if (!r.validation_errors) return false;
          try { const e = JSON.parse(r.validation_errors); return Array.isArray(e) && e.length > 0; } catch { return false; }
        });
        break;
    }

    // Type filter
    if (typeFilter !== 'all') {
      result = result.filter(r => r.target_type === typeFilter);
    }

    // Workflow filter
    if (workflowFilter !== 'all') {
      result = result.filter(r => r.workflow_run_id === workflowFilter);
    }

    return result;
  }, [pendingRecords, globalFilter, typeFilter, workflowFilter]);

  // Parsed data for table rows with sorting
  const tableRows = useMemo(() => {
    const rows = filteredRecords.map(record => {
      let data: Record<string, any> = {};
      try { data = JSON.parse(record.record_data); } catch {}
      const displayName = data.first_name
        ? `${data.first_name} ${data.last_name || ''}`
        : data.name || data.title || 'Untitled';
      return { record, displayName, data, workflowName: runNameMap[record.workflow_run_id] || '' };
    });

    // Apply sorting
    rows.sort((a, b) => {
      let cmp = 0;
      switch (sortField) {
        case 'name': cmp = a.displayName.localeCompare(b.displayName); break;
        case 'type': cmp = a.record.target_type.localeCompare(b.record.target_type); break;
        case 'workflow': cmp = a.workflowName.localeCompare(b.workflowName); break;
        case 'status': cmp = a.record.status.localeCompare(b.record.status); break;
        case 'confidence': cmp = (a.record.confidence ?? 0) - (b.record.confidence ?? 0); break;
      }
      return sortDir === 'desc' ? -cmp : cmp;
    });

    return rows;
  }, [filteredRecords, runNameMap, sortField, sortDir]);

  // If a run is selected, show full-page inline review
  if (activeRunId) {
    return (
      <div className="h-full flex flex-col -m-4 sm:-m-6 lg:-m-8">
        <StagingReviewContent
          workflowRunId={activeRunId}
          workflowName={runNameMap[activeRunId]}
          onBack={clearRun}
          alwaysEnabled
        />
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading staging records...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4 max-w-[1400px] mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-lg font-semibold">Staging Review</h2>
          <p className="text-sm text-muted-foreground">
            Review and approve records extracted by workflows before they are committed to the CRM.
          </p>
        </div>
        <div className="flex items-center gap-3">
          {pendingRecords.length > 0 && (
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Badge variant="outline">{totalPending} pending</Badge>
              {totalApproved > 0 && <Badge variant="outline" className="text-green-600 border-green-200">{totalApproved} approved</Badge>}
              {totalDuplicates > 0 && <Badge variant="outline" className="text-amber-600 border-amber-200">{totalDuplicates} duplicates</Badge>}
              <span className="text-muted-foreground/50">|</span>
              <span>{grouped.length} run{grouped.length !== 1 ? 's' : ''}</span>
            </div>
          )}
          {/* View toggle */}
          {pendingRecords.length > 0 && (
            <div className="flex items-center border rounded-md">
              <button
                onClick={setCardsView}
                className={`p-1.5 rounded-l-md transition-colors ${viewMode === 'cards' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
                title="Card view"
              >
                <LayoutGrid className="h-3.5 w-3.5" />
              </button>
              <button
                onClick={setTableView}
                className={`p-1.5 rounded-r-md transition-colors ${viewMode === 'table' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted text-muted-foreground'}`}
                title="Table view"
              >
                <TableProperties className="h-3.5 w-3.5" />
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Global batch action bar */}
      {pendingRecords.length > 0 && (
        <div className="flex items-center gap-2 p-3 rounded-lg border bg-muted/30">
          <span className="text-xs text-muted-foreground mr-auto">Batch actions across all runs:</span>
          {totalDuplicates > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1 text-amber-600 border-amber-200 hover:bg-amber-50"
              onClick={handleRejectDups}
              disabled={batchRejectDupsMutation.isPending}
            >
              <XCircle className="h-3 w-3" />
              {batchRejectDupsMutation.isPending ? 'Removing...' : `Reject ${totalDuplicates} duplicates`}
            </Button>
          )}
          {pendingNonDuplicate.length > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs gap-1 text-green-600 border-green-200 hover:bg-green-50"
              onClick={handleApproveAll}
              disabled={batchApproveMutation.isPending}
            >
              <CheckCheck className="h-3 w-3" />
              {batchApproveMutation.isPending ? 'Approving & committing...' : `Approve & commit ${pendingNonDuplicate.length} valid`}
            </Button>
          )}
        </div>
      )}

      {grouped.length === 0 ? (
        <div className="text-center py-16 text-muted-foreground">
          <ClipboardCheck className="h-10 w-10 mx-auto mb-3 opacity-30" />
          <p className="text-sm font-medium">No pending records</p>
          <p className="text-xs mt-1">Records extracted by workflow runs will appear here for review.</p>
        </div>
      ) : viewMode === 'cards' ? (
        <div className="grid gap-3 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3">
          {grouped.map((group) => {
            const statuses = group.records.reduce<Record<string, number>>((acc, r) => {
              acc[r.status] = (acc[r.status] || 0) + 1;
              return acc;
            }, {});
            const targetTypes = group.records.reduce<Record<string, number>>((acc, r) => {
              acc[r.target_type] = (acc[r.target_type] || 0) + 1;
              return acc;
            }, {});
            const typeLabels: Record<string, string> = {
              crm_contact: 'contacts',
              company: 'companies',
              crm_deal: 'deals',
              task: 'tasks',
            };
            return (
              <Card
                key={group.runId}
                className="card-interactive cursor-pointer"
                onClick={() => selectRun(group.runId)}
              >
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-sm">
                      {group.workflowName || 'Workflow Run'}
                    </CardTitle>
                    <Badge variant="outline" className="text-[10px]">
                      {group.records.length} pending
                    </Badge>
                  </div>
                  <div className="flex flex-wrap gap-1 mt-1">
                    {Object.entries(targetTypes).map(([type, count]) => (
                      <Badge key={type} variant="secondary" className="text-[9px]">
                        {count} {typeLabels[type] || type}
                      </Badge>
                    ))}
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="flex items-center justify-between">
                    <div className="flex flex-wrap gap-1">
                      {Object.entries(statuses).map(([status, count]) => {
                        const statusColors: Record<string, string> = {
                          pending: 'text-amber-600',
                          approved: 'text-green-600',
                          rejected: 'text-red-600',
                        };
                        return (
                          <span key={status} className={`text-[10px] ${statusColors[status] || 'text-muted-foreground'}`}>
                            {count} {status}
                          </span>
                        );
                      })}
                    </div>
                    <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      ) : (
        /* Global table view — all records across all runs */
        <div className="border rounded-lg overflow-hidden">
          {/* Filter toolbar */}
          <div className="flex items-center gap-2 px-4 py-2 border-b bg-muted/20 flex-wrap">
            {/* Status filter tabs */}
            <div className="flex items-center gap-0.5 mr-2">
              {([
                { key: 'all' as const, label: 'All', count: pendingRecords.length, color: '' },
                { key: 'valid' as const, label: 'Valid', count: pendingNonDuplicate.length, color: '' },
                { key: 'duplicates' as const, label: 'Dups', count: totalDuplicates, color: 'text-amber-600' },
                { key: 'issues' as const, label: 'Issues', count: totalWithIssues, color: 'text-amber-600' },
                { key: 'approved' as const, label: 'Approved', count: totalApproved, color: 'text-green-600' },
                { key: 'rejected' as const, label: 'Rejected', count: totalRejected, color: 'text-red-600' },
                { key: 'error' as const, label: 'Errors', count: totalError, color: 'text-red-600' },
              ]).map(({ key, label, count, color }) => {
                if (count === 0 && key !== 'all') return null;
                return (
                  <button
                    key={key}
                    onClick={() => updateParams({ filter: key })}
                    className={`px-2 py-0.5 rounded text-[11px] transition-colors ${
                      globalFilter === key
                        ? 'bg-primary text-primary-foreground'
                        : `hover:bg-muted text-muted-foreground ${color && globalFilter !== key ? color : ''}`
                    }`}
                  >
                    {label} ({count})
                  </button>
                );
              })}
            </div>

            <div className="h-4 w-px bg-border" />

            {/* Type filter */}
            {uniqueTypes.length > 1 && (
              <select
                value={typeFilter}
                onChange={(e) => updateParams({ type: e.target.value })}
                className="h-6 text-[11px] rounded border bg-background px-1.5 text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="all">All types</option>
                {uniqueTypes.map(type => (
                  <option key={type} value={type}>
                    {STAGING_TARGET_CONFIG[type]?.label || type}
                  </option>
                ))}
              </select>
            )}

            {/* Workflow filter */}
            {uniqueWorkflows.length > 1 && (
              <select
                value={workflowFilter}
                onChange={(e) => updateParams({ wf: e.target.value })}
                className="h-6 text-[11px] rounded border bg-background px-1.5 text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring max-w-[200px]"
              >
                <option value="all">All workflows</option>
                {uniqueWorkflows.map(([id, name]) => (
                  <option key={id} value={id}>{name}</option>
                ))}
              </select>
            )}

            <div className="flex-1" />
            <span className="text-[10px] text-muted-foreground">{filteredRecords.length} record{filteredRecords.length !== 1 ? 's' : ''}</span>
          </div>

          {/* Common warnings banner — collapse identical warnings */}
          {viewMode === 'table' && commonWarnings.length > 0 && (
            <div className="px-4 py-2 border-b bg-amber-50/60 dark:bg-amber-950/15 flex items-start gap-2">
              <AlertTriangle className="h-3.5 w-3.5 text-amber-500 shrink-0 mt-0.5" />
              <div className="text-[11px] text-amber-700 dark:text-amber-400">
                {commonWarnings.map(w => (
                  <div key={w.msg}>{w.msg} <span className="text-amber-500">({w.count} records)</span></div>
                ))}
              </div>
            </div>
          )}

          {/* Sortable table header */}
          <div className="grid grid-cols-[20px_28px_minmax(120px,1fr)_minmax(100px,0.7fr)_80px_60px_60px_minmax(160px,1.5fr)_100px] gap-2 px-4 py-1.5 text-[10px] font-medium text-muted-foreground uppercase tracking-wider border-b bg-muted/10">
            <div />
            <div />
            <button onClick={() => handleSort('name')} className="flex items-center gap-1 hover:text-foreground text-left">
              Name {sortField === 'name' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('workflow')} className="flex items-center gap-1 hover:text-foreground text-left">
              Workflow {sortField === 'workflow' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('type')} className="flex items-center gap-1 hover:text-foreground text-left">
              Type {sortField === 'type' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('status')} className="flex items-center gap-1 hover:text-foreground text-left">
              Status {sortField === 'status' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <button onClick={() => handleSort('confidence')} className="flex items-center gap-1 hover:text-foreground text-left">
              Conf. {sortField === 'confidence' ? (sortDir === 'asc' ? <ArrowUp className="h-2.5 w-2.5" /> : <ArrowDown className="h-2.5 w-2.5" />) : <ArrowUpDown className="h-2.5 w-2.5 opacity-30" />}
            </button>
            <div>Details</div>
            <div className="text-right">Actions</div>
          </div>

          {/* Table rows */}
          {tableRows.length === 0 && (
            <div className="text-center py-8 text-muted-foreground text-sm">
              No records match the selected filter.
            </div>
          )}
          {tableRows.map(({ record, displayName, data, workflowName }) => {
            const config = STAGING_TARGET_CONFIG[record.target_type];
            const Icon = config?.icon || Users;
            const isExpanded = expandedId === record.id;

            const detailFields = Object.entries(data)
              .filter(([k, v]) => v != null && !['first_name', 'last_name', 'name', 'title'].includes(k))
              .slice(0, 3);

            const allFields = Object.entries(data).filter(([, v]) => v != null && v !== '');

            let validationErrs: string[] = [];
            if (record.validation_errors) {
              try { validationErrs = JSON.parse(record.validation_errors); } catch {}
            }

            return (
              <div key={record.id}>
                <div
                  role="row"
                  tabIndex={0}
                  onClick={() => handleToggleExpand(record.id)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter' || e.key === ' ') {
                      e.preventDefault();
                      handleToggleExpand(record.id);
                    }
                  }}
                  className={`grid grid-cols-[20px_28px_minmax(120px,1fr)_minmax(100px,0.7fr)_80px_60px_60px_minmax(160px,1.5fr)_100px] gap-2 px-4 py-1.5 items-center border-b text-xs hover:bg-muted/30 transition-colors cursor-pointer select-none focus:outline-none focus:ring-1 focus:ring-ring focus:ring-inset ${
                    record.status === 'approved' ? 'bg-green-50/30 dark:bg-green-950/10' : ''
                  } ${record.status === 'rejected' ? 'bg-red-50/30 dark:bg-red-950/10 opacity-50' : ''
                  } ${record.duplicate_of_id ? 'bg-amber-50/20 dark:bg-amber-950/5' : ''
                  } ${isExpanded ? 'bg-muted/40 border-b-0' : ''}`}
                >
                  <ChevronRight className={`h-3.5 w-3.5 text-muted-foreground transition-transform ${isExpanded ? 'rotate-90' : ''}`} />
                  <Icon className={`h-3.5 w-3.5 ${config?.color || 'text-muted-foreground'}`} />

                  <div className="flex items-center gap-1.5 min-w-0">
                    <span className="font-medium truncate">{displayName}</span>
                    {record.duplicate_of_id && (
                      <span title="Potential duplicate"><AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" /></span>
                    )}
                    {validationErrs.length > 0 && !record.duplicate_of_id && (
                      <span title={`${validationErrs.length} issue(s)`}><AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" /></span>
                    )}
                  </div>

                  <span className="text-[10px] text-muted-foreground truncate">{workflowName || record.workflow_run_id.slice(0, 8)}</span>
                  <span className="text-[10px] text-muted-foreground">{config?.label || record.target_type}</span>

                  <Badge
                    variant="outline"
                    className={`text-[9px] h-5 px-1.5 justify-center ${
                      record.status === 'pending_review' ? 'text-amber-600 border-amber-200'
                        : record.status === 'approved' ? 'text-green-600 border-green-200'
                        : record.status === 'rejected' ? 'text-red-600 border-red-200'
                        : 'text-blue-600 border-blue-200'
                    }`}
                  >
                    {record.status === 'pending_review' ? 'pending' : record.status}
                  </Badge>

                  {record.confidence != null ? (
                    <Badge variant="outline" className={`text-[10px] ${
                      record.confidence >= 0.8 ? 'text-green-600 border-green-200'
                        : record.confidence >= 0.5 ? 'text-amber-600 border-amber-200'
                        : 'text-red-600 border-red-200'
                    }`}>
                      {Math.round(record.confidence * 100)}%
                    </Badge>
                  ) : <div />}

                  <div className="flex items-center gap-3 min-w-0 overflow-hidden">
                    {detailFields.map(([key, value]) => {
                      const dv = typeof value === 'object' ? JSON.stringify(value) : String(value);
                      return (
                        <span key={key} className="text-[10px] text-muted-foreground truncate">
                          <span className="opacity-60">{key}:</span> {dv}
                        </span>
                      );
                    })}
                  </div>

                  <div className="flex items-center gap-0.5 justify-end" onClick={handleStopPropagation}>
                    {record.status === 'pending_review' && (
                      <>
                        <button onClick={() => handleApproveRecord(record.id)} className="p-1 rounded hover:bg-green-100 dark:hover:bg-green-900" title="Approve & Commit">
                          <Check className="h-3 w-3 text-green-600" />
                        </button>
                        <button onClick={() => handleRejectRecord(record.id)} className="p-1 rounded hover:bg-red-100 dark:hover:bg-red-900" title="Reject">
                          <X className="h-3 w-3 text-red-500" />
                        </button>
                      </>
                    )}
                    {record.status === 'error' && (
                      <button onClick={() => handleRetryRecord(record.id)} className="p-1 rounded hover:bg-amber-100 dark:hover:bg-amber-900" title="Retry" disabled={retryMutation.isPending}>
                        <RotateCcw className="h-3 w-3 text-amber-600" />
                      </button>
                    )}
                  </div>
                </div>

                {/* Expanded detail panel with inline editing */}
                {isExpanded && (
                  <div className="border-b bg-muted/20 px-4 py-3" onClick={handleStopPropagation}>
                    <div className="grid grid-cols-[1fr_1fr] lg:grid-cols-[1fr_1fr_1fr] gap-x-6 gap-y-2">
                      {allFields.map(([key, value]) => {
                        const dv = typeof value === 'object' ? JSON.stringify(value, null, 2) : String(value);
                        const isLong = dv.length > 80;
                        const canEdit = record.status === 'pending_review';
                        return (
                          <div key={key} className={isLong ? 'col-span-2 lg:col-span-3' : ''}>
                            <dt className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide">{key.replace(/_/g, ' ')}</dt>
                            <InlineEditField
                              fieldKey={key}
                              value={dv}
                              recordId={record.id}
                              onSave={handleInlineFieldSave}
                              editable={canEdit}
                            />
                          </div>
                        );
                      })}
                    </div>

                    <div className="flex items-center gap-4 mt-3 pt-2 border-t border-muted text-[10px] text-muted-foreground">
                      <span>ID: <code className="text-[9px]">{record.id.slice(0, 8)}</code></span>
                      <span>Run: <code className="text-[9px]">{record.workflow_run_id.slice(0, 8)}</code></span>
                      {workflowName && <span>Workflow: {workflowName}</span>}
                      {record.duplicate_of_id && (
                        <span className="text-amber-600">Duplicate of: <code className="text-[9px]">{record.duplicate_of_id.slice(0, 8)}</code></span>
                      )}
                      {record.created_at && <span>Created: {new Date(record.created_at).toLocaleString()}</span>}
                    </div>

                    {validationErrs.length > 0 && (
                      <div className="mt-2 pt-2 border-t border-amber-200 dark:border-amber-800">
                        <p className="text-[10px] font-medium text-amber-600 mb-1">Validation Issues</p>
                        <ul className="space-y-0.5">
                          {validationErrs.map((err, i) => (
                            <li key={i} className="text-[10px] text-amber-600 flex items-start gap-1.5">
                              <AlertTriangle className="h-3 w-3 shrink-0 mt-0.5" />
                              {err}
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                )}

                {/* Compact inline warnings when NOT expanded — skip common warnings shown in banner */}
                {!isExpanded && validationErrs.filter(e => !commonWarningSet.has(e)).length > 0 && record.status !== 'rejected' && (
                  <div className="px-4 py-1 bg-amber-50/50 dark:bg-amber-950/10 border-b flex items-center gap-1.5">
                    <AlertTriangle className="h-3 w-3 text-amber-500 shrink-0" />
                    <span className="text-[10px] text-amber-600">{validationErrs.filter(e => !commonWarningSet.has(e)).join(' · ')}</span>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

function RunsTab() {
  const navigate = useNavigate();

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
            <Card
              key={run.id}
              className={`border-l-4 ${run.status === 'completed' ? 'border-l-green-500' : run.status === 'failed' ? 'border-l-red-500' : 'border-l-yellow-500'}`}
            >
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
                    {run.records_staged != null && (
                      <span className={run.records_staged > 0 ? 'text-foreground font-medium' : ''}>
                        {run.records_staged} records
                      </span>
                    )}
                    <span>{new Date(run.started_at).toLocaleString()}</span>
                    {run.records_staged > 0 && (
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-6 text-[10px] gap-1 ml-1"
                        onClick={() => navigate(`/workflows?tab=staging&run=${run.id}`)}
                      >
                        <ClipboardCheck className="h-3 w-3" />
                        Review Staging
                      </Button>
                    )}
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
