import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Activity,
  AlertTriangle,
  CheckCircle2,
  ExternalLink,
  Loader2,
  Radio,
  RefreshCw,
  Rss,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { pulseApi } from '@/lib/api';
import { pulseKeys } from '@/lib/query-keys';

// --- Types ---

interface PulseProject {
  name: string;
  description: string | null;
  adapters: number;
  active_sources: string[];
  scheduler_running: boolean;
}

// --- Helpers ---

function timeAgo(dateStr: string): string {
  const now = new Date();
  const date = new Date(dateStr);
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);

  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function sourceColor(sourceId: string): string {
  if (sourceId.includes('reddit') || sourceId.startsWith('r-'))
    return 'bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300';
  if (sourceId.includes('web'))
    return 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300';
  return 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300';
}

// --- Components ---

function ProjectCard({ project }: { project: PulseProject }) {
  return (
    <div className="flex items-center justify-between py-2">
      <div className="flex items-center gap-2">
        <Radio
          className={cn(
            'h-3.5 w-3.5',
            project.scheduler_running
              ? 'text-green-500 animate-pulse'
              : 'text-muted-foreground'
          )}
        />
        <div>
          <p className="text-sm font-medium">{project.name}</p>
          <p className="text-xs text-muted-foreground">
            {project.adapters} sources
          </p>
        </div>
      </div>
      <Badge
        variant="outline"
        className={cn(
          'text-xs',
          project.scheduler_running
            ? 'border-green-500/30 text-green-600'
            : 'border-muted text-muted-foreground'
        )}
      >
        {project.scheduler_running ? 'Active' : 'Idle'}
      </Badge>
    </div>
  );
}

