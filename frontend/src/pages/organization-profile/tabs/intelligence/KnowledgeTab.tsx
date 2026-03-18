import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useQuery, useQueries } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  BookOpen,
  Brain,
  FolderOpen,
  FileText,
  MessageSquare,
  Radio,
  Pencil,
  Boxes,
  Network,
  AlertTriangle,
} from 'lucide-react';
import {
  organizationsApi,
  knowledgeApi,
} from '@/lib/api';
import { organizationKeys, knowledgeKeys } from '@/lib/query-keys';
import type { ProjectKnowledgeResponse, ProjectKnowledgeSource } from '@/lib/api';
import { DataSourcesIntelView } from './ArtifactsView';
import { DataSourcesView } from './DataSourcesView';
import { ArtifactsIntelView, ArtifactsView } from './ArtifactsView';
import { EditableWorkflowsView, SystemAutomationsSection, LegacyPipelinesView } from './WorkflowsView';
import { PulseView } from './PulseSection';
import { TopologyView } from './TopologyView';

// ── Source Type Metadata ─────────────────────────────────────────────────────

const SOURCE_TYPE_META: Record<string, { label: string; icon: typeof BookOpen }> = {
  conversation: { label: 'Conversations', icon: MessageSquare },
  artifact: { label: 'Artifacts', icon: FileText },
  pulse_content: { label: 'Pulse Content', icon: Radio },
  context_injection: { label: 'Context Injections', icon: Pencil },
  entity: { label: 'Entities', icon: Boxes },
  topology_snapshot: { label: 'Topology Snapshots', icon: Network },
};

// ── Knowledge Tab ────────────────────────────────────────────────────────────

