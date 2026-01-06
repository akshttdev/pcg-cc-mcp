import { useEffect, useState, useMemo } from 'react';
import TaskDetailsHeader from './TaskDetailsHeader';
import { TaskFollowUpSection } from './TaskFollowUpSection';
import { TaskTitleDescription } from './TaskDetails/TaskTitleDescription';
import { TimeTrackerWidget } from '@/components/time-tracking/TimeTrackerWidget';
import { TimeEntriesList } from '@/components/time-tracking/TimeEntriesList';
import { DependencyManager } from '@/components/dependencies/DependencyManager';
import { ActivityFeed } from '@/components/activity/ActivityFeed';
import { CustomPropertiesPanel } from '@/components/custom-properties/CustomPropertiesPanel';
import { TaskCommentThread } from './TaskCommentThread';
import { ActivityTimeline } from './ActivityTimeline';
import { ApprovalPanel } from './ApprovalPanel';
import type {
  AgentFlowEvent,
  ArtifactType,
  ExecutionArtifact,
  FlowEventType,
  TaskAttempt,
  TaskWithAttemptStatus,
} from 'shared/types';
import {
  getBackdropClasses,
  getTaskPanelClasses,
  getTaskPanelInnerClasses,
} from '@/lib/responsive-config';
import type { TabType } from '@/types/tabs';
import DiffTab from '@/components/tasks/TaskDetails/DiffTab.tsx';
import LogsTab from '@/components/tasks/TaskDetails/LogsTab.tsx';
import ProcessesTab from '@/components/tasks/TaskDetails/ProcessesTab.tsx';
import TabNavigation from '@/components/tasks/TaskDetails/TabNavigation.tsx';
import TaskDetailsToolbar from './TaskDetailsToolbar.tsx';
import TodoPanel from '@/components/tasks/TodoPanel';
import { TabNavContext } from '@/contexts/TabNavigationContext';
import { ProcessSelectionProvider } from '@/contexts/ProcessSelectionContext';
import { ReviewProvider } from '@/contexts/ReviewProvider';
import { EntriesProvider } from '@/contexts/EntriesContext';
import { AttemptHeaderCard } from './AttemptHeaderCard';
import { inIframe } from '@/vscode/bridge';
import { TaskRelationshipViewer } from './TaskRelationshipViewer';
import { useTaskViewManager } from '@/hooks/useTaskViewManager.ts';
import { useExecutionSummary } from '@/hooks';
import { ExecutionSummaryCard } from './ExecutionSummaryCard';
import { AirtableRecordLinkBadge } from './AirtableRecordLinkBadge';
import { TaskArtifactsPanel } from './TaskArtifactsPanel';
import { WorkflowTerminal } from './WorkflowTerminal';
import { Skeleton } from '@/components/ui/skeleton';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { agentFlowsApi, taskArtifactsApi, agentsApi } from '@/lib/api';
import type {
  ExecutionArtifact as ApiExecutionArtifact,
} from '@/lib/api';
import type { AgentChatRequest } from 'shared/types';

interface TaskDetailsPanelProps {
  task: TaskWithAttemptStatus | null;
  projectHasDevScript?: boolean;
  projectId: string;
  onClose: () => void;
  onEditTask?: (task: TaskWithAttemptStatus) => void;
  onDeleteTask?: (taskId: string) => void;
  onDuplicateTask?: (task: TaskWithAttemptStatus) => void;
  onNavigateToTask?: (taskId: string) => void;
  hideBackdrop?: boolean;
  className?: string;
  hideHeader?: boolean;
  isFullScreen?: boolean;
  forceCreateAttempt?: boolean;
  onLeaveForceCreateAttempt?: () => void;
  onNewAttempt?: () => void;
  selectedAttempt: TaskAttempt | null;
  attempts: TaskAttempt[];
  setSelectedAttempt: (attempt: TaskAttempt | null) => void;
  tasksById?: Record<string, TaskWithAttemptStatus>;
}

