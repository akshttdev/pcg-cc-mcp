import { useState, useCallback } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Separator } from '@/components/ui/separator';
import {
  Activity,
  Bot,
  LayoutGrid,
  Clock,
  Cpu,
  RefreshCw,
  MessageSquare,
  ArrowLeftRight,
  Shield,
} from 'lucide-react';
import {
  AgentCard,
  AgentCardSkeleton,
  ArtifactPanel,
  ExecutionTimeline,
  LiveCommsPanel,
} from '@/components/mission-control';
import type { CommEvent } from '@/components/mission-control/LiveCommsPanel';
import {
  useMissionControlDashboard,
  useExecutionArtifacts,
} from '@/hooks/useMissionControl';
import { useExecutionEvents } from '@/hooks/useExecutionEvents';
import { useEventStream } from '@/lib/event-stream';
import type { AgentFlowEvent } from '@/lib/event-stream';
import { useAuth } from '@/contexts/AuthContext';
import { SlotUtilizationBadge, CompactSlotIndicator } from '@/components/parallel-execution';
import {
  ExecutionControlPanel,
  HandoffTimeline,
  ContextInjectionPanel,
} from '@/components/collaboration';
import {
  CheckpointReviewPanel,
  PendingApprovalsIndicator,
} from '@/components/autonomy';

/** Map an AgentFlowEvent to a CommEvent for the LiveCommsPanel */
function mapFlowEventToCommEvent(event: AgentFlowEvent): CommEvent {
  let eventData: Record<string, unknown> = {};
  try {
    eventData = JSON.parse(event.event_data);
  } catch { /* non-JSON event_data is fine */ }

  const eventType = event.event_type.toLowerCase();
  let type: CommEvent['type'] = 'system_event';
  let actor = (eventData.agent_codename as string) ?? (eventData.agent as string) ?? 'System';
  let message = (eventData.message as string) ?? (eventData.summary as string) ?? event.event_type;

  if (eventType.includes('execution_started') || eventType.includes('stage_started')) {
    type = 'agent_message';
    message = (eventData.stage_name as string)
      ? `Starting stage: ${eventData.stage_name}`
      : `Execution started${eventData.workflow_name ? ` — ${eventData.workflow_name}` : ''}`;
  } else if (eventType.includes('stage_completed')) {
    type = 'checkpoint';
    message = `Completed stage: ${(eventData.stage_name as string) ?? 'unknown'}`;
  } else if (eventType.includes('execution_completed')) {
    type = 'checkpoint';
    message = `Execution completed${eventData.duration_ms ? ` in ${((eventData.duration_ms as number) / 1000).toFixed(1)}s` : ''}`;
  } else if (eventType.includes('execution_failed')) {
    type = 'system_event';
    message = `Execution failed: ${(eventData.error as string) ?? 'unknown error'}`;
  } else if (eventType.includes('task_created') || eventType.includes('artifact')) {
    type = 'plan_update';
    message = (eventData.title as string) ?? (eventData.artifact_type as string) ?? event.event_type;
  } else if (eventType.includes('approval') || eventType.includes('human')) {
    type = 'human_action';
    actor = (eventData.user_name as string) ?? 'Human';
  } else if (eventType.includes('agent_message') || eventType.includes('chat')) {
    type = 'agent_message';
  }

  return {
    id: event.id,
    timestamp: new Date(event.created_at),
    type,
    actor,
    message,
    metadata: eventData,
  };
}

const MAX_COMM_EVENTS = 200;

