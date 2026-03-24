import { useCallback, useMemo, useState } from 'react';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { StatusBadge } from '@/components/ui/status-badge';
import { CardGrid } from '@/components/ui/card-grid';
import { EmptyState } from '@/components/ui/empty-state';
import { Search, X, SortAsc, SortDesc } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { useQuery } from '@tanstack/react-query';
import { agentsApi, type AgentSearchParams } from '@/lib/api';
import type { AgentWithParsedFields, AgentStatus } from 'shared/types';
import { AgentDetailDialog } from '@/components/dialogs/agent-detail-dialog';

// Status options for filter
const STATUS_OPTIONS: { value: AgentStatus | 'all'; label: string }[] = [
  { value: 'all', label: 'All Statuses' },
  { value: 'active', label: 'Active' },
  { value: 'inactive', label: 'Inactive' },
  { value: 'maintenance', label: 'Maintenance' },
  { value: 'training', label: 'Training' },
];

// Sort options
const SORT_OPTIONS = [
  { value: 'name', label: 'Name' },
  { value: 'designation', label: 'Designation' },
  { value: 'status', label: 'Status' },
  { value: 'priority', label: 'Priority' },
  { value: 'tasks_completed', label: 'Tasks Completed' },
] as const;

const agentStatusToVariant: Record<string, 'success' | 'muted' | 'warning' | 'info'> = {
  active: 'success',
  inactive: 'muted',
  maintenance: 'warning',
  training: 'info',
};

export function AgentDirectoryCard() {
  // Search and filter state
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<AgentStatus | 'all'>('all');
  const [sortBy, setSortBy] = useState<AgentSearchParams['sort_by']>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  // Agent detail dialog state
  const [selectedAgent, setSelectedAgent] = useState<AgentWithParsedFields | null>(null);
  const [detailDialogOpen, setDetailDialogOpen] = useState(false);

  // Build search params
  const searchParams = useMemo((): AgentSearchParams => {
    const params: AgentSearchParams = {};
    if (searchQuery.trim()) params.q = searchQuery.trim();
    if (statusFilter !== 'all') params.status = statusFilter;
    params.sort_by = sortBy;
    params.sort_dir = sortDir;
    return params;
  }, [searchQuery, statusFilter, sortBy, sortDir]);

  const hasFilters = searchQuery.trim() || statusFilter !== 'all';

  const {
    data: agentDirectory = [],
    isLoading: agentsLoading,
    error: agentsError,
  } = useQuery<AgentWithParsedFields[], Error>({
    queryKey: ['agents', 'search', searchParams],
    queryFn: () => agentsApi.search(searchParams),
  });

  // Clear all filters
  const clearFilters = useCallback(() => {
    setSearchQuery('');
    setStatusFilter('all');
    setSortBy('name');
    setSortDir('asc');
  }, []);

  const handleAgentClick = useCallback((agent: AgentWithParsedFields) => {
    setSelectedAgent(agent);
    setDetailDialogOpen(true);
  }, []);

  return (
    <>
      <Card>
        <CardHeader>
          <div className="flex flex-col gap-4">
            <div>
              <CardTitle>Autonomous Agents</CardTitle>
              <CardDescription>
                Live Directory of all Powerclub Global Agents
              </CardDescription>
            </div>

            {/* Search and Filter Controls */}
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
              {/* Search Input */}
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder="Search agents by name, role, or description..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 pr-9"
                />
                {searchQuery && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 p-0"
                    onClick={() => setSearchQuery('')}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                )}
              </div>

              {/* Status Filter */}
              <Select
                value={statusFilter}
                onValueChange={(v) => setStatusFilter(v as AgentStatus | 'all')}
              >
                <SelectTrigger className="w-[150px]">
                  <SelectValue placeholder="Status" />
                </SelectTrigger>
                <SelectContent>
                  {STATUS_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              {/* Sort Controls */}
              <Select value={sortBy} onValueChange={(v) => setSortBy(v as AgentSearchParams['sort_by'])}>
                <SelectTrigger className="w-[150px]">
                  <SelectValue placeholder="Sort by" />
                </SelectTrigger>
                <SelectContent>
                  {SORT_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              <Button
                variant="outline"
                size="icon"
                onClick={() => setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'))}
                title={sortDir === 'asc' ? 'Sort ascending' : 'Sort descending'}
              >
                {sortDir === 'asc' ? <SortAsc className="h-4 w-4" /> : <SortDesc className="h-4 w-4" />}
              </Button>

              {/* Clear Filters */}
              {hasFilters && (
                <Button variant="ghost" size="sm" onClick={clearFilters}>
                  Clear filters
                </Button>
              )}
            </div>

            {/* Results count */}
            {!agentsLoading && (
              <p className="text-sm text-muted-foreground">
                {agentDirectory.length} agent{agentDirectory.length !== 1 ? 's' : ''} found
                {hasFilters && ' (filtered)'}
              </p>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {agentsLoading ? (
            <Skeleton className="h-40 w-full" />
          ) : agentsError ? (
            <Alert variant="destructive">
              <AlertDescription>
                {agentsError instanceof Error
                  ? agentsError.message
                  : 'Unable to load agent directory.'}
              </AlertDescription>
            </Alert>
          ) : agentDirectory.length === 0 ? (
            <EmptyState
              title="No registered agents"
              description="Seed the registry to expose Nora's team."
              className="py-8"
            />
          ) : (
            <CardGrid columns={{ sm: 2, lg: 3 }} gap={4}>
              {agentDirectory.map((agent) => {
                const initials = agent.short_name
                  .split(' ')
                  .map((part) => part[0])
                  .join('')
                  .slice(0, 2)
                  .toUpperCase();
                return (
                  <div
                    key={agent.id}
                    onClick={() => handleAgentClick(agent)}
                    className="group relative rounded-xl border bg-card overflow-hidden transition-all hover:shadow-md hover:border-primary/20 cursor-pointer"
                  >
                    {/* Status indicator */}
                    <div className="absolute top-3 right-3 z-10">
                      <StatusBadge
                        status={agentStatusToVariant[agent.status] || 'muted'}
                        label={agent.status}
                        className="capitalize backdrop-blur-sm"
                      />
                    </div>

                    {/* Agent image */}
                    <div className="aspect-square w-full bg-muted relative overflow-hidden">
                      {agent.avatar_url ? (
                        <img
                          src={agent.avatar_url}
                          alt={agent.short_name}
                          className="h-full w-full object-cover transition-transform group-hover:scale-105"
                        />
                      ) : (
                        <div className="h-full w-full flex items-center justify-center bg-gradient-to-br from-primary/10 to-primary/5">
                          <span className="text-4xl font-semibold text-primary/40">
                            {initials}
                          </span>
                        </div>
                      )}
                    </div>

                    {/* Agent info */}
                    <div className="p-4 space-y-2">
                      <div>
                        <h3 className="font-semibold text-lg leading-tight">
                          {agent.short_name}
                        </h3>
                        <p className="text-sm font-medium text-primary/80">
                          {agent.designation || 'Specialist Agent'}
                        </p>
                      </div>

                      {agent.description && (
                        <p className="text-sm text-muted-foreground line-clamp-3">
                          {agent.description}
                        </p>
                      )}
                    </div>
                  </div>
                );
              })}
            </CardGrid>
          )}
        </CardContent>
      </Card>

      {/* Agent Detail Dialog */}
      <AgentDetailDialog
        agent={selectedAgent}
        open={detailDialogOpen}
        onOpenChange={setDetailDialogOpen}
      />
    </>
  );
}
