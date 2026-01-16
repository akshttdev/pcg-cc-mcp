import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Separator } from '@/components/ui/separator';
import { NoraAssistant, NoraCoordinationPanel, NoraVoiceControls, NoraPlansPanel } from '@/components/nora';
import { Crown, Users, Mic, Settings, MessageSquare, Activity, RefreshCw, Zap, Shuffle, Bot, Cpu, Clock, LayoutGrid } from 'lucide-react';
import { toast } from 'sonner';
import {
  applyNoraMode,
  fetchNoraModes,
  NoraModeSummary,
  runRapidPlaybook,
  syncNoraContext,
} from '@/lib/api';
import {
  AgentCard,
  AgentCardSkeleton,
  ExecutionTimeline,
  LiveCommsPanel,
  generateMockEvents,
} from '@/components/mission-control';
import {
  useMissionControlDashboard,
} from '@/hooks/useMissionControl';
import { useExecutionEvents } from '@/hooks/useExecutionEvents';
import { SlotUtilizationBadge, CompactSlotIndicator } from '@/components/parallel-execution';
import { PendingApprovalsIndicator } from '@/components/autonomy';

export function NoraPage() {
  const [activeTab, setActiveTab] = useState('assistant');
  const [modes, setModes] = useState<NoraModeSummary[]>([]);
  const [isSyncing, setIsSyncing] = useState(false);
  const [isPlaybookRunning, setIsPlaybookRunning] = useState(false);
  const [isApplyingMode, setIsApplyingMode] = useState(false);
  const [selectedExecutionId, setSelectedExecutionId] = useState<string | null>(null);

  // Mission Control data
  const { data: dashboard, isLoading: dashboardLoading, refetch: refetchDashboard } = useMissionControlDashboard();
  const { activeExecutions, completedExecutions, connected: eventsConnected, activeCount } = useExecutionEvents();
  const [mockEvents] = useState(generateMockEvents);

  // Combine dashboard active count with real-time execution count
  const totalActiveCount = (dashboard?.total_active ?? 0) + activeCount;

  useEffect(() => {
    void (async () => {
      try {
        const data = await fetchNoraModes();
        setModes(data);
      } catch (error) {
        console.warn('Unable to preload Nora modes', error);
      }
    })();
  }, []);

  const handleSyncContext = useCallback(async () => {
    setIsSyncing(true);
    try {
      const result = await syncNoraContext();
      toast.success(`Synced ${result.projects_refreshed} projects into Nora's context`);
    } catch (error) {
      console.error(error);
      toast.error('Failed to sync Nora context');
    } finally {
      setIsSyncing(false);
    }
  }, []);

  const handleApplyMode = useCallback(async () => {
    const selection = window.prompt(
      `Enter mode id to apply (${modes.map((mode) => mode.id).join(', ') || 'rapid-builder/boardroom'})`
    );
    if (!selection) return;
    setIsApplyingMode(true);
    try {
      const response = await applyNoraMode(selection.trim(), true);
      toast.success(`Nora switched to ${response.active_mode}`);
    } catch (error) {
      console.error(error);
      toast.error('Unable to apply mode');
    } finally {
      setIsApplyingMode(false);
    }
  }, [modes]);

  const handleRapidPlaybook = useCallback(async () => {
    const project = window.prompt('Project or initiative name?');
    if (!project) return;
    const objectives = window.prompt('Comma separated objectives?') || '';
    setIsPlaybookRunning(true);
    try {
      const result = await runRapidPlaybook({
        project_name: project,
        objectives: objectives
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean),
      });
      toast.success('Rapid playbook ready');
      window.alert(result.summary);
    } catch (error) {
      console.error(error);
      toast.error('Playbook failed to run');
    } finally {
      setIsPlaybookRunning(false);
    }
  }, []);

  return (
    <div className="flex flex-col h-full bg-background">
      {/* Header */}
      <div className="border-b bg-white shadow-sm">
        <div className="flex items-center justify-between p-6">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-purple-100 rounded-lg">
              <Crown className="w-6 h-6 text-purple-600" />
            </div>
            <div>
              <h1 className="text-2xl font-bold text-gray-900">
                Nora Executive Assistant
              </h1>
              <p className="text-sm text-gray-600">
                Your AI-powered Executive Assistant and COO for organizational coordination
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <div className="text-right">
              <div className="text-sm font-medium text-green-600">Active</div>
              <div className="text-xs text-gray-500">British Executive Mode</div>
            </div>
            <div className="w-3 h-3 bg-green-500 rounded-full animate-pulse" />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 p-6">
        <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full">
          <TabsList className="grid w-full grid-cols-5 mb-6">
            <TabsTrigger value="assistant" className="flex items-center gap-2">
              <MessageSquare className="w-4 h-4" />
              Chat Assistant
            </TabsTrigger>
            <TabsTrigger value="coordination" className="flex items-center gap-2">
              <Users className="w-4 h-4" />
              Coordination
            </TabsTrigger>
            <TabsTrigger value="voice" className="flex items-center gap-2">
              <Mic className="w-4 h-4" />
              Voice Settings
            </TabsTrigger>
            <TabsTrigger value="analytics" className="flex items-center gap-2">
              <Activity className="w-4 h-4" />
              Analytics
            </TabsTrigger>
            <TabsTrigger value="plans" className="flex items-center gap-2">
              <Settings className="w-4 h-4" />
              Orchestration
            </TabsTrigger>
          </TabsList>

          <TabsContent value="assistant" className="h-full">
            <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 h-full">
              {/* Main Chat Interface */}
              <div className="lg:col-span-2">
                <NoraAssistant className="h-full" />
              </div>

              {/* Side Panel - Quick Actions & Context */}
              <div className="space-y-6">
                <Card>
                  <CardHeader>
                    <CardTitle className="text-lg">Quick Executive Actions</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <Button
                      variant="outline"
                      className="w-full justify-start"
                      onClick={handleSyncContext}
                      disabled={isSyncing}
                    >
                      <RefreshCw className="w-4 h-4 mr-2" />
                      {isSyncing ? 'Syncing live context…' : 'Sync Live Context'}
                    </Button>
                    <Button
                      variant="outline"
                      className="w-full justify-start"
                      onClick={handleRapidPlaybook}
                      disabled={isPlaybookRunning}
                    >
                      <Zap className="w-4 h-4 mr-2 text-amber-600" />
                      {isPlaybookRunning ? 'Running playbook…' : 'Rapid Prototype Playbook'}
                    </Button>
                    <Button
                      variant="outline"
                      className="w-full justify-start"
                      onClick={handleApplyMode}
                      disabled={isApplyingMode}
                    >
                      <Shuffle className="w-4 h-4 mr-2" />
                      {isApplyingMode ? 'Switching mode…' : 'Switch Nora Mode'}
                    </Button>
                    <Button variant="outline" className="w-full justify-start">
                      <Settings className="w-4 h-4 mr-2" />
                      Resource Allocation Brief
                    </Button>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle className="text-lg">Executive Context</CardTitle>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    <div className="text-sm">
                      <div className="font-medium text-gray-700">Current Focus</div>
                      <div className="text-gray-600">PowerClub Global coordination and strategic oversight</div>
                    </div>
                    <div className="text-sm">
                      <div className="font-medium text-gray-700">Active Projects</div>
                      <div className="text-gray-600">PCG Dashboard, Multi-agent coordination</div>
                    </div>
                    <div className="text-sm">
                      <div className="font-medium text-gray-700">Priority Level</div>
                      <div className="text-purple-600 font-medium">Executive</div>
                    </div>
                  </CardContent>
                </Card>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="coordination" className="h-full">
            <NoraCoordinationPanel className="h-full" />
          </TabsContent>

          <TabsContent value="voice" className="h-full">
            <div className="max-w-4xl">
              <NoraVoiceControls />
            </div>
          </TabsContent>

          <TabsContent value="analytics" className="h-full overflow-hidden">
            {/* Mission Control Dashboard */}
            <div className="h-full flex flex-col">
              {/* Status Bar */}
              <div className="flex items-center justify-between mb-4 px-1">
                <div className="flex items-center gap-6 text-sm">
                  <div className="flex items-center gap-2">
                    <Bot className="h-4 w-4 text-muted-foreground" />
                    <span className="font-medium">{totalActiveCount}</span>
                    <span className="text-muted-foreground">Active Agents</span>
                  </div>
                  <Separator orientation="vertical" className="h-5" />
                  <div className="flex items-center gap-2">
                    <Cpu className="h-4 w-4 text-muted-foreground" />
                    <span className="text-muted-foreground">
                      {dashboard?.by_project.length ?? 0} Projects
                    </span>
                  </div>
                  <Separator orientation="vertical" className="h-5" />
                  <div className="flex items-center gap-2">
                    <div className={`h-2 w-2 rounded-full ${eventsConnected ? 'bg-green-500' : 'bg-red-500'}`} />
                    <span className="text-muted-foreground text-xs">
                      {eventsConnected ? 'Live' : 'Disconnected'}
                    </span>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <PendingApprovalsIndicator />
                  <Button variant="outline" size="sm" onClick={() => refetchDashboard()}>
                    <RefreshCw className="h-4 w-4 mr-2" />
                    Refresh
                  </Button>
                </div>
              </div>

              {/* Main Grid */}
              <div className="flex-1 grid grid-cols-12 gap-4 overflow-hidden">
                {/* Left: Active Agents */}
                <div className="col-span-3 flex flex-col overflow-hidden">
                  <div className="flex items-center justify-between mb-3">
                    <h3 className="text-sm font-medium flex items-center gap-2">
                      <Bot className="h-4 w-4" />
                      Active Agents
                    </h3>
                    {dashboard?.total_active !== undefined && (
                      <Badge variant="secondary">{dashboard.total_active}</Badge>
                    )}
                  </div>
                  <div className="flex-1 overflow-auto space-y-2 pr-2">
                    {dashboardLoading ? (
                      <>
                        <AgentCardSkeleton />
                        <AgentCardSkeleton />
                      </>
                    ) : dashboard?.active_executions.length === 0 ? (
                      <Card className="border-dashed">
                        <CardContent className="py-6 text-center text-muted-foreground text-sm">
                          No active executions
                        </CardContent>
                      </Card>
                    ) : (
                      dashboard?.active_executions.map((execution) => (
                        <AgentCard
                          key={execution.process.id}
                          execution={execution}
                          isSelected={selectedExecutionId === execution.process.id}
                          onClick={() =>
                            setSelectedExecutionId(
                              selectedExecutionId === execution.process.id
                                ? null
                                : execution.process.id
                            )
                          }
                        />
                      ))
                    )}
                  </div>
                </div>

                {/* Center: Workflows & Timeline */}
                <div className="col-span-6 flex flex-col overflow-hidden">
                  <Tabs defaultValue="workflows" className="h-full flex flex-col">
                    <TabsList className="w-fit">
                      <TabsTrigger value="workflows" className="gap-2">
                        <Activity className="h-4 w-4" />
                        Workflows
                        {activeCount > 0 && (
                          <Badge variant="secondary" className="ml-1 h-5 px-1.5">
                            {activeCount}
                          </Badge>
                        )}
                      </TabsTrigger>
                      <TabsTrigger value="timeline" className="gap-2">
                        <Clock className="h-4 w-4" />
                        Timeline
                      </TabsTrigger>
                      <TabsTrigger value="projects" className="gap-2">
                        <LayoutGrid className="h-4 w-4" />
                        By Project
                      </TabsTrigger>
                    </TabsList>

                    <TabsContent value="workflows" className="flex-1 mt-3 overflow-hidden">
                      <Card className="h-full">
                        <CardContent className="p-4 overflow-auto h-full">
                          {activeExecutions.length === 0 && completedExecutions.length === 0 && (dashboard?.active_workflows?.length ?? 0) === 0 ? (
                            <div className="h-full flex items-center justify-center text-muted-foreground">
                              <div className="text-center">
                                <Activity className="h-8 w-8 mx-auto mb-2 opacity-50" />
                                <p>No workflow executions yet</p>
                                <p className="text-xs mt-1">Ask Nora to run a workflow to see activity here</p>
                              </div>
                            </div>
                          ) : (
                            <div className="space-y-4">
                              {activeExecutions.length > 0 && (
                                <div>
                                  <h4 className="text-xs font-medium text-muted-foreground mb-2">ACTIVE</h4>
                                  <div className="space-y-2">
                                    {activeExecutions.map((exec) => (
                                      <div key={exec.executionId} className="p-3 border rounded-lg bg-muted/50">
                                        <div className="flex items-center justify-between mb-2">
                                          <div className="flex items-center gap-2">
                                            <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                                            <span className="font-medium">{exec.agentCodename}</span>
                                          </div>
                                          <Badge variant="outline">{exec.workflowName ?? 'Custom'}</Badge>
                                        </div>
                                        <div className="text-sm text-muted-foreground">
                                          Stage {exec.currentStage + 1}/{exec.totalStages || '?'}: {exec.stageName}
                                        </div>
                                        {exec.totalStages > 0 && (
                                          <div className="mt-2 h-1.5 bg-muted rounded-full overflow-hidden">
                                            <div
                                              className="h-full bg-primary transition-all duration-500"
                                              style={{ width: `${((exec.currentStage + 1) / exec.totalStages) * 100}%` }}
                                            />
                                          </div>
                                        )}
                                      </div>
                                    ))}
                                  </div>
                                </div>
                              )}
                              {completedExecutions.length > 0 && (
                                <div>
                                  <h4 className="text-xs font-medium text-muted-foreground mb-2">RECENT</h4>
                                  <div className="space-y-2">
                                    {completedExecutions.slice(0, 5).map((exec) => (
                                      <div key={exec.executionId} className="p-3 border rounded-lg">
                                        <div className="flex items-center justify-between mb-1">
                                          <div className="flex items-center gap-2">
                                            <div className={`h-2 w-2 rounded-full ${exec.status === 'completed' ? 'bg-green-500' : 'bg-red-500'}`} />
                                            <span className="font-medium">{exec.agentCodename}</span>
                                          </div>
                                          <Badge variant={exec.status === 'completed' ? 'secondary' : 'destructive'}>
                                            {exec.status}
                                          </Badge>
                                        </div>
                                        <div className="text-sm text-muted-foreground flex items-center gap-4">
                                          <span>{exec.workflowName ?? 'Custom workflow'}</span>
                                          {exec.durationMs && (
                                            <span className="text-xs">
                                              {exec.durationMs < 1000
                                                ? `${exec.durationMs}ms`
                                                : `${(exec.durationMs / 1000).toFixed(1)}s`}
                                            </span>
                                          )}
                                          {exec.tasksCreated > 0 && (
                                            <span className="text-xs">{exec.tasksCreated} tasks</span>
                                          )}
                                        </div>
                                      </div>
                                    ))}
                                  </div>
                                </div>
                              )}
                              {/* Database-backed workflows */}
                              {dashboard?.active_workflows && dashboard.active_workflows.length > 0 && (
                                <div>
                                  <h4 className="text-xs font-medium text-muted-foreground mb-2">WORKFLOW HISTORY</h4>
                                  <div className="space-y-2">
                                    {dashboard.active_workflows.map((workflow) => (
                                      <div key={workflow.flow.id} className="p-3 border rounded-lg">
                                        <div className="flex items-center justify-between mb-1">
                                          <div className="flex items-center gap-2">
                                            <div className={`h-2 w-2 rounded-full ${
                                              workflow.flow.status === 'completed' ? 'bg-green-500' :
                                              workflow.flow.status === 'failed' ? 'bg-red-500' :
                                              'bg-yellow-500 animate-pulse'
                                            }`} />
                                            <span className="font-medium">{workflow.task_title}</span>
                                          </div>
                                          <Badge variant={
                                            workflow.flow.status === 'completed' ? 'secondary' :
                                            workflow.flow.status === 'failed' ? 'destructive' :
                                            'outline'
                                          }>
                                            {workflow.flow.status}
                                          </Badge>
                                        </div>
                                        <div className="text-sm text-muted-foreground flex items-center gap-4">
                                          {workflow.project_name && <span>{workflow.project_name}</span>}
                                          <span className="text-xs">{workflow.events_count} events</span>
                                        </div>
                                      </div>
                                    ))}
                                  </div>
                                </div>
                              )}
                            </div>
                          )}
                        </CardContent>
                      </Card>
                    </TabsContent>

                    <TabsContent value="timeline" className="flex-1 mt-3 overflow-hidden">
                      <Card className="h-full">
                        <CardHeader className="pb-2">
                          <CardTitle className="text-sm font-medium">Execution Timeline</CardTitle>
                        </CardHeader>
                        <CardContent className="overflow-auto">
                          <ExecutionTimeline executions={dashboard?.active_executions ?? []} />
                        </CardContent>
                      </Card>
                    </TabsContent>

                    <TabsContent value="projects" className="flex-1 mt-3 overflow-hidden">
                      <div className="grid grid-cols-2 gap-3 h-full overflow-auto">
                        {dashboard?.by_project.map((project) => (
                          <Card key={project.project_id}>
                            <CardHeader className="pb-2">
                              <div className="flex items-center justify-between">
                                <CardTitle className="text-sm font-medium truncate">
                                  {project.project_name}
                                </CardTitle>
                                <SlotUtilizationBadge
                                  capacity={{
                                    project_id: project.project_id,
                                    max_concurrent_agents: project.capacity.max_concurrent_agents,
                                    max_concurrent_browser_agents: project.capacity.max_concurrent_browser_agents,
                                    active_agent_slots: project.capacity.active_agent_slots,
                                    active_browser_slots: project.capacity.active_browser_slots,
                                    available_agent_slots: project.capacity.max_concurrent_agents - project.capacity.active_agent_slots,
                                    available_browser_slots: project.capacity.max_concurrent_browser_agents - project.capacity.active_browser_slots,
                                  }}
                                />
                              </div>
                            </CardHeader>
                            <CardContent className="pt-0">
                              <div className="space-y-2">
                                <div className="flex items-center justify-between text-sm">
                                  <span className="text-muted-foreground">Active</span>
                                  <span className="font-medium">{project.active_count}</span>
                                </div>
                                <CompactSlotIndicator projectId={project.project_id} />
                              </div>
                            </CardContent>
                          </Card>
                        ))}
                      </div>
                    </TabsContent>
                  </Tabs>
                </div>

                {/* Right: Live Feed */}
                <div className="col-span-3 flex flex-col overflow-hidden">
                  <h3 className="text-sm font-medium mb-3 flex items-center gap-2">
                    <Activity className="h-4 w-4" />
                    Live Activity
                  </h3>
                  <div className="flex-1 min-h-0">
                    <LiveCommsPanel events={mockEvents} className="h-full" />
                  </div>
                </div>
              </div>
            </div>
          </TabsContent>

          <TabsContent value="plans" className="h-full">
            <NoraPlansPanel />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
