import { useEffect, useState, useCallback, useRef } from 'react';
import type { AgentFlowEvent, FlowEventType } from 'shared/types';

// Valid flow event types for normalization
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

interface UseWorkflowEventStreamOptions {
  enabled?: boolean;
  onEvent?: (event: AgentFlowEvent) => void;
  onError?: (error: Error) => void;
}

interface WorkflowEventStreamState {
  events: AgentFlowEvent[];
  isConnected: boolean;
  error: string | null;
}

/**
 * Hook to subscribe to real-time workflow events via SSE for a specific flow
 */
export function useWorkflowEventStream(
  flowId: string | undefined,
  options: UseWorkflowEventStreamOptions = {}
) {
  const { enabled = true, onEvent, onError } = options;

  const [state, setState] = useState<WorkflowEventStreamState>({
    events: [],
    isConnected: false,
    error: null,
  });

  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const reconnectAttemptsRef = useRef(0);

  const connect = useCallback(() => {
    if (!flowId || !enabled) return;

    // Clean up existing connection
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    try {
      const url = `/api/events/agent-flows/${flowId}/stream`;
      const eventSource = new EventSource(url);
      eventSourceRef.current = eventSource;

      eventSource.onopen = () => {
        setState((prev) => ({ ...prev, isConnected: true, error: null }));
        reconnectAttemptsRef.current = 0;
      };

      eventSource.onmessage = (e) => {
        try {
          const rawEvent = JSON.parse(e.data);
          const event: AgentFlowEvent = {
            ...rawEvent,
            event_type: normalizeFlowEventType(rawEvent.event_type || ''),
            event_data: rawEvent.event_data ?? '',
          };

          setState((prev) => ({
            ...prev,
            events: [...prev.events, event],
          }));

          onEvent?.(event);
        } catch (parseError) {
          console.warn('Failed to parse workflow event:', parseError);
        }
      };

      eventSource.onerror = (e) => {
        console.error('Workflow event stream error:', e);
        eventSource.close();
        setState((prev) => ({ ...prev, isConnected: false }));

        // Reconnect with exponential backoff
        const delay = Math.min(1000 * Math.pow(2, reconnectAttemptsRef.current), 30000);
        reconnectAttemptsRef.current++;

        if (reconnectAttemptsRef.current < 5) {
          reconnectTimeoutRef.current = setTimeout(connect, delay);
        } else {
          const error = new Error('Failed to connect to workflow event stream');
          setState((prev) => ({ ...prev, error: error.message }));
          onError?.(error);
        }
      };
    } catch (err) {
      const error = err instanceof Error ? err : new Error('Failed to connect');
      setState((prev) => ({ ...prev, error: error.message, isConnected: false }));
      onError?.(error);
    }
  }, [flowId, enabled, onEvent, onError]);

  const disconnect = useCallback(() => {
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
    }
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }
    setState((prev) => ({ ...prev, isConnected: false }));
  }, []);

  const clearEvents = useCallback(() => {
    setState((prev) => ({ ...prev, events: [] }));
  }, []);

  useEffect(() => {
    connect();
    return disconnect;
  }, [connect, disconnect]);

  return {
    ...state,
    connect,
    disconnect,
    clearEvents,
  };
}

/**
 * Hook to subscribe to workflow events for multiple tasks
 * Uses polling with smart refresh instead of multiple SSE connections
 */
export function useTasksWorkflowEventPolling(
  taskIds: string[],
  options: {
    enabled?: boolean;
    interval?: number;
  } = {}
) {
  const { enabled = true, interval = 5000 } = options;

  const [eventsByTask, setEventsByTask] = useState<Map<string, AgentFlowEvent[]>>(new Map());
  const [isPolling, setIsPolling] = useState(false);
  const [lastUpdate, setLastUpdate] = useState<Date | null>(null);

  const poll = useCallback(async () => {
    if (!enabled || taskIds.length === 0) return;

    setIsPolling(true);
    try {
      // Fetch latest events for all tasks via API
      const response = await fetch('/api/agent-flows/events/batch', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          task_ids: taskIds,
          since: lastUpdate?.toISOString(),
        }),
      });

      if (!response.ok) {
        // Fallback: fetch individually if batch endpoint doesn't exist
        return;
      }

      const data = await response.json();

      // Update events map
      const newEventsMap = new Map(eventsByTask);
      for (const [taskId, events] of Object.entries(data.events || {})) {
        const normalizedEvents = (events as any[]).map((event) => ({
          ...event,
          event_type: normalizeFlowEventType(event.event_type || ''),
          event_data: event.event_data ?? '',
        }));
        const existing = newEventsMap.get(taskId) || [];
        newEventsMap.set(taskId, [...existing, ...normalizedEvents]);
      }
      setEventsByTask(newEventsMap);
      setLastUpdate(new Date());
    } catch (err) {
      console.warn('Failed to poll workflow events:', err);
    } finally {
      setIsPolling(false);
    }
  }, [enabled, taskIds, lastUpdate, eventsByTask]);

  useEffect(() => {
    if (!enabled || taskIds.length === 0) return;

    // Initial poll
    poll();

    // Set up interval
    const intervalId = setInterval(poll, interval);
    return () => clearInterval(intervalId);
  }, [enabled, taskIds.length, interval]); // Intentionally not including poll to avoid infinite loops

  return {
    eventsByTask,
    isPolling,
    lastUpdate,
    refresh: poll,
  };
}

/**
 * Hook for streaming events from a specific execution process
 */
export function useExecutionEventStream(
  processId: string | undefined,
  options: UseWorkflowEventStreamOptions = {}
) {
  const { enabled = true, onEvent, onError } = options;

  const [events, setEvents] = useState<any[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const eventSourceRef = useRef<EventSource | null>(null);

  useEffect(() => {
    if (!processId || !enabled) return;

    const url = `/api/events/processes/${processId}/logs`;
    const eventSource = new EventSource(url);
    eventSourceRef.current = eventSource;

    eventSource.onopen = () => {
      setIsConnected(true);
      setError(null);
    };

    eventSource.onmessage = (e) => {
      try {
        const event = JSON.parse(e.data);
        setEvents((prev) => [...prev, event]);
        onEvent?.(event);
      } catch (parseError) {
        console.warn('Failed to parse execution event:', parseError);
      }
    };

    eventSource.onerror = () => {
      eventSource.close();
      setIsConnected(false);
      setError('Connection lost');
      onError?.(new Error('Execution event stream disconnected'));
    };

    return () => {
      eventSource.close();
      eventSourceRef.current = null;
    };
  }, [processId, enabled, onEvent, onError]);

  return {
    events,
    isConnected,
    error,
    clearEvents: () => setEvents([]),
  };
}
