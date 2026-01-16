import { useEffect, useState, useRef } from 'react';
import type { PatchType } from 'shared/types';
import { executionProcessesApi } from '@/lib/api';

type LogEntry = Extract<PatchType, { type: 'STDOUT' } | { type: 'STDERR' }>;

interface UseLogStreamResult {
  logs: LogEntry[];
  error: string | null;
  isLoading: boolean;
}

export const useLogStream = (processId: string): UseLogStreamResult => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const wsRef = useRef<WebSocket | null>(null);
  const retryCountRef = useRef<number>(0);
  const retryTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isIntentionallyClosed = useRef<boolean>(false);
  const loadedHistoricLogs = useRef<boolean>(false);

  useEffect(() => {
    if (!processId) {
      return;
    }

    // Clear logs when process changes
    setLogs([]);
    setError(null);
    setIsLoading(true);
    loadedHistoricLogs.current = false;

    // First, try to load stored logs from REST API
    const loadStoredLogs = async () => {
      try {
        const storedLogs = await executionProcessesApi.getStoredLogs(processId);
        if (storedLogs && storedLogs.logs) {
          const entries: LogEntry[] = [];
          const lines = storedLogs.logs.split('\n').filter(line => line.trim());

          for (const line of lines) {
            try {
              const logMsg = JSON.parse(line);
              if (logMsg.Stdout) {
                entries.push({ type: 'STDOUT', content: logMsg.Stdout });
              } else if (logMsg.Stderr) {
                entries.push({ type: 'STDERR', content: logMsg.Stderr });
              }
            } catch {
              // Skip malformed lines
            }
          }

          if (entries.length > 0) {
            setLogs(entries);
            loadedHistoricLogs.current = true;
          }
        }
      } catch (err) {
        console.warn('Failed to load stored logs, will try WebSocket', err);
      } finally {
        setIsLoading(false);
      }
    };

    loadStoredLogs();

    const open = () => {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const host = window.location.host;
      const ws = new WebSocket(
        `${protocol}//${host}/api/execution-processes/${processId}/raw-logs/ws`
      );
      wsRef.current = ws;
      isIntentionallyClosed.current = false;

      ws.onopen = () => {
        setError(null);
        // Only reset logs if we haven't loaded historic logs from REST API
        if (!loadedHistoricLogs.current) {
          setLogs([]);
        }
        retryCountRef.current = 0;
      };

      const addLogEntry = (entry: LogEntry) => {
        setLogs((prev) => [...prev, entry]);
      };

      // Handle WebSocket messages
      ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);

          // Handle different message types based on LogMsg enum
          if ('JsonPatch' in data) {
            const patches = data.JsonPatch;
            patches.forEach((patch: any) => {
              const value = patch?.value;
              if (!value || !value.type) return;

              switch (value.type) {
                case 'STDOUT':
                case 'STDERR':
                  addLogEntry({ type: value.type, content: value.content });
                  break;
                // Ignore other patch types (NORMALIZED_ENTRY, DIFF, etc.)
                default:
                  break;
              }
            });
          } else if (data.finished === true) {
            isIntentionallyClosed.current = true;
            ws.close();
          }
        } catch (e) {
          console.error('Failed to parse message:', e);
        }
      };

      ws.onerror = () => {
        setError('Connection failed');
      };

      ws.onclose = (event) => {
        // Only retry if the close was not intentional and not a normal closure
        if (!isIntentionallyClosed.current && event.code !== 1000) {
          const next = retryCountRef.current + 1;
          retryCountRef.current = next;
          if (next <= 6) {
            const delay = Math.min(1500, 250 * 2 ** (next - 1));
            retryTimerRef.current = setTimeout(() => open(), delay);
          }
        }
      };
    };

    open();

    return () => {
      if (wsRef.current) {
        isIntentionallyClosed.current = true;
        wsRef.current.close();
        wsRef.current = null;
      }
      if (retryTimerRef.current) {
        clearTimeout(retryTimerRef.current);
        retryTimerRef.current = null;
      }
    };
  }, [processId]);

  return { logs, error, isLoading };
};
