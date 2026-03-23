import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { agentKeys, taskKeys } from '@/lib/query-keys';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Bot, Play, CheckCircle, XCircle, Clock, RefreshCw, AlertCircle } from 'lucide-react';
import { agentsApi, taskAttemptsApi } from '@/lib/api';

function statusBadge(status: string) {
  switch (status) {
    case 'SetupRunning':
    case 'ExecutorRunning':
      return <Badge variant="outline" className="bg-blue-500/10 text-blue-500 border-blue-500/30"><Play className="w-3 h-3 mr-1" />Running</Badge>;
    case 'SetupComplete':
    case 'ExecutorComplete':
      return <Badge variant="outline" className="bg-green-500/10 text-green-500 border-green-500/30"><CheckCircle className="w-3 h-3 mr-1" />Complete</Badge>;
    case 'SetupFailed':
    case 'ExecutorFailed':
      return <Badge variant="outline" className="bg-red-500/10 text-red-500 border-red-500/30"><XCircle className="w-3 h-3 mr-1" />Failed</Badge>;
    default:
      return <Badge variant="outline"><Clock className="w-3 h-3 mr-1" />{status}</Badge>;
  }
}

function timeAgo(dateStr: string | null | undefined): string {
  if (!dateStr) return '—';
  const diff = Date.now() - new Date(dateStr).getTime();
  if (diff < 60_000) return 'just now';
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)}m ago`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)}h ago`;
  return `${Math.floor(diff / 86400_000)}d ago`;
}

type FilterStatus = 'all' | 'running' | 'complete' | 'failed';

export function AgentExecutionsPage() {
  const [statusFilter, setStatusFilter] = useState<FilterStatus>('all');

  const { data: agents = [], isLoading: agentsLoading } = useQuery({
    queryKey: agentKeys.all,
    queryFn: () => agentsApi.list(),
  });

  const { data: recentAttempts = [], isLoading: attemptsLoading, isError: attemptsError, refetch } = useQuery({
    queryKey: taskKeys.attemptsAll(),
    queryFn: () => taskAttemptsApi.list(),
    refetchInterval: 10_000,
  });

  const filtered = recentAttempts.filter((a) => {
    if (statusFilter === 'all') return true;
    const status = (a.status || '').toLowerCase();
    if (statusFilter === 'running') return status.includes('running');
    if (statusFilter === 'complete') return status.includes('complete');
    if (statusFilter === 'failed') return status.includes('failed');
    return true;
  });

  // Summary stats
  const running = recentAttempts.filter((a) => (a.status || '').toLowerCase().includes('running')).length;
  const completed = recentAttempts.filter((a) => (a.status || '').toLowerCase().includes('complete')).length;
  const failed = recentAttempts.filter((a) => (a.status || '').toLowerCase().includes('failed')).length;

  // Build agent lookup map for displaying agent names
  const agentsMap = new Map<string, string>();
  for (const agent of agents) {
    agentsMap.set(agent.id, agent.short_name || agent.designation || agent.id);
  }

  const isLoading = agentsLoading || attemptsLoading;

  return (
    <div className="p-6 space-y-6 max-w-7xl mx-auto">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Bot className="w-6 h-6" />
            Agent Executions
          </h1>
          <p className="text-muted-foreground text-sm mt-1">
            Active attempts, completions, and agent queue
          </p>
        </div>
        <Button variant="outline" size="sm" onClick={() => refetch()}>
          <RefreshCw className="w-4 h-4 mr-1" />
          Refresh
        </Button>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Active Agents</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold">{agents.filter(a => a.status === 'active').length}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-blue-500">Running</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold text-blue-500">{running}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-green-500">Completed</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold text-green-500">{completed}</div>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-red-500">Failed</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-semibold text-red-500">{failed}</div>
          </CardContent>
        </Card>
      </div>

      {/* Filters */}
      <div className="flex items-center gap-4">
        <Select value={statusFilter} onValueChange={(v) => setStatusFilter(v as FilterStatus)}>
          <SelectTrigger className="w-[180px]">
            <SelectValue placeholder="Filter by status" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All Statuses</SelectItem>
            <SelectItem value="running">Running</SelectItem>
            <SelectItem value="complete">Complete</SelectItem>
            <SelectItem value="failed">Failed</SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Error state */}
      {attemptsError && (
        <Card className="border-destructive">
          <CardContent className="flex items-center gap-2 py-4 text-destructive">
            <AlertCircle className="w-4 h-4" />
            Failed to load task attempts. Check that the backend is running.
          </CardContent>
        </Card>
      )}

      {/* Attempts table */}
      {isLoading ? (
        <div className="flex justify-center py-12"><Loader /></div>
      ) : (
        <Card>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Task</TableHead>
                <TableHead>Agent</TableHead>
                <TableHead>Executor</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Branch</TableHead>
                <TableHead>Started</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filtered.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={6} className="text-center text-muted-foreground py-8">
                    No task attempts found
                  </TableCell>
                </TableRow>
              ) : (
                filtered.map((attempt) => (
                  <TableRow key={attempt.id}>
                    <TableCell className="font-medium max-w-[300px] truncate">
                      {attempt.task_title || attempt.task_id || '—'}
                    </TableCell>
                    <TableCell>
                      {attempt.agent_id ? (
                        <Badge variant="outline" className="text-xs">
                          <Bot className="w-3 h-3 mr-1" />
                          {agentsMap.get(attempt.agent_id) || attempt.agent_id.slice(0, 8)}
                        </Badge>
                      ) : (
                        <span className="text-muted-foreground">—</span>
                      )}
                    </TableCell>
                    <TableCell>
                      <code className="text-xs">{attempt.executor || '—'}</code>
                    </TableCell>
                    <TableCell>{statusBadge(attempt.status || 'unknown')}</TableCell>
                    <TableCell>
                      {attempt.branch
                        ? <code className="text-xs">{attempt.branch}</code>
                        : <span className="text-muted-foreground">—</span>
                      }
                    </TableCell>
                    <TableCell className="text-muted-foreground text-sm">
                      {timeAgo(attempt.created_at)}
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </Card>
      )}
    </div>
  );
}
