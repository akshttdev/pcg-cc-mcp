import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { knowledgeApi } from '@/lib/api';
import { knowledgeKeys } from '@/lib/query-keys';
import type { ProjectKnowledgeSource, ProjectKnowledgeResponse } from '@/lib/api';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Loader } from '@/components/ui/loader';
import {
  BookOpen,
  MessageSquare,
  FileText,
  Radio,
  Pencil,
  Boxes,
  Network,
  RefreshCw,
  AlertTriangle,
  Eye,
} from 'lucide-react';

const SOURCE_TYPE_META: Record<string, { label: string; icon: typeof BookOpen }> = {
  conversation: { label: 'Conversations', icon: MessageSquare },
  artifact: { label: 'Artifacts', icon: FileText },
  pulse_content: { label: 'Pulse Content', icon: Radio },
  context_injection: { label: 'Context Injections', icon: Pencil },
  entity: { label: 'Entities', icon: Boxes },
  topology_snapshot: { label: 'Topology Snapshots', icon: Network },
};

const ALL_SOURCE_TYPES = [
  'conversation',
  'artifact',
  'pulse_content',
  'context_injection',
  'entity',
  'topology_snapshot',
];

function SourceCard({
  source,
  projectId,
  onView,
}: {
  source: ProjectKnowledgeSource;
  projectId: string;
  onView: (source: ProjectKnowledgeSource) => void;
}) {
  const queryClient = useQueryClient();

  const refreshMutation = useMutation({
    mutationFn: () => knowledgeApi.refreshSource(projectId, source.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: knowledgeKeys.project(projectId!) }),
  });

  const staleMutation = useMutation({
    mutationFn: () => knowledgeApi.markStale(projectId, source.id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: knowledgeKeys.project(projectId!) }),
  });

  return (
    <Card className="mb-2">
      <CardContent className="p-3">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 mb-1">
              <span className="text-sm font-medium truncate">{source.source_title}</span>
              {source.is_stale && (
                <Badge variant="outline" className="text-yellow-600 border-yellow-300 text-[10px]">
                  Stale
                </Badge>
              )}
              {source.auto_registered && (
                <Badge variant="outline" className="text-[10px]">
                  Auto
                </Badge>
              )}
            </div>
            {source.source_summary && (
              <p className="text-xs text-muted-foreground line-clamp-2 mb-2">
                {source.source_summary}
              </p>
            )}
            <div className="flex items-center gap-2">
              <Progress value={source.coverage_score * 100} className="h-1.5 flex-1 max-w-[120px]" />
              <span className="text-[10px] text-muted-foreground">
                {Math.round(source.coverage_score * 100)}% coverage
              </span>
            </div>
          </div>
          <div className="flex gap-1 shrink-0">
            {source.source_type === 'conversation' && source.source_id && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-muted-foreground"
                onClick={() => {
                  // Navigate to Nora meetings tab - deep link via URL hash
                  window.location.href = `/nora#meeting-${source.source_id}`;
                }}
              >
                <Eye className="h-3 w-3 mr-1" />
                Open
              </Button>
            )}
            {source.source_summary && source.source_type !== 'conversation' && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-muted-foreground"
                onClick={() => onView(source)}
              >
                <Eye className="h-3 w-3 mr-1" />
                View
              </Button>
            )}
            {source.source_summary && source.source_type === 'conversation' && (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-muted-foreground"
                onClick={() => onView(source)}
              >
                View
              </Button>
            )}
            {source.is_stale ? (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs"
                onClick={() => refreshMutation.mutate()}
                disabled={refreshMutation.isPending}
              >
                <RefreshCw className="h-3 w-3 mr-1" />
                Refresh
              </Button>
            ) : (
              <Button
                variant="ghost"
                size="sm"
                className="h-7 px-2 text-xs text-muted-foreground"
                onClick={() => staleMutation.mutate()}
                disabled={staleMutation.isPending}
              >
                <AlertTriangle className="h-3 w-3 mr-1" />
                Mark Stale
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

export function KnowledgePage() {
  const { projectId } = useParams<{ projectId: string }>();
  const [viewingSource, setViewingSource] = useState<ProjectKnowledgeSource | null>(null);

  const { data, isLoading, error } = useQuery<ProjectKnowledgeResponse>({
    queryKey: knowledgeKeys.project(projectId!),
    queryFn: () => knowledgeApi.getProjectKnowledge(projectId!),
    enabled: !!projectId,
  });

  if (!projectId) {
    return (
      <div className="container mx-auto p-6">
        <p className="text-muted-foreground">No project selected.</p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="container mx-auto p-6 flex items-center justify-center min-h-[200px]">
        <Loader message="Loading knowledge..." size={24} />
      </div>
    );
  }

  if (error || !data) {
    return (
      <div className="container mx-auto p-6">
        <p className="text-destructive">Failed to load knowledge data.</p>
      </div>
    );
  }

  const completeness = data.completeness;
  const completenessPercent = completeness
    ? Math.round(completeness.knowledge_completeness * 100)
    : 0;

  return (
    <div className="container mx-auto p-6 max-w-4xl">
      <div className="mb-6">
        <h1 className="text-2xl font-bold flex items-center gap-2">
          <BookOpen className="h-6 w-6" />
          Knowledge Sources
        </h1>
        <p className="text-muted-foreground">
          Unified view of all knowledge sources for this project
        </p>
      </div>

      {/* Overview Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Overall Completeness
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <Progress value={completenessPercent} className="flex-1 h-2" />
              <span className="text-2xl font-bold">{completenessPercent}%</span>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Source Types
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex gap-1 flex-wrap">
              {ALL_SOURCE_TYPES.map((type) => {
                const hasSources = data.sources_by_type[type]?.length > 0;
                return (
                  <Badge
                    key={type}
                    variant={hasSources ? 'default' : 'outline'}
                    className={`text-[10px] ${!hasSources ? 'opacity-40' : ''}`}
                  >
                    {SOURCE_TYPE_META[type]?.label || type}
                  </Badge>
                );
              })}
            </div>
            <p className="text-xs text-muted-foreground mt-2">
              {completeness?.type_count || 0} of 6 types covered
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">
              Sources
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-baseline gap-2">
              <span className="text-2xl font-bold">{data.total_sources}</span>
              <span className="text-sm text-muted-foreground">total</span>
            </div>
            {data.stale_count > 0 && (
              <p className="text-xs text-yellow-600 mt-1">
                {data.stale_count} stale source{data.stale_count !== 1 ? 's' : ''}
              </p>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Sources by type */}
      <Tabs defaultValue={Object.keys(data.sources_by_type)[0] || 'conversation'}>
        <TabsList className="mb-4">
          {ALL_SOURCE_TYPES.map((type) => {
            const count = data.sources_by_type[type]?.length || 0;
            const Icon = SOURCE_TYPE_META[type]?.icon || BookOpen;
            return (
              <TabsTrigger key={type} value={type} className="text-xs gap-1">
                <Icon className="h-3 w-3" />
                {SOURCE_TYPE_META[type]?.label || type}
                {count > 0 && (
                  <span className="text-[10px] ml-1 text-muted-foreground">({count})</span>
                )}
              </TabsTrigger>
            );
          })}
        </TabsList>

        {ALL_SOURCE_TYPES.map((type) => (
          <TabsContent key={type} value={type}>
            {(data.sources_by_type[type] || []).length === 0 ? (
              <div className="text-center py-8 text-muted-foreground text-sm">
                No {SOURCE_TYPE_META[type]?.label.toLowerCase() || type} sources registered yet.
              </div>
            ) : (
              data.sources_by_type[type].map((source) => (
                <SourceCard key={source.id} source={source} projectId={projectId} onView={setViewingSource} />
              ))
            )}
          </TabsContent>
        ))}
      </Tabs>

      {/* Full content viewer dialog */}
      <Dialog open={!!viewingSource} onOpenChange={(open) => { if (!open) setViewingSource(null); }}>
        <DialogContent className="max-w-3xl max-h-[80vh] flex flex-col">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base">
              {viewingSource && SOURCE_TYPE_META[viewingSource.source_type] && (
                (() => {
                  const Icon = SOURCE_TYPE_META[viewingSource.source_type].icon;
                  return <Icon className="h-4 w-4 text-muted-foreground shrink-0" />;
                })()
              )}
              {viewingSource?.source_title}
            </DialogTitle>
          </DialogHeader>
          <ScrollArea className="flex-1 mt-2">
            <pre className="text-sm whitespace-pre-wrap font-sans leading-relaxed p-1 pr-4">
              {viewingSource?.source_summary}
            </pre>
          </ScrollArea>
        </DialogContent>
      </Dialog>
    </div>
  );
}
