import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  BookOpen,
  Building2,
  CheckCircle,
  Eye,
  EyeOff,
  FileText,
  Globe,
  Layers,
  RefreshCw,
  Zap,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { organizationsApi as orgApi } from '@/lib/api';
import type {
  ArtifactKnowledgeEntry,
  OrgKnowledgeSource,
} from '@/lib/api/communication';

const TYPE_ICONS: Record<string, React.ElementType> = {
  artifact: FileText,
  document: BookOpen,
  social: Globe,
  entity: Building2,
  brand_guide: Zap,
  research: RefreshCw,
  context_injection: Layers,
};

const TYPE_LABELS: Record<string, string> = {
  artifact: 'Deliverables',
  document: 'Documents',
  social: 'Social',
  entity: 'Entities',
  brand_guide: 'Brand Guide',
  research: 'Research',
  context_injection: 'Context',
};

function CoverageBar({ score }: { score: number }) {
  const pct = Math.round(score * 100);
  const color =
    pct >= 80 ? 'bg-green-500' : pct >= 50 ? 'bg-yellow-500' : 'bg-red-400';
  return (
    <div className="flex items-center gap-1.5">
      <div className="h-1.5 w-16 rounded-full bg-muted overflow-hidden">
        <div
          className={`h-full rounded-full ${color}`}
          style={{ width: `${pct}%` }}
        />
      </div>
      <span className="text-[10px] text-muted-foreground">{pct}%</span>
    </div>
  );
}

function KsCard({
  entry,
  showVisibilityToggle,
  onToggleVisibility,
  toggling,
}: {
  entry: (OrgKnowledgeSource | ArtifactKnowledgeEntry) & {
    _projectName?: string;
  };
  showVisibilityToggle?: boolean;
  onToggleVisibility?: (id: string, current: boolean) => void;
  toggling?: boolean;
}) {
  const Icon = TYPE_ICONS[entry.source_type] || FileText;
  return (
    <div className="flex items-start gap-3 p-3 rounded-lg hover:bg-muted/30 transition-colors border border-border/30">
      <div className="rounded-md p-1.5 bg-muted/60 shrink-0 mt-0.5">
        <Icon className="h-3.5 w-3.5 text-muted-foreground" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-start gap-2 justify-between">
          <p className="text-sm font-medium leading-snug truncate">
            {entry.source_title}
          </p>
          <div className="flex items-center gap-1.5 shrink-0">
            {entry.client_visible ? (
              <Badge
                variant="outline"
                className="text-[10px] px-1.5 py-0 border-green-500/40 text-green-600 bg-green-500/5"
              >
                <Eye className="h-2.5 w-2.5 mr-0.5" /> Client
              </Badge>
            ) : (
              <Badge
                variant="outline"
                className="text-[10px] px-1.5 py-0 text-muted-foreground"
              >
                <EyeOff className="h-2.5 w-2.5 mr-0.5" /> Internal
              </Badge>
            )}
            {showVisibilityToggle && onToggleVisibility && (
              <Button
                size="sm"
                variant="ghost"
                className="h-6 px-1.5 text-[10px]"
                disabled={toggling}
                onClick={() =>
                  onToggleVisibility(entry.id, entry.client_visible)
                }
              >
                {entry.client_visible ? 'Make internal' : 'Publish to client'}
              </Button>
            )}
          </div>
        </div>
        {entry.source_summary && (
          <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
            {entry.source_summary}
          </p>
        )}
        <div className="flex items-center gap-3 mt-1.5">
          <CoverageBar score={entry.coverage_score} />
          {'project_name' in entry && entry.project_name && (
            <span className="text-[10px] text-muted-foreground">
              {entry.project_name as string}
            </span>
          )}
          {'is_stale' in entry && (entry as OrgKnowledgeSource).is_stale && (
            <span className="text-[10px] text-yellow-600">Stale</span>
          )}
        </div>
      </div>
    </div>
  );
}

