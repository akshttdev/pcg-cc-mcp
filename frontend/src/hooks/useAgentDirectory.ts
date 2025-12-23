import { useCallback, useEffect, useRef, useState } from 'react';
import {
  AgentCoordinationState,
  CoordinationEvent,
  CoordinationStats,
} from '@/types/nora';

interface AgentDirectoryState {
  agents: AgentCoordinationState[];
  stats: CoordinationStats | null;
  socketConnected: boolean;
  connectionMode: 'websocket' | 'sse';
  lastEvent: CoordinationEvent | null;
  events: CoordinationEvent[];
  refresh: () => Promise<void>;
}

export function useAgentDirectory(): AgentDirectoryState {
  const [agents, setAgents] = useState<AgentCoordinationState[]>([]);
  const [stats, setStats] = useState<CoordinationStats | null>(null);
  const [socketConnected, setSocketConnected] = useState(false);
  const [connectionMode, setConnectionMode] = useState<'websocket' | 'sse'>('websocket');
  const [useSseFallback, setUseSseFallback] = useState(false);
  const [events, setEvents] = useState<CoordinationEvent[]>([]);
  const [lastEvent, setLastEvent] = useState<CoordinationEvent | null>(null);
  const websocketRef = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<number | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);
  const sseReconnectTimeout = useRef<number | null>(null);
  const sseFallbackRef = useRef(false);

  useEffect(() => {
    sseFallbackRef.current = useSseFallback;
  }, [useSseFallback]);

  const refresh = useCallback(async () => {
    try {
      const [statsResponse, agentsResponse] = await Promise.all([
        fetch('/api/nora/coordination/stats'),
        fetch('/api/nora/coordination/agents'),
      ]);

      if (statsResponse.ok) {
        const payload = (await statsResponse.json()) as CoordinationStats;
        setStats(payload);
      }

      if (agentsResponse.ok) {
        const payload = (await agentsResponse.json()) as AgentCoordinationState[];
        setAgents(payload);
      }
    } catch (error) {
      console.error('Failed to refresh agent directory', error);
    }
  }, []);

  const handleEvent = useCallback((event: CoordinationEvent) => {
    setLastEvent(event);
    setEvents((prev) => [event, ...prev].slice(0, 50));

    if (event.type === 'AgentStatusUpdate') {
      void refresh();
    }
  }, [refresh]);

  const setupWebSocket = useCallback(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/api/nora/coordination/events`);

    ws.onopen = () => {
      websocketRef.current = ws;
      setSocketConnected(true);
      setConnectionMode('websocket');
    };

    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data) as CoordinationEvent;
        handleEvent(payload);
      } catch (error) {
        console.error('Failed to parse coordination event payload', error);
      }
    };

    ws.onclose = () => {
      websocketRef.current = null;
      setSocketConnected(false);
      if (!sseFallbackRef.current && typeof window !== 'undefined') {
        reconnectTimeout.current = window.setTimeout(setupWebSocket, 5000);
      }
    };

    ws.onerror = (event) => {
      console.error('Coordination directory websocket error', event);
      sseFallbackRef.current = true;
      setUseSseFallback(true);
    };

    websocketRef.current = ws;
  }, [handleEvent]);

  const setupEventSource = useCallback(() => {
    if (typeof window === 'undefined') {
      return;
    }

    const es = new EventSource('/api/nora/coordination/events/sse');

    es.onopen = () => {
      eventSourceRef.current = es;
      setSocketConnected(true);
      setConnectionMode('sse');
    };

    const onEvent = (event: MessageEvent) => {
      try {
        const payload = JSON.parse(event.data) as CoordinationEvent;
        handleEvent(payload);
      } catch (error) {
        console.error('Failed to parse coordination SSE payload', error);
      }
    };

    es.addEventListener('coordination_event', onEvent);

    es.onerror = (event) => {
      console.error('Coordination directory SSE error', event);
      setSocketConnected(false);
      es.close();
      if (eventSourceRef.current === es) {
        eventSourceRef.current = null;
      }
      if (typeof window !== 'undefined') {
        sseReconnectTimeout.current = window.setTimeout(setupEventSource, 5000);
      }
    };

    eventSourceRef.current = es;
  }, [handleEvent]);

  useEffect(() => {
    void refresh();
    if (useSseFallback) {
      setupEventSource();
    } else {
      setupWebSocket();
    }

    return () => {
      if (websocketRef.current) {
        websocketRef.current.close();
      }
      if (eventSourceRef.current) {
        eventSourceRef.current.close();
      }
      if (reconnectTimeout.current && typeof window !== 'undefined') {
        window.clearTimeout(reconnectTimeout.current);
      }
      if (sseReconnectTimeout.current && typeof window !== 'undefined') {
        window.clearTimeout(sseReconnectTimeout.current);
      }
    };
  }, [refresh, setupEventSource, setupWebSocket, useSseFallback]);

  return {
    agents,
    stats,
    socketConnected,
    connectionMode,
    lastEvent,
    events,
    refresh,
  };
}
