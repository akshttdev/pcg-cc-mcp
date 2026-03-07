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
} from 'lucide-react';
import { agentFlowsApi } from '@/lib/api';

interface AgentFlow {
  id: string;
  flow_type: string;
  status: string;
  current_phase: string;
  created_at: string;
  task_id?: string;
}

interface AutomationDefinition {
  id: string;
  name: string;
  description: string;
  trigger: string;
  action: string;
  schedule: string;
}

export function WorkflowsPage() {
  const [selectedExecution, setSelectedExecution] = useState<ActiveExecution | null>(null);
  const [activeTab, setActiveTab] = useState('active');
  const [agentFlows, setAgentFlows] = useState<AgentFlow[]>([]);
  const [automations, setAutomations] = useState<AutomationDefinition[]>([]);

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

    fetch('/api/automations', { credentials: 'include' })
      .then((r) => r.json())
      .then((res) => { if (res?.data) setAutomations(res.data); })
      .catch(() => {});
  }, []);

  const stats = {
    active: activeExecutions.length,
    completed: completedExecutions.filter((e) => e.status === 'completed').length,
    failed: completedExecutions.filter((e) => e.status === 'failed').length,
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
              <h1 className="page-title">Agent Workflows</h1>
              <p className="page-description">
                Monitor agent execution pipelines in real-time
              </p>
            </div>
          </div>
          <div className="flex items-center gap-2 sm:gap-3 flex-wrap">
            {/* Connection status */}
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
            {/* Stats badges */}
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
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="h-full flex flex-col"
        >
          <div className="border-b px-4 sm:px-6">
            <TabsList className="sm:w-fit grid grid-cols-4 sm:grid-cols-4 gap-1">
              <TabsTrigger value="active">
                Active
                {activeExecutions.length > 0 && (
                  <Badge variant="secondary" className="ml-2 animate-pulse">
                    {activeExecutions.length}
                  </Badge>
                )}
              </TabsTrigger>
              <TabsTrigger value="recent">
                Recent
                <Badge variant="secondary" className="ml-2">
                  {completedExecutions.length}
                </Badge>
              </TabsTrigger>
              <TabsTrigger value="flows">
                <History className="h-3.5 w-3.5 mr-1.5" />
                Agent Flows
                <Badge variant="secondary" className="ml-2">{agentFlows.length}</Badge>
              </TabsTrigger>
              <TabsTrigger value="automations">
                <Zap className="h-3.5 w-3.5 mr-1.5" />
                Automations
                <Badge variant="secondary" className="ml-2">{automations.length}</Badge>
              </TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="active" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {activeExecutions.length === 0 ? (
              <div className="empty-state h-full">
                <GitBranch className="empty-state-icon" />
                <p className="empty-state-title">No Active Workflows</p>
                <p className="empty-state-description">
                  Workflows will appear here when agents execute tasks.
                  Try asking Nora to run a workflow.
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
                          value={
                            exec.totalStages > 0
                              ? (exec.currentStage / exec.totalStages) * 100
                              : 0
                          }
                          className="h-1.5"
                        />
                        <div className="flex items-center justify-between text-xs text-muted-foreground">
                          <span>
                            Stage {exec.currentStage} of {exec.totalStages || '?'}
                          </span>
                          <span>Started: {formatTime(exec.startedAt)}</span>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          <TabsContent value="recent" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {completedExecutions.length === 0 ? (
              <div className="empty-state h-full">
                <Clock className="empty-state-icon" />
                <p className="empty-state-title">No Recent Workflows</p>
                <p className="empty-state-description">
                  Completed workflows will appear here
                </p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {completedExecutions.map((exec) => (
                  <Card
                    key={exec.executionId}
                    className={`card-interactive border-l-4 ${
                      exec.status === 'completed'
                        ? 'border-l-green-500'
                        : 'border-l-red-500'
                    }`}
                    onClick={() => setSelectedExecution(exec)}
                  >
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-base">{exec.agentCodename}</CardTitle>
                        <Badge
                          variant={exec.status === 'completed' ? 'default' : 'destructive'}
                        >
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
                          <div className="text-sm text-destructive">
                            {exec.error || 'Execution failed'}
                          </div>
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

          {/* Agent Flows tab */}
          <TabsContent value="flows" className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8">
            {agentFlows.length === 0 ? (
              <div className="empty-state h-full">
                <History className="empty-state-icon" />
                <p className="empty-state-title">No Agent Flows</p>
                <p className="empty-state-description">Agent execution flows will appear here</p>
              </div>
            ) : (
              <div className="grid gap-4 grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 animate-stagger">
                {agentFlows.map((flow) => (
                  <Card
                    key={flow.id}
                    className={`border-l-4 ${
                      flow.status === 'completed' ? 'border-l-green-500' :
                      flow.status === 'failed' ? 'border-l-red-500' :
                      'border-l-yellow-500'
                    }`}
                  >
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-sm capitalize">{flow.flow_type} flow</CardTitle>
                        <Badge
                          variant={
                            flow.status === 'completed' ? 'default' :
                            flow.status === 'failed' ? 'destructive' : 'outline'
                          }
                          className="text-xs capitalize"
                        >
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
                        <div className="text-xs text-muted-foreground pt-1 truncate">
                          {flow.id}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                ))}
              </div>
            )}
          </TabsContent>

          {/* Automations tab */}
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
                        <Badge variant="outline" className="text-xs gap-1">
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
            <SheetDescription>
              {selectedExecution?.workflowName}
            </SheetDescription>
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
                  <div className="font-medium">
                    {formatDuration(selectedExecution.durationMs)}
                  </div>
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
