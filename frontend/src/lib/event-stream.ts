import { useEffect, useState, useCallback, useRef } from 'react';

export interface AgentFlowEvent {
  id: string;
  agent_flow_id: string;
  event_type: string;
  event_data: string;
  created_at: string;
}

export interface UseEventStreamOptions {
  /** Flow ID to subscribe to (for flow-specific events) */
  flowId?: string;
  /** Called when new events are received */
  onEvents?: (events: AgentFlowEvent[]) => void;
  /** Called on connection error */
  onError?: (error: Error) => void;
  /** Called when connection is established */
  onConnect?: () => void;
  /** Called when connection is closed */
  onDisconnect?: () => void;
  /** Auto-reconnect on error (default: true) */
  autoReconnect?: boolean;
  /** Reconnect delay in ms (default: 3000) */
  reconnectDelay?: number;
}

export interface UseEventStreamResult {
  /** All received events */
  events: AgentFlowEvent[];
  /** Whether the stream is connected */
  isConnected: boolean;
  /** Last error if any */
  error: Error | null;
  /** Manually connect to the stream */
  connect: () => void;
  /** Manually disconnect from the stream */
  disconnect: () => void;
  /** Clear all stored events */
  clearEvents: () => void;
}

/**
 * Hook to subscribe to SSE event stream for agent flow events
 */
export function useEventStream(options: UseEventStreamOptions = {}): UseEventStreamResult {
  const {
    flowId,
    onEvents,
    onError,
    onConnect,
    onDisconnect,
    autoReconnect = true,
    reconnectDelay = 3000,
  } = options;

  const [events, setEvents] = useState<AgentFlowEvent[]>([]);
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<Error | null>(null);

  const eventSourceRef = useRef<EventSource | null>(null);
  const reconnectTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const clearEvents = useCallback(() => {
    setEvents([]);
  }, []);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (eventSourceRef.current) {
      eventSourceRef.current.close();
      eventSourceRef.current = null;
      setIsConnected(false);
      onDisconnect?.();
    }
  }, [onDisconnect]);

  const connect = useCallback(() => {
    // Clean up existing connection
    disconnect();

    const url = flowId
      ? `/api/events/flows/${flowId}`
      : '/api/events/all';

    try {
      const eventSource = new EventSource(url);
      eventSourceRef.current = eventSource;

      eventSource.onopen = () => {
        setIsConnected(true);
        setError(null);
        onConnect?.();
      };

      eventSource.addEventListener('flow_events', (event) => {
        try {
          const newEvents = JSON.parse(event.data) as AgentFlowEvent[];
          setEvents((prev) => [...prev, ...newEvents]);
          onEvents?.(newEvents);
        } catch (e) {
          console.error('Failed to parse flow events:', e);
        }
      });

      eventSource.addEventListener('all_events', (event) => {
        try {
          const newEvents = JSON.parse(event.data) as AgentFlowEvent[];
          setEvents((prev) => [...prev, ...newEvents]);
          onEvents?.(newEvents);
        } catch (e) {
          console.error('Failed to parse all events:', e);
        }
      });

      eventSource.onerror = () => {
        const err = new Error('EventSource connection error');
        setError(err);
        setIsConnected(false);
        onError?.(err);

        eventSource.close();
        eventSourceRef.current = null;

        // Auto-reconnect
        if (autoReconnect) {
          reconnectTimeoutRef.current = setTimeout(() => {
            connect();
          }, reconnectDelay);
        }
      };
    } catch (e) {
      const err = e instanceof Error ? e : new Error('Failed to connect');
      setError(err);
      onError?.(err);
    }
  }, [flowId, onEvents, onError, onConnect, onDisconnect, autoReconnect, reconnectDelay, disconnect]);

  // Connect on mount, disconnect on unmount
  useEffect(() => {
    connect();
    return () => disconnect();
  }, [connect, disconnect]);

  return {
    events,
    isConnected,
    error,
    connect,
    disconnect,
    clearEvents,
  };
}

/**
 * Simple function to subscribe to events without React hooks
 */
export function subscribeToEvents(
  options: {
    flowId?: string;
    onEvent: (event: AgentFlowEvent) => void;
    onError?: (error: Error) => void;
  }
): () => void {
  const url = options.flowId
    ? `/api/events/flows/${options.flowId}`
    : '/api/events/all';

  const eventSource = new EventSource(url);

  const handleEvent = (event: MessageEvent) => {
    try {
      const events = JSON.parse(event.data) as AgentFlowEvent[];
      events.forEach(options.onEvent);
    } catch (e) {
      console.error('Failed to parse event:', e);
    }
  };

  eventSource.addEventListener('flow_events', handleEvent);
  eventSource.addEventListener('all_events', handleEvent);

  eventSource.onerror = () => {
    options.onError?.(new Error('EventSource connection error'));
  };

  // Return unsubscribe function
  return () => {
    eventSource.close();
  };
}
