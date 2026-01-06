import { useState } from 'react';
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
  Wifi,
  WifiOff,
  Clock,
} from 'lucide-react';

export function WorkflowsPage() {
  const [selectedExecution, setSelectedExecution] = useState<ActiveExecution | null>(null);
  const [activeTab, setActiveTab] = useState('active');

  // Real-time execution events from WebSocket
  const {
    activeExecutions,
    completedExecutions,
    connected,
    connectionMode,
  } = useExecutionEvents({ maxHistory: 50 });

  // Stats
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

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b px-6 py-4">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold">Agent Workflows</h1>
            <p className="text-muted-foreground text-sm mt-1">
              Monitor agent execution pipelines in real-time
            </p>
          </div>
          <div className="flex items-center gap-4">
            {/* Connection status */}
            <div className="flex items-center gap-2">
              {connected ? (
                <Badge variant="outline" className="gap-1 text-green-600 border-green-300">
                  <Wifi className="h-3 w-3" />
                  Live ({connectionMode})
                </Badge>
              ) : (
                <Badge variant="outline" className="gap-1 text-red-600 border-red-300">
                  <WifiOff className="h-3 w-3" />
                  Disconnected
                </Badge>
              )}
            </div>
            {/* Stats badges */}
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="gap-1 text-yellow-600 border-yellow-300">
                <Play className="h-3 w-3" />
                {stats.active} Active
              </Badge>
              <Badge variant="outline" className="gap-1 text-green-600 border-green-300">
                <CheckCircle2 className="h-3 w-3" />
                {stats.completed} Completed
              </Badge>
              {stats.failed > 0 && (
                <Badge variant="destructive" className="gap-1">
                  <AlertCircle className="h-3 w-3" />
                  {stats.failed} Failed
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
          <div className="border-b px-6">
            <TabsList>
              <TabsTrigger value="active">
                Active Executions
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
            </TabsList>
          </div>

          <TabsContent value="active" className="flex-1 overflow-auto p-4">
            {activeExecutions.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <GitBranch className="h-12 w-12 mb-4 opacity-50" />
                <p className="text-lg font-medium">No Active Workflows</p>
                <p className="text-sm mt-1">
                  Workflows will appear here when agents execute tasks
                </p>
                <p className="text-xs mt-4">
                  Try asking Nora: &quot;Scout, research competitors for [project]&quot;
                </p>
              </div>
            ) : (
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {activeExecutions.map((exec) => (
                  <Card
                    key={exec.executionId}
                    className="cursor-pointer hover:shadow-md transition-shadow border-l-4 border-l-yellow-500"
                    onClick={() => setSelectedExecution(exec)}
                  >
                    <CardHeader className="pb-2">
                      <div className="flex items-center justify-between">
                        <CardTitle className="text-base">{exec.agentCodename}</CardTitle>
                        <Badge variant="outline" className="text-yellow-600">
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
                          className="h-2"
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

          <TabsContent value="recent" className="flex-1 overflow-auto p-4">
            {completedExecutions.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                <Clock className="h-12 w-12 mb-4 opacity-50" />
                <p className="text-lg font-medium">No Recent Workflows</p>
                <p className="text-sm mt-1">
                  Completed workflows will appear here
                </p>
              </div>
            ) : (
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                {completedExecutions.map((exec) => (
                  <Card
                    key={exec.executionId}
                    className={`cursor-pointer hover:shadow-md transition-shadow border-l-4 ${
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
                          <div className="text-sm text-red-600">
                            {exec.error || 'Execution failed'}
                          </div>
                        )}
                        <div className="flex items-center justify-between text-xs text-muted-foreground pt-2 border-t">
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
                <div className="p-3 border rounded-lg">
                  <div className="text-xs text-muted-foreground">Status</div>
                  <div className="font-medium capitalize">{selectedExecution.status}</div>
                </div>
                <div className="p-3 border rounded-lg">
                  <div className="text-xs text-muted-foreground">Duration</div>
                  <div className="font-medium">
                    {formatDuration(selectedExecution.durationMs)}
                  </div>
                </div>
                <div className="p-3 border rounded-lg">
                  <div className="text-xs text-muted-foreground">Tasks Created</div>
                  <div className="font-medium">{selectedExecution.tasksCreated || 0}</div>
                </div>
                <div className="p-3 border rounded-lg">
                  <div className="text-xs text-muted-foreground">Artifacts</div>
                  <div className="font-medium">{selectedExecution.artifactsCount || 0}</div>
                </div>
              </div>

              <div className="p-3 border rounded-lg">
                <div className="text-xs text-muted-foreground mb-1">Execution ID</div>
                <code className="text-xs">{selectedExecution.executionId}</code>
              </div>

              {selectedExecution.error && (
                <div className="p-3 border border-red-200 rounded-lg bg-red-50">
                  <div className="text-xs text-red-600 mb-1">Error</div>
                  <div className="text-sm text-red-800">{selectedExecution.error}</div>
                </div>
              )}

              <div className="p-3 border rounded-lg">
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
