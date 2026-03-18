import type React from 'react';
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useQuery, useQueries } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import {
  FileText,
  Database,
  ExternalLink,
  Clock,
  ChevronDown,
  ChevronRight,
  Loader2,
} from 'lucide-react';
import {
  dataSourcesApi,
  workflowsApi,
  knowledgeApi,
  type ExecutionArtifact,
  type DataSourceRecord,
} from '@/lib/api';
import { dataSourceKeys, knowledgeKeys } from '@/lib/query-keys';

// ── Artifacts View (execution artifacts accordion) ──────────────────────────

export function ArtifactsView({ orgId }: { orgId: string }) {
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const { data: artifacts = [], isLoading } = useQuery({
    queryKey: dataSourceKeys.recentArtifacts(),
    queryFn: () => workflowsApi.listRecentArtifacts(),
  });

  const toggleExpand = (id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const parseContent = (content?: string) => {
    if (!content) return null;
    try { return JSON.parse(content); } catch { return content; }
  };

  const renderValue = (val: unknown): React.ReactNode => {
    if (val === null || val === undefined) return <span className="text-muted-foreground italic">null</span>;
    if (typeof val === 'string') return <span className="text-sm">{val}</span>;
    if (typeof val === 'number' || typeof val === 'boolean') return <span className="text-sm font-mono">{String(val)}</span>;
    if (Array.isArray(val)) {
      return (
        <div className="ml-3 space-y-1">
          {val.map((item: unknown, i: number) => (
            <div key={i} className="text-sm border-l-2 border-border/50 pl-2">
              {typeof item === 'object' ? renderValue(item) : String(item)}
            </div>
          ))}
        </div>
      );
    }
    if (typeof val === 'object') {
      return (
        <div className="ml-3 space-y-1">
          {Object.entries(val as Record<string, unknown>).map(([k, v]) => (
            <div key={k}>
              <span className="text-xs font-medium text-muted-foreground uppercase tracking-wide">{k.replace(/_/g, ' ')}: </span>
              {typeof v === 'object' && v !== null ? renderValue(v) : <span className="text-sm">{String(v ?? '')}</span>}
            </div>
          ))}
        </div>
      );
    }
    return <span>{String(val)}</span>;
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-48">
        <div className="text-sm text-muted-foreground">Loading artifacts...</div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2">
        <FileText className="h-5 w-5 text-muted-foreground" />
        <h2 className="text-lg font-semibold">Artifacts</h2>
        <Badge variant="secondary">{artifacts.length}</Badge>
      </div>

      {artifacts.length === 0 ? (
        <div className="text-center py-12 text-muted-foreground">
          <FileText className="h-8 w-8 mx-auto mb-2 opacity-40" />
          <p className="text-sm">No artifacts generated yet.</p>
          <p className="text-xs mt-1">Run a workflow on a data source to generate artifacts.</p>
        </div>
      ) : (
        <div className="space-y-2">
          {artifacts.map((artifact: ExecutionArtifact) => {
            const isExpanded = expandedIds.has(artifact.id);
            const meta = artifact.metadata ? (() => { try { return JSON.parse(artifact.metadata); } catch { return {}; } })() : {};
            const content = parseContent(artifact.content ?? undefined);

            return (
              <Card key={artifact.id} className="bg-card/80 border-border/50">
                <button
                  className="flex items-center justify-between w-full p-4 text-left hover:bg-muted/30 transition-colors"
                  onClick={() => toggleExpand(artifact.id)}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    {isExpanded ? <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" /> : <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />}
                    <div className="min-w-0">
                      <p className="text-sm font-medium truncate">{artifact.title || 'Untitled Artifact'}</p>
                      <div className="flex items-center gap-2 mt-0.5">
                        <Badge variant="outline" className="text-[10px]">{artifact.artifact_type}</Badge>
                        {meta.step_id && <span className="text-xs text-muted-foreground">{meta.step_id.replace(/_/g, ' ')}</span>}
                      </div>
                    </div>
                  </div>
                  <span className="text-xs text-muted-foreground shrink-0 ml-2 flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {new Date(artifact.created_at).toLocaleDateString('en-US', { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' })}
                  </span>
                </button>
                {isExpanded && (
                  <div className="px-4 pb-4 border-t">
                    <div className="mt-3">
                      {content ? renderValue(content) : <p className="text-sm text-muted-foreground italic">No content.</p>}
                    </div>
                    {meta.data_source_id && (
                      <div className="mt-3 pt-2 border-t border-border/30">
                        <Link
                          to={`/organizations/${orgId}/data-sources/${meta.data_source_id}`}
                          className="text-xs text-blue-600 hover:underline"
                        >
                          View source data source
                        </Link>
                      </div>
                    )}
                  </div>
                )}
              </Card>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── Data Sources Intel View (summary cards) ──────────────────────────────────

export function DataSourcesIntelView({ orgId, projectEntries }: { orgId: string; projectEntries: { id: string; name: string }[] }) {
  const { data: orgSources = [], isLoading: orgLoading } = useQuery({
    queryKey: dataSourceKeys.list(orgId),
    queryFn: () => dataSourcesApi.listByOrganization(orgId),
    staleTime: 60_000,
  });

  const projSourceQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: dataSourceKeys.project(entry.id),
      queryFn: () => dataSourcesApi.listByProject(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const isLoading = orgLoading || projSourceQueries.some(q => q.isLoading);

  const sources = useMemo(() => {
    const projSources = projSourceQueries.flatMap(q => q.data || []);
    const all: DataSourceRecord[] = [...orgSources, ...projSources];
    const seen = new Set<string>();
    return all.filter(s => {
      if (seen.has(s.id)) return false;
      seen.add(s.id);
      return true;
    });
  }, [orgSources, projSourceQueries]);

  const typeCount = useMemo(() => {
    const counts: Record<string, number> = {};
    sources.forEach((s) => { counts[s.data_type] = (counts[s.data_type] || 0) + 1; });
    return counts;
  }, [sources]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-muted-foreground">Data Library Summary</h3>
        <Link
          to={`/organizations/${orgId}/data-sources`}
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
        >
          <Database className="h-3.5 w-3.5" />
          Open Full Library
          <ExternalLink className="h-3 w-3" />
        </Link>
      </div>
      {isLoading ? (
        <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
          <Loader2 className="h-4 w-4 animate-spin" />Loading data sources...
        </div>
      ) : sources.length === 0 ? (
        <Card className="bg-card/80 border-border/50">
          <CardContent className="py-8 text-center">
            <Database className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
            <p className="text-sm text-muted-foreground">No data sources yet.</p>
            <Link to={`/organizations/${orgId}/intelligence/data-sources`} className="text-sm text-primary hover:underline mt-1 inline-block">
              Add your first data source →
            </Link>
          </CardContent>
        </Card>
      ) : (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {Object.entries(typeCount).map(([type, count]) => (
            <Card key={type} className="bg-card/80 border-border/50">
              <CardContent className="pt-4 pb-4">
                <p className="text-xs text-muted-foreground capitalize">{type.replace(/_/g, ' ')}</p>
                <p className="text-2xl font-bold mt-1">{count as number}</p>
              </CardContent>
            </Card>
          ))}
          <Card className="bg-primary/5 border-primary/20">
            <CardContent className="pt-4 pb-4">
              <p className="text-xs text-muted-foreground">Total Files</p>
              <p className="text-2xl font-bold mt-1 text-primary">{sources.length}</p>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}

// ── Artifacts Intel View (knowledge graph artifacts summary) ───────────────

export function ArtifactsIntelView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const knowledgeQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: knowledgeKeys.project(entry.id),
      queryFn: () => knowledgeApi.getProjectKnowledge(entry.id),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const artifacts = useMemo(() => {
    const all: { title: string; summary?: string; projectName: string; projectId: string }[] = [];
    knowledgeQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      (q.data.sources_by_type?.artifact || []).forEach((src) => {
        all.push({ title: src.source_title, summary: src.source_summary, projectName: entry.name, projectId: entry.id });
      });
    });
    return all;
  }, [knowledgeQueries, projectEntries]);

  const isLoading = knowledgeQueries.some(q => q.isLoading);

  if (isLoading) return (
    <div className="flex items-center gap-2 text-muted-foreground text-sm py-4">
      <Loader2 className="h-4 w-4 animate-spin" />Loading knowledge graph artifacts...
    </div>
  );

  if (artifacts.length === 0) return (
    <Card className="bg-card/80 border-border/50">
      <CardContent className="py-8 text-center">
        <FileText className="h-8 w-8 text-muted-foreground mx-auto mb-2" />
        <p className="text-sm text-muted-foreground">No artifacts in the knowledge graph yet.</p>
        <p className="text-xs text-muted-foreground mt-1">Artifacts are added automatically when deliverables are marked done.</p>
      </CardContent>
    </Card>
  );

  return (
    <div className="space-y-3">
      <h3 className="text-sm font-medium text-muted-foreground">Knowledge Graph Artifacts</h3>
      <p className="text-xs text-muted-foreground">{artifacts.length} artifact{artifacts.length !== 1 ? 's' : ''} across {new Set(artifacts.map(a => a.projectId)).size} project{new Set(artifacts.map(a => a.projectId)).size !== 1 ? 's' : ''}</p>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {artifacts.map((a, i) => (
          <Card key={i} className="bg-card/80 border-border/50">
            <CardContent className="pt-4 pb-4">
              <div className="flex items-start gap-2">
                <FileText className="h-4 w-4 text-[hsl(var(--brand))] shrink-0 mt-0.5" />
                <div className="min-w-0">
                  <p className="text-sm font-medium truncate">{a.title}</p>
                  {a.summary && <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">{a.summary}</p>}
                  <Link to={`/projects/${a.projectId}`} className="text-[10px] text-muted-foreground hover:text-foreground mt-1 block">{a.projectName}</Link>
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </div>
  );
}
