import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useQueries } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CardGrid } from '@/components/ui/card-grid';
import {
  Network,
  Loader2,
} from 'lucide-react';
import { knowledgeApi } from '@/lib/api';
import { knowledgeKeys } from '@/lib/query-keys';
import type { ProjectKnowledgeSource } from '@/lib/api';

// ── Topology View (used by KnowledgeTab) ─────────────────────────────────────

export function TopologyView({
  projectEntries,
  aggregated,
  isLoading,
  loadedCount,
}: {
  projectEntries: { id: string; name: string }[];
  aggregated: { byType: Record<string, { source: ProjectKnowledgeSource; projectName: string; projectId: string }[]> };
  isLoading: boolean;
  loadedCount: number;
}) {
  const topologyItems = aggregated.byType['topology_snapshot'] || [];

  if (isLoading && topologyItems.length === 0) return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
      <Loader2 className="h-4 w-4 animate-spin" />Loading topology data ({loadedCount}/{projectEntries.length} projects)...
    </div>
  );

  if (topologyItems.length === 0) return (
    <Card className="bg-card/80 border-border/50">
      <CardContent className="py-8 text-center">
        <Network className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
        <p className="text-sm text-muted-foreground">No topology snapshots in the knowledge graph yet.</p>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <Network className="h-5 w-5 text-muted-foreground" />
        <h2 className="text-lg font-semibold">Topology</h2>
        <Badge variant="secondary">{topologyItems.length}</Badge>
      </div>
      <CardGrid columns={{ md: 2 }} gap={3}>
        {topologyItems.map(({ source, projectName, projectId }) => (
          <Card key={source.id} className={`bg-card/80 ${source.is_stale ? 'border-yellow-500/30' : 'border-border/50'}`}>
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <Network className="h-4 w-4 text-[hsl(var(--info))] shrink-0 mt-0.5" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium truncate">{source.source_title}</p>
                  {source.source_summary && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{source.source_summary}</p>}
                  <div className="flex items-center justify-between mt-1.5">
                    <Link to={`/projects/${projectId}`} className="text-xs text-muted-foreground hover:text-foreground">{projectName}</Link>
                    <span className="text-xs text-muted-foreground">{Math.round(source.coverage_score * 100)}% coverage</span>
                  </div>
                  {source.is_stale && <Badge variant="outline" className="text-[9px] mt-1 text-yellow-600 border-yellow-600">Stale</Badge>}
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </CardGrid>
    </div>
  );
}

// ── Topology Intel View (used by intelligence tab) ───────────────────────────

export function TopologyIntelView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: knowledgeKeys.project(entry.id),
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const topologyItems = useMemo(() => {
    const all: { title: string; summary?: string; coverage?: number; projectName: string; projectId: string; isStale: boolean }[] = [];
    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      (q.data.sources_by_type?.topology_snapshot || []).forEach((src) => {
        all.push({ title: src.source_title, summary: src.source_summary, coverage: src.coverage_score, projectName: entry.name, projectId: entry.id, isStale: src.is_stale || false });
      });
    });
    return all;
  }, [knowledgeQueries, projectEntries]);

  const isLoading = knowledgeQueries.some(q => q.isLoading);

  if (isLoading) return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
      <Loader2 className="h-4 w-4 animate-spin" />Loading topology data...
    </div>
  );

  if (topologyItems.length === 0) return (
    <Card className="bg-card/80 border-border/50">
      <CardContent className="py-8 text-center">
        <Network className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
        <p className="text-sm text-muted-foreground">No topology snapshots in the knowledge graph yet.</p>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-3">
      <CardGrid columns={{ md: 2 }} gap={3}>
        {topologyItems.map((t, i) => (
          <Card key={i} className={`bg-card/80 ${t.isStale ? 'border-yellow-500/30' : 'border-border/50'}`}>
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <Network className="h-4 w-4 text-[hsl(var(--info))] shrink-0 mt-0.5" />
                <div className="min-w-0 flex-1">
                  <p className="text-sm font-medium truncate">{t.title}</p>
                  {t.summary && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{t.summary}</p>}
                  <div className="flex items-center justify-between mt-1.5">
                    <Link to={`/projects/${t.projectId}`} className="text-xs text-muted-foreground hover:text-foreground">{t.projectName}</Link>
                    {t.coverage != null && <span className="text-xs text-muted-foreground">{Math.round(t.coverage * 100)}% coverage</span>}
                  </div>
                  {t.isStale && <Badge variant="outline" className="text-[9px] mt-1 text-yellow-600 border-yellow-600">Stale</Badge>}
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </CardGrid>
    </div>
  );
}