function ContentRow({ item }: { item: any }) {
  return (
    <div className="flex items-start gap-2 py-2 border-b border-border/50 last:border-0">
      <Rss className="h-3.5 w-3.5 mt-0.5 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <a
          href={item.url}
          target="_blank"
          rel="noopener noreferrer"
          className="text-sm font-medium hover:underline line-clamp-1 flex items-center gap-1"
        >
          {item.title}
          <ExternalLink className="h-3 w-3 shrink-0 text-muted-foreground" />
        </a>
        <div className="flex items-center gap-2 mt-0.5">
          <Badge className={cn('text-[10px] px-1.5 py-0', sourceColor(item.source_id))}>
            {item.source_id}
          </Badge>
          <span className="text-[10px] text-muted-foreground">
            {timeAgo(item.collected_at)}
          </span>
          {item.relevance_score != null && (
            <span className="text-[10px] text-muted-foreground">
              rel: {(item.relevance_score * 100).toFixed(0)}%
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

// --- Main Widget ---

interface PulseWidgetProps {
  className?: string;
  projectId?: string;
}

export function PulseWidget({ className, projectId }: PulseWidgetProps) {
  const queryClient = useQueryClient();

  // Project-scoped queries (when projectId is provided)
  const {
    data: stats,
    isLoading: statsLoading,
    error: statsError,
  } = useQuery({
    queryKey: pulseKeys.stats(projectId!),
    queryFn: () => pulseApi.getStats(projectId!),
    refetchInterval: 30000,
    enabled: !!projectId,
  });

  const {
    data: projectContent,
    isLoading: projectContentLoading,
  } = useQuery({
    queryKey: pulseKeys.contentLatest(projectId!),
    queryFn: () => pulseApi.getLatestContent(projectId!, 8),
    refetchInterval: 15000,
    enabled: !!projectId,
  });

  // Legacy queries (when no projectId — global overview)
  const {
    data: projects,
    isLoading: projectsLoading,
    error: projectsError,
  } = useQuery({
    queryKey: pulseKeys.projects(),
    queryFn: () => pulseApi.listProjects(),
    refetchInterval: 30000,
    enabled: !projectId,
  });

  const {
    data: legacyContent,
    isLoading: legacyContentLoading,
  } = useQuery({
    queryKey: pulseKeys.contentLegacy(),
    queryFn: () => pulseApi.getLatestContent('_all', 8),
    refetchInterval: 15000,
    enabled: !projectId,
  });

  const collectMutation = useMutation({
    mutationFn: () => {
      if (projectId) {
        return pulseApi.triggerCollection(projectId);
      }
      // Legacy: global collection
      return pulseApi.collectAll();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: pulseKeys.all() });
    },
  });

  const isLoading = projectId
    ? statsLoading || projectContentLoading
    : projectsLoading || legacyContentLoading;

  const error = projectId ? statsError : projectsError;

  const content = projectId ? projectContent : legacyContent;
  const contentItems = content?.items ?? [];

  const activeProjects = projects?.projects.filter((p) => p.scheduler_running).length ?? 0;
  const totalSources = projectId
    ? (stats?.active_sources ?? 0)
    : (projects?.projects.reduce((sum, p) => sum + p.adapters, 0) ?? 0);
  const alertCount = projectId
    ? (stats?.unacknowledged_alerts ?? 0)
    : contentItems.filter((i: any) => i.relevance_score != null && i.relevance_score > 0.8).length;

  if (error) {
    return (
      <Card className={cn('border-dashed', className)}>
        <CardHeader className="p-4 pb-2">
          <CardTitle className="text-sm flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Pulse Engine
          </CardTitle>
        </CardHeader>
        <CardContent className="p-4 pt-0">
          <p className="text-xs text-muted-foreground">
            Pulse Engine is not reachable. Check that the service is running.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="p-4 pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm flex items-center gap-2">
            <Activity className="h-4 w-4 text-green-500" />
            Pulse Engine
          </CardTitle>
          <Button
            variant="ghost"
            size="sm"
            className="h-7 w-7 p-0"
            onClick={() => collectMutation.mutate()}
            disabled={isLoading || collectMutation.isPending}
          >
            <RefreshCw
              className={cn('h-3.5 w-3.5', (isLoading || collectMutation.isPending) && 'animate-spin')}
            />
          </Button>
        </div>

        {/* Stats row */}
        <div className="flex items-center gap-4 mt-2">
          {!projectId && (
            <div className="flex items-center gap-1">
              <CheckCircle2 className="h-3 w-3 text-green-500" />
              <span className="text-xs text-muted-foreground">
                {activeProjects} active
              </span>
            </div>
          )}
          <div className="flex items-center gap-1">
            <Rss className="h-3 w-3 text-blue-500" />
            <span className="text-xs text-muted-foreground">
              {totalSources} sources
            </span>
          </div>
          {projectId && stats && (
            <div className="flex items-center gap-1">
              <Activity className="h-3 w-3 text-muted-foreground" />
              <span className="text-xs text-muted-foreground">
                {stats.total_content} items
              </span>
            </div>
          )}
          {alertCount > 0 && (
            <div className="flex items-center gap-1">
              <AlertTriangle className="h-3 w-3 text-amber-500" />
              <span className="text-xs text-amber-600">{alertCount} alerts</span>
            </div>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-4 pt-2">
        {isLoading ? (
          <div className="flex items-center justify-center py-6">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : (
          <>
            {/* Projects (legacy mode only) */}
            {!projectId && projects && projects.projects.length > 0 && (
              <div className="mb-3">
                <p className="text-xs font-medium text-muted-foreground mb-1">
                  Projects
                </p>
                <div className="divide-y divide-border/50">
                  {projects.projects.map((p) => (
                    <ProjectCard key={p.name} project={p} />
                  ))}
                </div>
              </div>
            )}

            {/* Recent content */}
            {contentItems.length > 0 && (
              <div>
                <p className="text-xs font-medium text-muted-foreground mb-1">
                  Recent Content
                </p>
                <div>
                  {contentItems.slice(0, 5).map((item: any) => (
                    <ContentRow key={item.content_hash || item.id} item={item} />
                  ))}
                </div>
                {contentItems.length > 5 && (
                  <p className="text-[10px] text-muted-foreground text-center mt-2">
                    +{contentItems.length - 5} more items
                  </p>
                )}
              </div>
            )}

            {contentItems.length === 0 &&
              (!projects || projects.projects.length === 0) && (
                <p className="text-xs text-muted-foreground text-center py-4">
                  No Pulse data available yet. Configure projects and run a
                  collection.
                </p>
              )}
          </>
        )}
      </CardContent>
    </Card>
  );
}