export default function MissionControlPage() {
  const { data: dashboard, isLoading, refetch } = useMissionControlDashboard();
  const [selectedExecutionId, setSelectedExecutionId] = useState<string | null>(null);
  const { data: artifacts = [] } = useExecutionArtifacts(selectedExecutionId ?? undefined);
  const { user } = useAuth();

  // Real-time execution events from SSE/WebSocket
  const { activeExecutions, completedExecutions, connected: eventsConnected, activeCount } = useExecutionEvents();

  // Real-time flow events from SSE stream → mapped to CommEvents for LiveCommsPanel
  const [commEvents, setCommEvents] = useState<CommEvent[]>([]);
  const onFlowEvents = useCallback((events: AgentFlowEvent[]) => {
    setCommEvents((prev) => {
      const mapped = events.map(mapFlowEventToCommEvent);
      const combined = [...prev, ...mapped];
      return combined.slice(-MAX_COMM_EVENTS);
    });
  }, []);
  useEventStream({ onEvents: onFlowEvents });

  const selectedExecution = dashboard?.active_executions.find(
    (e) => e.process.id === selectedExecutionId
  );

  // Combine dashboard active count with real-time execution count
  const totalActiveCount = (dashboard?.total_active ?? 0) + activeCount;

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="relative border-b border-border/40 bg-card/50 backdrop-blur-sm overflow-hidden">
        <div className="ambient-glow -top-48 -right-32" />
        <div className="page-header px-4 sm:px-6 lg:px-8 py-4 max-w-[1600px] mx-auto relative">
          <div className="flex items-center gap-3">
            <div className="section-header-icon">
              <Activity className="h-5 w-5" />
            </div>
            <div>
              <h1 className="page-title">Mission Control</h1>
              <p className="page-description">
                Monitor and coordinate active agent executions
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 sm:gap-3 flex-wrap">
            <PendingApprovalsIndicator />

            <div className="hidden sm:flex items-center gap-4 text-sm">
              <div className="flex items-center gap-2">
                <Bot className="h-4 w-4 text-muted-foreground" />
                <span className="font-semibold">{totalActiveCount}</span>
                <span className="text-muted-foreground">Active</span>
              </div>
              <Separator orientation="vertical" className="h-5" />
              <div className="flex items-center gap-2">
                <Cpu className="h-4 w-4 text-muted-foreground" />
                <span className="text-muted-foreground">
                  {dashboard?.by_project.length ?? 0} Projects
                </span>
              </div>
            </div>

            <div className="flex items-center gap-2 px-2.5 py-1 rounded-md bg-surface-2">
              <div className={`status-dot ${eventsConnected ? 'status-dot-online' : 'status-dot-error'}`} />
              <span className="text-muted-foreground text-xs font-medium">
                {eventsConnected ? 'Live' : 'Disconnected'}
              </span>
            </div>

            <Button variant="outline" size="sm" onClick={() => refetch()}>
              <RefreshCw className="h-3.5 w-3.5 sm:mr-2" />
              <span className="hidden sm:inline">Refresh</span>
            </Button>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-hidden p-4 sm:p-6 lg:p-8 max-w-[1600px] mx-auto w-full">
        <div className="h-full grid grid-cols-1 md:grid-cols-2 xl:grid-cols-12 gap-4 sm:gap-6">
          {/* Left panel - Active agents */}
          <div className="xl:col-span-3 md:col-span-1 flex flex-col overflow-hidden">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-sm font-medium flex items-center gap-2">
                <Bot className="h-4 w-4" />
                Active Agents
              </h2>
              {dashboard?.total_active !== undefined && (
                <Badge variant="secondary">{dashboard.total_active}</Badge>
              )}
            </div>

            <div className="flex-1 overflow-auto space-y-3 pr-2 animate-stagger">
              {isLoading ? (
                <>
                  <AgentCardSkeleton />
                  <AgentCardSkeleton />
                  <AgentCardSkeleton />
                </>
              ) : dashboard?.active_executions.length === 0 ? (
                <Card className="border-dashed">
                  <CardContent className="py-8 text-center text-muted-foreground text-sm">
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

          {/* Center panel - Timeline and details */}
          <div className="xl:col-span-6 md:col-span-1 flex flex-col overflow-hidden">
            <Tabs defaultValue="timeline" className="h-full flex flex-col">
              <TabsList className="w-fit">
                <TabsTrigger value="timeline" className="gap-2">
                  <Clock className="h-4 w-4" />
                  Timeline
                </TabsTrigger>
                <TabsTrigger value="workflows" className="gap-2">
                  <Activity className="h-4 w-4" />
                  Workflows
                  {activeCount > 0 && (
                    <Badge variant="secondary" className="ml-1 h-5 px-1.5">
                      {activeCount}
                    </Badge>
                  )}
                </TabsTrigger>
                <TabsTrigger value="grid" className="gap-2">
                  <LayoutGrid className="h-4 w-4" />
                  Grid
                </TabsTrigger>
              </TabsList>

              <TabsContent value="timeline" className="flex-1 mt-4 overflow-hidden">
                <Card className="h-full">
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium">
                      Execution Timeline
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="overflow-auto">
                    <ExecutionTimeline
                      executions={dashboard?.active_executions ?? []}
                    />
                  </CardContent>
                </Card>
              </TabsContent>

              <TabsContent value="workflows" className="flex-1 mt-4 overflow-hidden">
                <Card className="h-full">
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm font-medium">
                      Agent Workflows
                    </CardTitle>
                  </CardHeader>
                  <CardContent className="overflow-auto space-y-4">
                    {activeExecutions.length === 0 && completedExecutions.length === 0 && (dashboard?.active_workflows?.length ?? 0) === 0 ? (
                      <div className="text-center py-8 text-muted-foreground">
                        <Activity className="h-8 w-8 mx-auto mb-2 opacity-50" />
                        <p>No workflow executions yet</p>
                        <p className="text-xs mt-1">Ask Nora to run a workflow to see activity here</p>
                      </div>
                    ) : (
                      <>
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
                                  {exec.error && (
                                    <div className="mt-2 text-xs text-destructive">{exec.error}</div>
                                  )}
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
                                    <span className="text-xs">
                                      {new Date(workflow.flow.updated_at).toLocaleTimeString()}
                                    </span>
                                  </div>
                                </div>
                              ))}
                            </div>
                          </div>
                        )}
                      </>
                    )}
                  </CardContent>
                </Card>
              </TabsContent>

              <TabsContent value="grid" className="flex-1 mt-4 overflow-hidden">
                <div className="grid grid-cols-2 gap-4 h-full overflow-auto animate-stagger">
                  {dashboard?.by_project.map((project) => (
                    <Card key={project.project_id}>
                      <CardHeader className="pb-2">
                        <div className="flex items-center justify-between">
                          <CardTitle className="text-sm font-medium">
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
                      <CardContent>
                        <div className="space-y-2">
                          <div className="flex items-center justify-between text-sm">
                            <span className="text-muted-foreground">
                              Active Executions
                            </span>
                            <span className="font-medium">
                              {project.active_count}
                            </span>
                          </div>
                          <CompactSlotIndicator
                            projectId={project.project_id}
                          />
                          <div className="text-xs text-muted-foreground">
                            Browser slots: {project.capacity.active_browser_slots}/
                            {project.capacity.max_concurrent_browser_agents}
                          </div>
                        </div>
                      </CardContent>
                    </Card>
                  ))}
                </div>
              </TabsContent>
            </Tabs>
          </div>

          {/* Right panel - Control, Artifacts and Collaboration */}
          <div className="xl:col-span-3 md:col-span-2 xl:col-auto flex flex-col gap-4 overflow-hidden">
            {selectedExecution ? (
              <>
                {/* Execution Control Panel */}
                <ExecutionControlPanel
                  executionId={selectedExecution.process.id}
                  currentUserId={user?.id ?? 'anonymous'}
                  currentUserName={user?.full_name ?? user?.username ?? 'You'}
                />

                {/* Artifacts panel */}
                <div className="flex-1 min-h-0">
                  <ArtifactPanel artifacts={artifacts} className="h-full" />
                </div>

                {/* Collaboration tabs */}
                <div className="h-72">
                  <Tabs defaultValue="checkpoints" className="h-full flex flex-col">
                    <TabsList className="w-fit">
                      <TabsTrigger value="checkpoints" className="gap-2">
                        <Shield className="h-3 w-3" />
                        Checkpoints
                      </TabsTrigger>
                      <TabsTrigger value="injections" className="gap-2">
                        <MessageSquare className="h-3 w-3" />
                        Injections
                      </TabsTrigger>
                      <TabsTrigger value="handoffs" className="gap-2">
                        <ArrowLeftRight className="h-3 w-3" />
                        Handoffs
                      </TabsTrigger>
                      <TabsTrigger value="comms" className="gap-2">
                        <Activity className="h-3 w-3" />
                        Live
                      </TabsTrigger>
                    </TabsList>

                    <TabsContent value="checkpoints" className="flex-1 mt-2 overflow-hidden">
                      <CheckpointReviewPanel
                        executionId={selectedExecution.process.id}
                        currentUserId={user?.id ?? 'anonymous'}
                        currentUserName={user?.full_name ?? user?.username ?? 'You'}
                        className="h-full"
                      />
                    </TabsContent>

                    <TabsContent value="injections" className="flex-1 mt-2 overflow-hidden">
                      <ContextInjectionPanel
                        executionId={selectedExecution.process.id}
                        className="h-full"
                      />
                    </TabsContent>

                    <TabsContent value="handoffs" className="flex-1 mt-2 overflow-hidden">
                      <HandoffTimeline
                        executionId={selectedExecution.process.id}
                        className="h-full"
                      />
                    </TabsContent>

                    <TabsContent value="comms" className="flex-1 mt-2 overflow-hidden">
                      <LiveCommsPanel events={commEvents} className="h-full" />
                    </TabsContent>
                  </Tabs>
                </div>
              </>
            ) : (
              <>
                {/* Empty state when no execution selected */}
                <Card className="flex-1">
                  <CardContent className="h-full flex items-center justify-center text-muted-foreground text-sm">
                    Select an execution to view details
                  </CardContent>
                </Card>

                {/* Live coordination panel - always visible */}
                <div className="h-64">
                  <LiveCommsPanel events={commEvents} className="h-full" />
                </div>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