export function TaskDetailsPanel({
  task,
  projectHasDevScript,
  projectId,
  onClose,
  onEditTask,
  onDeleteTask,
  onDuplicateTask,
  onNavigateToTask,
  hideBackdrop = false,
  className,
  isFullScreen,
  forceCreateAttempt,
  onLeaveForceCreateAttempt,
  selectedAttempt,
  attempts,
  setSelectedAttempt,
  tasksById,
}: TaskDetailsPanelProps) {
  // Attempt number, find the current attempt number
  const attemptNumber =
    attempts.length -
    attempts.findIndex((attempt) => attempt.id === selectedAttempt?.id);

  // Fetch execution summary for the selected attempt
  const { data: executionSummary } = useExecutionSummary(selectedAttempt?.id);

  // Tab and collapsible state
  const [activeTab, setActiveTab] = useState<TabType>('logs');
  const [artifacts, setArtifacts] = useState<ExecutionArtifact[]>([]);
  const [artifactsLoading, setArtifactsLoading] = useState(false);
  const [artifactsError, setArtifactsError] = useState<string | null>(null);
  const [workflowEvents, setWorkflowEvents] = useState<AgentFlowEvent[]>([]);
  const [workflowLoading, setWorkflowLoading] = useState(false);
  const [workflowError, setWorkflowError] = useState<string | null>(null);

  // Handler for jumping to diff tab in full screen
  const { toggleFullscreen } = useTaskViewManager();

  const jumpToDiffFullScreen = () => {
    toggleFullscreen(true);
    setActiveTab('diffs');
  };

  const jumpToLogsTab = () => {
    setActiveTab('logs');
  };

  // Reset to logs tab when task changes
  useEffect(() => {
    if (task?.id) {
      setActiveTab('logs');
    }
  }, [task?.id]);

  useEffect(() => {
    if (!task?.id) {
      setArtifacts([]);
      setArtifactsError(null);
      return;
    }

    let cancelled = false;
    setArtifactsLoading(true);
    setArtifactsError(null);

    taskArtifactsApi
      .list(task.id)
      .then((data) => {
        if (cancelled) return;
        const executionArtifacts = data
          .map((item) => item.artifact)
          .filter((artifact): artifact is ApiExecutionArtifact => Boolean(artifact));
        const normalized: ExecutionArtifact[] = executionArtifacts.map((artifact) => ({
          id: artifact.id,
          execution_process_id: artifact.execution_process_id ?? '',
          artifact_type: artifact.artifact_type as ArtifactType,
          title: artifact.title ?? 'Untitled',
          content: artifact.content ?? null,
          file_path: artifact.file_path ?? null,
          metadata: artifact.metadata ?? null,
          created_at: artifact.created_at,
        }));
        setArtifacts(normalized);
      })
      .catch((error) => {
        if (cancelled) return;
        setArtifacts([]);
        setArtifactsError(
          error instanceof Error
            ? error.message
            : 'Failed to load agent artifacts.'
        );
      })
      .finally(() => {
        if (!cancelled) {
          setArtifactsLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [task?.id]);

  useEffect(() => {
    if (!task?.id) {
      setWorkflowEvents([]);
      setWorkflowError(null);
      return;
    }

    let cancelled = false;
    setWorkflowLoading(true);
    setWorkflowError(null);

    agentFlowsApi
      .list({ task_id: task.id })
      .then(async (flows) => {
        if (cancelled) return [] as AgentFlowEvent[];
        if (!flows.length) {
          setWorkflowEvents([]);
          return [] as AgentFlowEvent[];
        }
        return agentFlowsApi.getEvents(flows[0].id);
      })
      .then((events) => {
        if (!events || cancelled) return;
        const normalized: AgentFlowEvent[] = events.map((event) => ({
          ...event,
          event_type: normalizeFlowEventType(event.event_type),
          event_data: event.event_data ?? '',
        }));
        setWorkflowEvents(normalized);
      })
      .catch((error) => {
        if (cancelled) return;
        setWorkflowEvents([]);
        setWorkflowError(
          error instanceof Error
            ? error.message
            : 'Failed to load workflow activity.'
        );
      })
      .finally(() => {
        if (!cancelled) {
          setWorkflowLoading(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [task?.id]);

  const renderArtifactsBody = () => {
    if (!task) return null;
    if (artifactsLoading) {
      return <Skeleton className="h-28 w-full" />;
    }
    if (artifactsError) {
      return (
        <Alert variant="destructive">
          <AlertDescription>{artifactsError}</AlertDescription>
        </Alert>
      );
    }
    if (!artifacts.length) {
      return <p className="text-xs text-muted-foreground">No artifacts yet.</p>;
    }

    return (
      <TaskArtifactsPanel
        taskId={task.id}
        artifacts={artifacts}
        className="shadow-none border"
      />
    );
  };

  // Extract agent name from workflow events
  const executingAgentName = useMemo(() => {
    for (const event of workflowEvents) {
      try {
        const data = JSON.parse(event.event_data || '{}');
        if (data.agent_name) {
          return data.agent_name as string;
        }
      } catch {
        // Skip invalid JSON
      }
    }
    return null;
  }, [workflowEvents]);

  // Handler to send messages regarding workflow - routes to agent directly if available
  const handleSendWorkflowMessage = async (message: string, agentName?: string): Promise<string> => {
    if (!task) throw new Error('No task selected');

    // Try to route to the executing agent by looking up their UUID
    const targetAgentName = executingAgentName || agentName;
    if (targetAgentName) {
      try {
        // Look up agent by name to get the real UUID
        const agent = await agentsApi.getByName(targetAgentName);

        const request: AgentChatRequest = {
          message,
          sessionId: `task-${task.id}`,
          projectId: projectId || null,
          context: {
            taskId: task.id,
            taskTitle: task.title,
            isWorkflowFollowUp: true,
          },
          stream: false,
        };

        const response = await agentsApi.chat(agent.id, request);
        return response.content;
      } catch (error) {
        // Fall back to Nora if agent lookup or chat fails
        console.warn(`Agent chat failed for ${targetAgentName}, falling back to Nora:`, error);
      }
    }

    // Fallback: route through Nora with agent context
    const agentContext = agentName ? `[To ${agentName}] ` : '';
    const contextualMessage = `${agentContext}Regarding workflow task "${task.title}": ${message}`;

    const response = await fetch('/api/nora/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        message: contextualMessage,
        sessionId: `task-${task.id}`,
        requestType: 'textInteraction',
        voiceEnabled: false,
        priority: 'normal',
        context: {
          taskId: task.id,
          taskTitle: task.title,
          projectId,
          executingAgent: agentName,
          isWorkflowFollowUp: true,
        },
      }),
    });

    if (!response.ok) {
      throw new Error(`Failed to send message${agentName ? ` to ${agentName}` : ''}`);
    }

    // Parse the response to get the reply
    const data = await response.json();
    // NoraResponse has 'content' field (camelCase from Rust)
    return data.content || data.message || data.response || 'Response received';
  };

  const renderWorkflowBody = () => {
    if (!task) return null;
    if (workflowLoading) {
      return <Skeleton className="h-40 w-full" />;
    }
    if (workflowError) {
      return (
        <Alert variant="destructive">
          <AlertDescription>{workflowError}</AlertDescription>
        </Alert>
      );
    }

    // Always show the terminal - it handles empty state internally
    return (
      <WorkflowTerminal
        events={workflowEvents}
        taskId={task.id}
        onSendMessage={handleSendWorkflowMessage}
        className="shadow-none"
      />
    );
  };

  return (
    <>
      {!task ? null : (
        <TabNavContext.Provider value={{ activeTab, setActiveTab }}>
          <ProcessSelectionProvider>
            <ReviewProvider>
              <EntriesProvider>
                {/* Backdrop - only on smaller screens (overlay mode) */}
                {!hideBackdrop && (
                  <div
                    className={getBackdropClasses(isFullScreen || false)}
                    onClick={onClose}
                  />
                )}

                {/* Panel */}
                <div
                  className={
                    className || getTaskPanelClasses(isFullScreen || false)
                  }
                >
                  <div className={getTaskPanelInnerClasses()}>
                    {!inIframe() && (
                      <TaskDetailsHeader
                        task={task}
                        onClose={onClose}
                        onEditTask={onEditTask}
                        onDeleteTask={onDeleteTask}
                        onDuplicateTask={onDuplicateTask}
                        hideCloseButton={hideBackdrop}
                        isFullScreen={isFullScreen}
                      />
                    )}

                    {isFullScreen ? (
                      <div className="flex-1 min-h-0 flex">
                        {/* Sidebar */}
                        <aside
                          className={`w-[28rem] shrink-0 border-r overflow-y-auto ${inIframe() ? 'hidden' : ''}`}
                        >
                          {/* Fullscreen sidebar shows title and description above edit/delete */}
                          <div className="space-y-2 p-3">
                            <TaskTitleDescription task={task} />
                            <AirtableRecordLinkBadge
                              taskId={task.id}
                              hasExecutionSummary={!!executionSummary}
                            />
                          </div>

                          {/* Current Attempt / Actions */}
                          <TaskDetailsToolbar
                            task={task}
                            projectId={projectId}
                            projectHasDevScript={projectHasDevScript}
                            forceCreateAttempt={forceCreateAttempt}
                            onLeaveForceCreateAttempt={
                              onLeaveForceCreateAttempt
                            }
                            attempts={attempts}
                            selectedAttempt={selectedAttempt}
                            setSelectedAttempt={setSelectedAttempt}
                            // hide actions in sidebar; moved to header in fullscreen
                          />

                          {/* Task Breakdown (TODOs) */}
                          <TodoPanel />

                          {/* Execution Summary */}
                          {executionSummary && (
                            <div className="p-3">
                              <ExecutionSummaryCard summary={executionSummary} />
                            </div>
                          )}

                          {/* Time Tracking */}
                          <div className="p-3 space-y-3">
                            <TimeTrackerWidget taskId={task.id} />
                            <TimeEntriesList taskId={task.id} />
                          </div>

                          {/* Task Dependencies */}
                          {tasksById && (
                            <div className="p-3">
                              <DependencyManager
                                taskId={task.id}
                                projectTasks={Object.values(tasksById)}
                                onNavigateToTask={onNavigateToTask}
                              />
                            </div>
                          )}

                          {/* Approval Panel */}
                          {task.requires_approval && (
                            <div className="p-3">
                              <ApprovalPanel task={task} />
                            </div>
                          )}

                          {/* Activity Timeline (new collaboration feature) */}
                          <div className="p-3">
                            <ActivityTimeline taskId={task.id} />
                          </div>

                          {/* Comment Thread (new collaboration feature) */}
                          <div className="p-3">
                            <TaskCommentThread taskId={task.id} />
                          </div>

                          {/* Activity Feed (legacy) */}
                          <div className="p-3">
                            <ActivityFeed taskId={task.id} />
                          </div>

                          {/* Agent artifacts */}
                          <div className="p-3 space-y-2">
                            <p className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                              Agent Artifacts
                            </p>
                            {renderArtifactsBody()}
                          </div>

                          {/* Custom Properties */}
                          <div className="p-3">
                            <CustomPropertiesPanel
                              projectId={projectId}
                              taskId={task.id}
                            />
                          </div>

                          {/* Task Relationships */}
                          <TaskRelationshipViewer
                            selectedAttempt={selectedAttempt}
                            onNavigateToTask={onNavigateToTask}
                            task={task}
                            tasksById={tasksById}
                          />
                        </aside>

                        {/* Main content */}
                        <main className="flex-1 min-h-0 min-w-0 flex flex-col">
                          {selectedAttempt ? (
                            <>
                              <TabNavigation
                                activeTab={activeTab}
                                setActiveTab={setActiveTab}
                                selectedAttempt={selectedAttempt}
                              />

                              <div className="flex-1 flex flex-col min-h-0">
                                {activeTab === 'diffs' ? (
                                  <DiffTab selectedAttempt={selectedAttempt} />
                                ) : activeTab === 'processes' ? (
                                  <ProcessesTab
                                    attemptId={selectedAttempt?.id}
                                  />
                                ) : (
                                  <LogsTab selectedAttempt={selectedAttempt} />
                                )}
                              </div>

                              <TaskFollowUpSection
                                task={task}
                                selectedAttemptId={selectedAttempt?.id}
                                jumpToLogsTab={jumpToLogsTab}
                              />
                            </>
                          ) : workflowEvents.length > 0 ? (
                            /* Show WorkflowTerminal when no code attempts but has workflow events */
                            <div className="flex-1 p-4 overflow-auto">
                              {renderWorkflowBody()}
                            </div>
                          ) : null}
                        </main>
                      </div>
                    ) : (
                      <>
                        {attempts.length === 0 ? (
                          <>
                            <TaskDetailsToolbar
                              task={task}
                              projectId={projectId}
                              projectHasDevScript={projectHasDevScript}
                              forceCreateAttempt={forceCreateAttempt}
                              onLeaveForceCreateAttempt={
                                onLeaveForceCreateAttempt
                              }
                              attempts={attempts}
                              selectedAttempt={selectedAttempt}
                              setSelectedAttempt={setSelectedAttempt}
                              // hide actions in sidebar; moved to header in fullscreen
                            />
                            {/* Show WorkflowTerminal as main content for workflow tasks (no code attempts) */}
                            {workflowEvents.length > 0 && (
                              <div className="mt-4">
                                {renderWorkflowBody()}
                              </div>
                            )}
                          </>
                        ) : (
                          <>
                            <AttemptHeaderCard
                              attemptNumber={attemptNumber}
                              totalAttempts={attempts.length}
                              selectedAttempt={selectedAttempt}
                              task={task}
                              projectId={projectId}
                              // onCreateNewAttempt={() => {
                              //   // TODO: Implement create new attempt
                              //   console.log('Create new attempt');
                              // }}
                              onJumpToDiffFullScreen={jumpToDiffFullScreen}
                            />

                            {selectedAttempt && (
                              <LogsTab selectedAttempt={selectedAttempt} />
                            )}

                            <TaskFollowUpSection
                              task={task}
                              selectedAttemptId={selectedAttempt?.id}
                              jumpToLogsTab={jumpToLogsTab}
                            />
                          </>
                        )}
                      </>
                    )}
                  </div>
                </div>
              </EntriesProvider>
            </ReviewProvider>
          </ProcessSelectionProvider>
        </TabNavContext.Provider>
      )}
    </>
  );
}

const FLOW_EVENT_TYPES: FlowEventType[] = [
  'phase_started',
  'phase_completed',
  'artifact_created',
  'artifact_updated',
  'approval_requested',
  'approval_decision',
  'wide_research_started',
  'subagent_progress',
  'wide_research_completed',
  'agent_handoff',
  'flow_paused',
  'flow_resumed',
  'flow_failed',
  'flow_completed',
];

function normalizeFlowEventType(value: string): FlowEventType {
  return FLOW_EVENT_TYPES.includes(value as FlowEventType)
    ? (value as FlowEventType)
    : 'flow_completed';
}
