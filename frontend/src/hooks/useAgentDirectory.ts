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
  lastEvent: CoordinationEvent | null;
  events: CoordinationEvent[];
  refresh: () => Promise<void>;
}

export function useAgentDirectory(): AgentDirectoryState {
  const [agents, setAgents] = useState<AgentCoordinationState[]>([]);
  const [stats, setStats] = useState<CoordinationStats | null>(null);
  const [socketConnected, setSocketConnected] = useState(false);
  const [events, setEvents] = useState<CoordinationEvent[]>([]);
  const [lastEvent, setLastEvent] = useState<CoordinationEvent | null>(null);
  const websocketRef = useRef<WebSocket | null>(null);
  const reconnectTimeout = useRef<number | null>(null);

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
      if (typeof window !== 'undefined') {
        reconnectTimeout.current = window.setTimeout(setupWebSocket, 5000);
      }
    };

    ws.onerror = (event) => {
      console.error('Coordination directory websocket error', event);
    };

    websocketRef.current = ws;
  }, [handleEvent]);

  useEffect(() => {
    void refresh();
    setupWebSocket();

    return () => {
      if (websocketRef.current) {
        websocketRef.current.close();
      }
      if (reconnectTimeout.current && typeof window !== 'undefined') {
        window.clearTimeout(reconnectTimeout.current);
      }
    };
  }, [refresh, setupWebSocket]);

  return {
    agents,
    stats,
    socketConnected,
    lastEvent,
    events,
    refresh,
  };
}
