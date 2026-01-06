import { useCallback, useEffect, useRef, useState } from 'react';
import type { CoordinationEvent } from '@/types/nora';

/** Filter for execution-specific events */
type ExecutionEventType =
  | 'ExecutionStarted'
  | 'ExecutionStageStarted'
  | 'ExecutionStageCompleted'
  | 'ExecutionCompleted'
  | 'ExecutionFailed'
  | 'ExecutionTaskCreated'
  | 'ExecutionArtifactProduced'
  | 'WorkflowProgress';

/** Active execution state for UI display */
export interface ActiveExecution {
  executionId: string;
  projectId: string | null;
  agentCodename: string;
  workflowName: string | null;
  status: 'running' | 'completed' | 'failed';
  currentStage: number;
  totalStages: number;
  stageName: string;
  startedAt: string;
  completedAt?: string;
  tasksCreated: number;
  artifactsCount: number;
  durationMs?: number;
  error?: string;
}

interface UseExecutionEventsOptions {
  /** Only track executions for this project */
  projectId?: string;
  /** Max number of completed executions to keep */
  maxHistory?: number;
}

export function useExecutionEvents(options: UseExecutionEventsOptions = {}) {
  const { projectId, maxHistory = 10 } = options;

  const [activeExecutions, setActiveExecutions] = useState<Map<string, ActiveExecution>>(new Map());
  const [completedExecutions, setCompletedExecutions] = useState<ActiveExecution[]>([]);
  const [connected, setConnected] = useState(false);
  const [connectionMode, setConnectionMode] = useState<'websocket' | 'sse' | null>(null);

  const websocketRef = useRef<WebSocket | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectTimeout = useRef<ReturnType<typeof setTimeout> | null>(null);

  const isExecutionEvent = (event: CoordinationEvent): boolean => {
    const executionTypes: ExecutionEventType[] = [
      'ExecutionStarted',
      'ExecutionStageStarted',
      'ExecutionStageCompleted',
      'ExecutionCompleted',
      'ExecutionFailed',
      'ExecutionTaskCreated',
      'ExecutionArtifactProduced',
      'WorkflowProgress',
    ];
    return executionTypes.includes(event.type as ExecutionEventType);
  };

  const handleEvent = useCallback((event: CoordinationEvent) => {
    if (!isExecutionEvent(event)) return;

    // Filter by project if specified
    if (projectId) {
      if ('projectId' in event && event.projectId !== projectId) {
        return;
      }
    }

    setActiveExecutions((prev) => {
      const next = new Map(prev);

      switch (event.type) {
        case 'ExecutionStarted': {
          next.set(event.executionId, {
            executionId: event.executionId,
            projectId: event.projectId,
            agentCodename: event.agentCodename,
            workflowName: event.workflowName,
            status: 'running',
            currentStage: 0,
            totalStages: 0,
            stageName: 'Starting',
            startedAt: event.timestamp,
            tasksCreated: 0,
            artifactsCount: 0,
          });
          break;
        }

        case 'ExecutionStageStarted': {
          const existing = next.get(event.executionId);
          if (existing) {
            next.set(event.executionId, {
              ...existing,
              currentStage: event.stageIndex,
              stageName: event.stageName,
            });
          }
          break;
        }

        case 'ExecutionStageCompleted': {
          const existing = next.get(event.executionId);
          if (existing) {
            next.set(event.executionId, {
              ...existing,
              currentStage: event.stageIndex + 1,
            });
          }
          break;
        }

        case 'ExecutionCompleted': {
          const existing = next.get(event.executionId);
          if (existing) {
            const completed: ActiveExecution = {
              ...existing,
              status: 'completed',
              completedAt: event.timestamp,
              tasksCreated: event.tasksCreated,
              artifactsCount: event.artifactsCount,
              durationMs: event.durationMs,
            };
            next.delete(event.executionId);

            // Add to completed history
            setCompletedExecutions((prevCompleted) => {
              const updated = [completed, ...prevCompleted];
              return updated.slice(0, maxHistory);
            });
          }
          break;
        }

        case 'ExecutionFailed': {
          const existing = next.get(event.executionId);
          if (existing) {
            const failed: ActiveExecution = {
              ...existing,
              status: 'failed',
              completedAt: event.timestamp,
              error: event.error,
            };
            next.delete(event.executionId);

            setCompletedExecutions((prevCompleted) => {
              const updated = [failed, ...prevCompleted];
              return updated.slice(0, maxHistory);
            });
          }
          break;
        }

        case 'ExecutionTaskCreated': {
          const existing = next.get(event.executionId);
          if (existing) {
            next.set(event.executionId, {
              ...existing,
              tasksCreated: existing.tasksCreated + 1,
            });
          }
          break;
        }

        case 'ExecutionArtifactProduced': {
          const existing = next.get(event.executionId);
          if (existing) {
            next.set(event.executionId, {
              ...existing,
              artifactsCount: existing.artifactsCount + 1,
            });
          }
          break;
        }

        case 'WorkflowProgress': {
          // WorkflowProgress can also update execution state
          const existing = next.get(event.workflowInstanceId);
          if (existing) {
            next.set(event.workflowInstanceId, {
              ...existing,
              currentStage: event.currentStage,
              totalStages: event.totalStages,
              stageName: event.stageName,
              status: event.status === 'failed' ? 'failed' :
                      event.status === 'completed' ? 'completed' : 'running',
            });
          }
          break;
        }
      }

      return next;
    });
  }, [projectId, maxHistory]);

  const setupWebSocket = useCallback(() => {
    if (typeof window === 'undefined') return;

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/api/nora/coordination/events`);

    ws.onopen = () => {
      websocketRef.current = ws;
      setConnected(true);
      setConnectionMode('websocket');
    };

    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as CoordinationEvent;
        handleEvent(payload);
      } catch (error) {
        console.error('Failed to parse execution event payload', error);
      }
    };

    ws.onclose = () => {
      websocketRef.current = null;
      setConnected(false);
      reconnectTimeout.current = window.setTimeout(setupWebSocket, 5000);
    };

    ws.onerror = () => {
      // Fall back to SSE
      ws.close();
      setupEventSource();
    };

    websocketRef.current = ws;
  }, [handleEvent]);

  const setupEventSource = useCallback(() => {
    if (typeof window === 'undefined') return;

    const es = new EventSource('/api/nora/coordination/events/sse');

    es.onopen = () => {
      eventSourceRef.current = es;
      setConnected(true);
      setConnectionMode('sse');
    };

    es.addEventListener('coordination_event', (event: MessageEvent) => {
      try {
        const payload = JSON.parse(event.data) as CoordinationEvent;
        handleEvent(payload);
      } catch (error) {
        console.error('Failed to parse execution SSE payload', error);
      }
    });

    es.onerror = () => {
      setConnected(false);
      es.close();
      eventSourceRef.current = null;
      reconnectTimeout.current = window.setTimeout(setupEventSource, 5000);
    };

    eventSourceRef.current = es;
  }, [handleEvent]);

  useEffect(() => {
    setupWebSocket();

    return () => {
      if (websocketRef.current) {
        websocketRef.current.close();
      }
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
      if (reconnectTimeout.current) {
        clearTimeout(reconnectTimeout.current);
      }
    };
  }, [setupWebSocket]);

  return {
    activeExecutions: Array.from(activeExecutions.values()),
    completedExecutions,
    connected,
    connectionMode,
    activeCount: activeExecutions.size,
  };
}
