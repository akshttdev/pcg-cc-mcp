import { useMemo, useState } from 'react';
import { WideResearchSessionCard } from './WideResearchSessionCard';
import { WideResearchGrid } from './WideResearchGrid';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from '@/components/ui/sheet';
import { Loader } from '@/components/ui/loader';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { Search, Grid3X3, List, Filter, PlayCircle, CheckCircle2, XCircle } from 'lucide-react';
import { cn } from '@/lib/utils';
import type {
  WideResearchSession,
  WideResearchSubagent,
  ResearchSessionStatus,
} from 'shared/types';

interface SessionWithSubagents {
  session: WideResearchSession;
  subagents: WideResearchSubagent[];
  progress_percent: number;
}

interface ResearchHubBoardProps {
  sessions: SessionWithSubagents[];
  isLoading?: boolean;
  onSessionClick?: (session: WideResearchSession) => void;
  onSubagentClick?: (subagent: WideResearchSubagent) => void;
  className?: string;
}

export function ResearchHubBoard({
  sessions,
  isLoading,
  onSessionClick,
  onSubagentClick,
  className,
}: ResearchHubBoardProps) {
  const [searchQuery, setSearchQuery] = useState('');
  const [filterStatus, setFilterStatus] = useState<ResearchSessionStatus | 'all'>('all');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');
  const [selectedSession, setSelectedSession] = useState<SessionWithSubagents | null>(null);

  // Filter sessions
  const filteredSessions = useMemo(() => {
    return sessions.filter((item) => {
      // Search filter
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        if (!item.session.task_description.toLowerCase().includes(query)) {
          return false;
        }
      }

      // Status filter
      if (filterStatus !== 'all' && item.session.status !== filterStatus) {
        return false;
      }

      return true;
    });
  }, [sessions, searchQuery, filterStatus]);

  // Stats
  const stats = useMemo(() => {
    const total = sessions.length;
    const activeStatuses: ResearchSessionStatus[] = [
      'spawning',
      'in_progress',
      'aggregating',
    ];
    const inProgress = sessions.filter((s) =>
      activeStatuses.includes(s.session.status)
    ).length;
    const completed = sessions.filter(
      (s) => s.session.status === 'completed'
    ).length;
    const failed = sessions.filter((s) => s.session.status === 'failed').length;

    return { total, inProgress, completed, failed };
  }, [sessions]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader />
      </div>
    );
  }

  return (
    <div className={cn('flex flex-col h-full', className)}>
      {/* Header */}
      <div className="p-4 border-b bg-background">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">Research Hub</h2>
          <div className="flex items-center gap-2">
            <Badge variant="outline">
              <PlayCircle className="h-3 w-3 mr-1 text-blue-500" />
              {stats.inProgress} Active
            </Badge>
            <Badge variant="outline">
              <CheckCircle2 className="h-3 w-3 mr-1 text-green-500" />
              {stats.completed} Completed
            </Badge>
            {stats.failed > 0 && (
              <Badge variant="outline">
                <XCircle className="h-3 w-3 mr-1 text-red-500" />
                {stats.failed} Failed
              </Badge>
            )}
          </div>
        </div>

        {/* Filters */}
        <div className="flex items-center gap-4">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search research sessions..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9"
            />
          </div>

          <Select
            value={filterStatus}
            onValueChange={(value) =>
              setFilterStatus(value as ResearchSessionStatus | 'all')
            }
          >
            <SelectTrigger className="w-40">
              <Filter className="h-4 w-4 mr-2" />
              <SelectValue placeholder="Filter status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Statuses</SelectItem>
              <SelectItem value="spawning">Spawning</SelectItem>
              <SelectItem value="in_progress">In Progress</SelectItem>
              <SelectItem value="aggregating">Aggregating</SelectItem>
              <SelectItem value="completed">Completed</SelectItem>
              <SelectItem value="failed">Failed</SelectItem>
              <SelectItem value="cancelled">Cancelled</SelectItem>
            </SelectContent>
          </Select>

          <div className="flex border rounded-md">
            <Button
              size="sm"
              variant={viewMode === 'grid' ? 'default' : 'ghost'}
              onClick={() => setViewMode('grid')}
              className="rounded-r-none"
            >
              <Grid3X3 className="h-4 w-4" />
            </Button>
            <Button
              size="sm"
              variant={viewMode === 'list' ? 'default' : 'ghost'}
              onClick={() => setViewMode('list')}
              className="rounded-l-none"
            >
              <List className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Content */}
      <ScrollArea className="flex-1">
        <div className="p-4">
          {viewMode === 'grid' ? (
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              {filteredSessions.map((item) => (
                <WideResearchSessionCard
                  key={item.session.id}
                  session={item.session}
                  progressPercent={item.progress_percent}
                  onClick={() => {
                    setSelectedSession(item);
                    onSessionClick?.(item.session);
                  }}
                />
              ))}
            </div>
          ) : (
            <div className="space-y-4">
              {filteredSessions.map((item) => (
                <WideResearchGrid
                  key={item.session.id}
                  session={item.session}
                  subagents={item.subagents}
                  onSubagentClick={onSubagentClick}
                />
              ))}
            </div>
          )}

          {filteredSessions.length === 0 && (
            <div className="flex flex-col items-center justify-center h-64 text-muted-foreground">
              <Grid3X3 className="h-12 w-12 mb-4" />
              <p>No research sessions found</p>
              {searchQuery && (
                <Button
                  variant="link"
                  className="mt-2"
                  onClick={() => setSearchQuery('')}
                >
                  Clear search
                </Button>
              )}
            </div>
          )}
        </div>
      </ScrollArea>

      {/* Detail Sheet */}
      <Sheet
        open={!!selectedSession}
        onOpenChange={(open) => !open && setSelectedSession(null)}
      >
        <SheetContent className="w-full sm:max-w-2xl overflow-auto">
          <SheetHeader>
            <SheetTitle>Research Session Details</SheetTitle>
          </SheetHeader>
          {selectedSession && (
            <div className="mt-4">
              <WideResearchGrid
                session={selectedSession.session}
                subagents={selectedSession.subagents}
                onSubagentClick={onSubagentClick}
              />
            </div>
          )}
        </SheetContent>
      </Sheet>
    </div>
  );
}
