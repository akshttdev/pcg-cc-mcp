import { useCallback, useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { AlertTriangle, Bot, Plus, RefreshCw, X, Eye, Search } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { agentWatchersApi, agentsApi } from '@/lib/api';
import { agentWatcherKeys } from '@/lib/query-keys';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import type { AgentWithParsedFields } from 'shared/types';
import { cn } from '@/lib/utils';

const WATCHER_POLL_INTERVAL = 10_000;

const STATUS_CONFIG: Record<string, { label: string; className: string }> = {
  watching: { label: 'Watching', className: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300' },
  triggered: { label: 'Running', className: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300' },
  qa_pass: { label: 'Passed', className: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300' },
  qa_needs_changes: { label: 'Changes Needed', className: 'bg-orange-100 text-orange-700 dark:bg-orange-900 dark:text-orange-300' },
  qa_fail: { label: 'Failed', className: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300' },
};

interface AgentWatcherPanelProps {
  taskId: string;
}

export function AgentWatcherPanel({ taskId }: AgentWatcherPanelProps) {
  const [pickerOpen, setPickerOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [removing, setRemoving] = useState<Set<string>>(new Set());

  const { data: watchers = [], error: loadError, refetch: refetchWatchers } = useQuery({
    queryKey: agentWatcherKeys.watchers(taskId),
    queryFn: () => agentWatchersApi.list(taskId),
    refetchInterval: WATCHER_POLL_INTERVAL,
  });

  const { data: availableAgents = [] } = useQuery({
    queryKey: agentWatcherKeys.availableAgents(),
    queryFn: () => agentsApi.listActive(),
    enabled: pickerOpen,
  });

  const handlePickerOpenChange = useCallback((open: boolean) => {
    setPickerOpen(open);
    if (!open) {
      setSearchQuery('');
    }
  }, []);

  const watcherIds = useMemo(
    () => new Set(watchers.map((w) => w.agent_id)),
    [watchers]
  );

  const filteredAgents = useMemo(() => {
    const q = searchQuery.toLowerCase();
    return availableAgents.filter(
      (a: AgentWithParsedFields) =>
        !watcherIds.has(a.id) &&
        (a.short_name.toLowerCase().includes(q) ||
          a.designation.toLowerCase().includes(q))
    );
  }, [availableAgents, watcherIds, searchQuery]);

  const addMutation = useMutationWithToast({
    mutationFn: (agentId: string) => agentWatchersApi.add(taskId, agentId),
    errorMessage: 'Failed to add agent reviewer',
    invalidateKeys: [agentWatcherKeys.watchers(taskId)],
    onSuccess: () => handlePickerOpenChange(false),
  });

  const removeMutation = useMutationWithToast({
    mutationFn: (agentId: string) => agentWatchersApi.remove(taskId, agentId),
    errorMessage: 'Failed to remove agent reviewer',
    invalidateKeys: [agentWatcherKeys.watchers(taskId)],
  });

  const handleRemove = async (agentId: string) => {
    if (removing.has(agentId)) return;
    setRemoving((prev) => new Set(prev).add(agentId));
    try {
      await removeMutation.mutateAsync(agentId);
    } finally {
      setRemoving((prev) => {
        const next = new Set(prev);
        next.delete(agentId);
        return next;
      });
    }
  };

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <p className="text-xs font-semibold uppercase tracking-wide text-muted-foreground flex items-center gap-1">
          <Eye className="h-3 w-3" />
          Agent Reviewers
        </p>
        <Popover open={pickerOpen} onOpenChange={handlePickerOpenChange}>
          <PopoverTrigger asChild>
            <Button variant="ghost" size="sm" className="h-6 px-1.5 text-xs">
              <Plus className="h-3 w-3 mr-1" />
              Add
            </Button>
          </PopoverTrigger>
          <PopoverContent className="w-64 p-2" align="end">
            <div className="relative mb-2">
              <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground" />
              <input
                type="text"
                placeholder="Search agents..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="w-full pl-7 pr-2 py-1.5 text-xs rounded-md border bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                autoFocus
              />
            </div>
            <div className="max-h-48 overflow-y-auto space-y-0.5">
              {filteredAgents.length === 0 ? (
                <p className="text-xs text-muted-foreground text-center py-3">
                  No available agents
                </p>
              ) : (
                filteredAgents.map((agent) => (
                  <button
                    key={agent.id}
                    disabled={addMutation.isPending}
                    onClick={() => addMutation.mutate(agent.id)}
                    className="w-full flex items-center gap-2 px-2 py-1.5 rounded-md text-xs hover:bg-accent transition-colors text-left"
                  >
                    <div className="h-5 w-5 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center flex-shrink-0">
                      <Bot className="h-3 w-3 text-blue-700 dark:text-blue-300" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="font-medium truncate">{agent.short_name}</div>
                      <div className="text-muted-foreground truncate">{agent.designation}</div>
                    </div>
                  </button>
                ))
              )}
            </div>
          </PopoverContent>
        </Popover>
      </div>

      {loadError ? (
        <div className="flex items-center gap-2 text-xs text-destructive py-1">
          <AlertTriangle className="h-3 w-3 shrink-0" />
          <span className="flex-1">Failed to load agent reviewers</span>
          <Button variant="ghost" size="sm" className="h-6 px-2 text-xs" onClick={() => refetchWatchers()}>
            <RefreshCw className="h-3 w-3 mr-1" />
            Retry
          </Button>
        </div>
      ) : watchers.length === 0 ? (
        <p className="text-xs text-muted-foreground">No agent reviewers assigned</p>
      ) : (
        <div className="space-y-1">
          {watchers.map((w) => {
            const status = STATUS_CONFIG[w.last_action] ?? STATUS_CONFIG.watching;
            return (
              <div
                key={w.agent_id}
                className={cn(
                  'flex items-center gap-2 px-2 py-1.5 rounded-md bg-muted/50 group transition-opacity',
                  removing.has(w.agent_id) && 'opacity-40 pointer-events-none'
                )}
              >
                <div className="h-5 w-5 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center flex-shrink-0">
                  <Bot className="h-3 w-3 text-blue-700 dark:text-blue-300" />
                </div>
                <div className="min-w-0 flex-1">
                  <Link
                    to={`/agents/${w.agent_id}/profile`}
                    className="text-xs font-medium truncate block hover:text-primary transition-colors"
                    title="View agent profile"
                  >
                    {w.agent_name}
                  </Link>
                </div>
                <Badge
                  variant="secondary"
                  className={cn('text-xs px-1.5 py-0', status.className)}
                >
                  {status.label}
                </Badge>
                <button
                  onClick={() => handleRemove(w.agent_id)}
                  disabled={removing.has(w.agent_id)}
                  className="opacity-0 group-hover:opacity-100 transition-opacity p-0.5 rounded hover:bg-destructive/10 disabled:opacity-50"
                  title="Remove watcher"
                >
                  <X className="h-3 w-3 text-muted-foreground hover:text-destructive" />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