export function KnowledgeTab({ orgId }: { orgId: string }) {
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const qc = useQueryClient();

  const { data, isLoading } = useQuery({
    queryKey: ['org-knowledge', orgId],
    queryFn: () => orgApi.getKnowledge(orgId),
    staleTime: 60_000,
  });

  const toggleVisibilityMut = useMutation({
    mutationFn: ({ id, current }: { id: string; current: boolean }) =>
      orgApi.setKnowledgeVisibility(id, !current),
    onSuccess: () =>
      qc.invalidateQueries({ queryKey: ['org-knowledge', orgId] }),
  });

  const allEntries = [
    ...(data?.knowledge_entries ?? []),
    ...(data?.artifact_entries ?? []),
  ];

  const types = Array.from(new Set(allEntries.map((e) => e.source_type)));

  const filtered =
    typeFilter === 'all'
      ? allEntries
      : allEntries.filter((e) => e.source_type === typeFilter);

  const stats = data?.stats;
  const brand = data?.brand_summary as
    | Record<string, string | null>
    | null
    | undefined;

  return (
    <div className="p-6 space-y-6">
      {/* Stats */}
      {stats && (
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          {[
            {
              label: 'Total Entries',
              value: (
                stats.knowledge_entry_count + stats.artifact_count
              ).toString(),
            },
            { label: 'Deliverables', value: stats.artifact_count.toString() },
            {
              label: 'Client-visible',
              value: stats.client_visible_count.toString(),
            },
            {
              label: 'Avg Coverage',
              value: `${Math.round(stats.avg_coverage * 100)}%`,
            },
          ].map(({ label, value }) => (
            <Card key={label} className="bg-card/80 border-border/50">
              <CardContent className="pt-4 pb-3">
                <p className="text-xs text-muted-foreground">{label}</p>
                <p className="text-2xl font-semibold mt-0.5">{value}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Brand summary */}
      {brand &&
        (brand.tagline || brand.industry || brand.mission_statement) && (
          <Card className="bg-card/80 border-border/50">
            <CardHeader className="pb-2 pt-4 px-4">
              <CardTitle className="text-sm font-medium flex items-center gap-2">
                <Zap className="h-4 w-4 text-yellow-500" /> Brand Profile
                {brand.research_status === 'done' && (
                  <CheckCircle className="h-3.5 w-3.5 text-green-500 ml-auto" />
                )}
              </CardTitle>
            </CardHeader>
            <CardContent className="px-4 pb-4 grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
              {brand.tagline && (
                <div>
                  <p className="text-xs text-muted-foreground mb-0.5">
                    Tagline
                  </p>
                  <p className="italic">{brand.tagline}</p>
                </div>
              )}
              {brand.industry && (
                <div>
                  <p className="text-xs text-muted-foreground mb-0.5">
                    Industry
                  </p>
                  <p>{brand.industry}</p>
                </div>
              )}
              {brand.mission_statement && (
                <div className="md:col-span-2">
                  <p className="text-xs text-muted-foreground mb-0.5">
                    Mission
                  </p>
                  <p className="text-sm text-muted-foreground">
                    {brand.mission_statement}
                  </p>
                </div>
              )}
            </CardContent>
          </Card>
        )}

      {/* Knowledge entries */}
      <Card className="bg-card/80 border-border/50">
        <CardHeader className="pb-3 pt-4 px-4">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Layers className="h-4 w-4 text-blue-500" /> Knowledge Graph
          </CardTitle>
        </CardHeader>
        <CardContent className="px-4 pb-4 space-y-3">
          {/* Type filter */}
          {types.length > 1 && (
            <div className="flex gap-1 flex-wrap">
              <button
                onClick={() => setTypeFilter('all')}
                className={`px-2.5 py-1 text-xs rounded-md border transition-colors ${typeFilter === 'all' ? 'bg-primary/10 border-primary/30 text-primary' : 'border-border text-muted-foreground hover:text-foreground'}`}
              >
                All ({allEntries.length})
              </button>
              {types.map((t) => {
                const Icon = TYPE_ICONS[t] || FileText;
                const count = allEntries.filter(
                  (e) => e.source_type === t
                ).length;
                return (
                  <button
                    key={t}
                    onClick={() => setTypeFilter(t)}
                    className={`flex items-center gap-1 px-2.5 py-1 text-xs rounded-md border transition-colors ${typeFilter === t ? 'bg-primary/10 border-primary/30 text-primary' : 'border-border text-muted-foreground hover:text-foreground'}`}
                  >
                    <Icon className="h-3 w-3" />
                    {TYPE_LABELS[t] ?? t} ({count})
                  </button>
                );
              })}
            </div>
          )}

          {isLoading ? (
            <p className="text-sm text-muted-foreground text-center py-8">
              Loading…
            </p>
          ) : filtered.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-8">
              No knowledge entries yet
            </p>
          ) : (
            <div className="space-y-1.5">
              {filtered.map((entry) => (
                <KsCard
                  key={entry.id}
                  entry={entry}
                  showVisibilityToggle={entry.source_type === 'artifact'}
                  onToggleVisibility={(id, current) =>
                    toggleVisibilityMut.mutate({ id, current })
                  }
                  toggling={toggleVisibilityMut.isPending}
                />
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