export function KnowledgeTab({
  orgId,
  projectEntries,
  view,
}: {
  orgId: string;
  projectEntries: { id: string; name: string }[];
  view?: string | null;
}) {
  // Org-level knowledge (brand research, intelligence)
  const { data: orgKnowledge } = useQuery({
    queryKey: organizationKeys.knowledge(orgId),
    queryFn: () => organizationsApi.getKnowledge(orgId),
    staleTime: 60_000,
    enabled: !!orgId,
  });

  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: knowledgeKeys.project(entry.id),
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const isLoading = knowledgeQueries.some(q => q.isLoading);
  const loadedCount = knowledgeQueries.filter(q => q.isSuccess).length;

  const aggregated = useMemo(() => {
    let totalSources = 0;
    let staleSources = 0;
    let totalCoverage = 0;
    let coverageCount = 0;
    const sourcesByProject: { projectName: string; projectId: string; data: ProjectKnowledgeResponse }[] = [];
    const byType: Record<string, { source: ProjectKnowledgeSource; projectName: string; projectId: string }[]> = {};

    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      totalSources += q.data.total_sources;
      staleSources += q.data.stale_count;
      if (q.data.completeness) {
        totalCoverage += q.data.completeness.knowledge_completeness;
        coverageCount++;
      }
      if (q.data.total_sources > 0) {
        sourcesByProject.push({ projectName: entry.name, projectId: entry.id, data: q.data });
      }
      for (const [type, sources] of Object.entries(q.data.sources_by_type)) {
        if (!byType[type]) byType[type] = [];
        for (const source of sources) {
          byType[type].push({ source, projectName: entry.name, projectId: entry.id });
        }
      }
    });

    const orgEntryCount = orgKnowledge?.knowledge_entries?.length ?? 0;
    totalSources += orgEntryCount;
    const avgCompleteness = coverageCount > 0 ? Math.round((totalCoverage / coverageCount) * 100) : 0;
    return { totalSources, staleSources, avgCompleteness, sourcesByProject, byType };
  }, [knowledgeQueries, projectEntries, orgKnowledge]);

  // Deep view: "datasources" shows the new data sources table; others filter knowledge by type
  if (view && view !== 'overview') {
    if (view === 'datasources') {
      return (
        <div className="space-y-6">
          <DataSourcesIntelView orgId={orgId} projectEntries={projectEntries} />
          <DataSourcesView orgId={orgId} projectEntries={projectEntries} />
        </div>
      );
    }

    if (view === 'artifacts') {
      return (
        <div className="space-y-6">
          <ArtifactsIntelView projectEntries={projectEntries} />
          <ArtifactsView orgId={orgId} />
        </div>
      );
    }

    if (view === 'workflows') {
      return (
        <div className="space-y-8">
          <EditableWorkflowsView orgId={orgId} />
          <SystemAutomationsSection />
          <div className="border-t pt-6">
            <div className="flex items-center gap-2 mb-4">
              <Network className="h-5 w-5 text-muted-foreground" />
              <h2 className="text-lg font-semibold">Pipeline Blueprints</h2>
              <Badge variant="secondary" className="text-[10px]">Legacy</Badge>
            </div>
            <p className="text-sm text-muted-foreground mb-4">
              Agent-based pipeline templates for client engagements and production workflows. These are read-only blueprints — use the workflow editor above to build custom pipelines.
            </p>
            <LegacyPipelinesView orgId={orgId} />
          </div>
        </div>
      );
    }

    if (view === 'pulse') {
      return <PulseView projectEntries={projectEntries} />;
    }

    if (view === 'topology') {
      return <TopologyView projectEntries={projectEntries} aggregated={aggregated} isLoading={isLoading} loadedCount={loadedCount} />;
    }

    // Generic fallback for other knowledge source types (e.g. conversations)
    const typeKey =
      view === 'conversations' ? 'conversation'
      : null;
    const items = typeKey ? (aggregated.byType[typeKey] || []) : [];
    const meta = typeKey ? SOURCE_TYPE_META[typeKey] : null;
    const Icon = meta?.icon ?? BookOpen;

    return (
      <div className="space-y-4">
        <div className="flex items-center gap-2">
          <Icon className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">{meta?.label ?? view}</h2>
          <Badge variant="secondary">{items.length}</Badge>
          {isLoading && (
            <span className="text-xs text-muted-foreground ml-2">
              Loading {loadedCount}/{projectEntries.length} projects...
            </span>
          )}
        </div>
        {items.length === 0 && !isLoading ? (
          <div className="text-center py-12 text-muted-foreground">
            <Icon className="h-8 w-8 mx-auto mb-2 opacity-40" />
            <p>No {meta?.label.toLowerCase() ?? view} indexed yet</p>
          </div>
        ) : (
          <div className="space-y-2">
            {items.map(({ source, projectName, projectId }) => (
              <Card key={source.id} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardContent className="p-4">
                  <div className="flex items-start justify-between gap-3">
                    <div className="min-w-0">
                      <p className="font-medium text-sm truncate">{source.source_title}</p>
                      {source.source_summary && (
                        <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{source.source_summary}</p>
                      )}
                      <div className="flex items-center gap-2 mt-2">
                        <Link
                          to={`/projects/${projectId}/knowledge`}
                          className="text-xs text-blue-600 hover:underline flex items-center gap-1"
                        >
                          <FolderOpen className="h-3 w-3" />
                          {projectName}
                        </Link>
                        {source.is_stale && (
                          <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-300">stale</Badge>
                        )}
                      </div>
                    </div>
                    <div className="shrink-0 text-right">
                      <Progress value={Math.round(source.coverage_score * 100)} className="w-16 h-1.5 mb-1" />
                      <span className="text-xs text-muted-foreground">{Math.round(source.coverage_score * 100)}%</span>
                    </div>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    );
  }

  // Overview (default)
  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Avg Completeness</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-center gap-3">
              <Progress value={aggregated.avgCompleteness} className="flex-1 h-2" />
              <span className="text-2xl font-bold">{aggregated.avgCompleteness}%</span>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Total Sources</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex items-baseline gap-2">
              <span className="text-2xl font-bold">{aggregated.totalSources}</span>
              <span className="text-sm text-muted-foreground">across {projectEntries.length} projects</span>
            </div>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Stale Sources</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.staleSources > 0 ? 'text-yellow-600' : ''}`}>
              {aggregated.staleSources}
            </span>
          </CardContent>
        </Card>
      </div>

      {isLoading && (
        <p className="text-xs text-muted-foreground">Loading {loadedCount}/{projectEntries.length} projects...</p>
      )}

      {orgKnowledge?.knowledge_entries && orgKnowledge.knowledge_entries.length > 0 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              <Brain className="h-4 w-4 text-[hsl(var(--brand))]" />
              Organization Intelligence
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {orgKnowledge.knowledge_entries.map((entry) => (
                <Badge key={entry.id} variant="secondary" className="text-xs gap-1 cursor-default" title={entry.source_summary || entry.source_title}>
                  <Brain className="h-3 w-3" />
                  {entry.source_title}
                  {entry.coverage_score != null && (
                    <span className="opacity-60 ml-1">{Math.round(entry.coverage_score * 100)}%</span>
                  )}
                </Badge>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {aggregated.sourcesByProject.length === 0 && !isLoading && !(orgKnowledge?.knowledge_entries?.length) ? (
        <div className="text-center py-12 text-muted-foreground">
          <BookOpen className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p>No knowledge sources indexed yet</p>
          <p className="text-xs mt-1">Connect a data source in one of your projects to start building intelligence.</p>
        </div>
      ) : aggregated.sourcesByProject.length > 0 ? (
        <div className="space-y-4">
          {aggregated.sourcesByProject.map(({ projectName, projectId, data }) => {
            const completeness = data.completeness
              ? Math.round(data.completeness.knowledge_completeness * 100)
              : 0;
            return (
              <Card key={projectId} className="bg-card/80 backdrop-blur-sm border-border/50">
                <CardHeader className="pb-3">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-base flex items-center gap-2">
                      <FolderOpen className="h-4 w-4 text-muted-foreground" />
                      {projectName}
                    </CardTitle>
                    <div className="flex items-center gap-2">
                      <Progress value={completeness} className="w-20 h-1.5" />
                      <span className="text-xs text-muted-foreground">{completeness}%</span>
                      <Link
                        to={`/projects/${projectId}/knowledge`}
                        className="text-xs text-blue-600 hover:underline ml-2"
                      >
                        View
                      </Link>
                    </div>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="flex flex-wrap gap-2">
                    {Object.entries(data.sources_by_type).map(([type, sources]) => {
                      if (sources.length === 0) return null;
                      const meta = SOURCE_TYPE_META[type];
                      const Icon = meta?.icon || BookOpen;
                      return (
                        <Badge key={type} variant="secondary" className="text-xs gap-1">
                          <Icon className="h-3 w-3" />
                          {meta?.label || type} ({sources.length})
                        </Badge>
                      );
                    })}
                    {data.stale_count > 0 && (
                      <Badge variant="outline" className="text-xs text-yellow-600 border-yellow-300 gap-1">
                        <AlertTriangle className="h-3 w-3" />
                        {data.stale_count} stale
                      </Badge>
                    )}
                  </div>
                </CardContent>
              </Card>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}
