import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertTriangle,
  BookOpen,
  Building2,
  FolderOpen,
  RefreshCw,
  User,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { knowledgeApi, type UserKnowledgeSource } from '@/lib/api/intelligence';
import { knowledgeKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';

// ── Coverage ring (SVG, no charting library) ─────────────────────────────────

function CoverageRing({ pct }: { pct: number }) {
  const r = 36;
  const circ = 2 * Math.PI * r;
  return (
    <svg width="88" height="88" viewBox="0 0 88 88">
      <circle
        cx="44"
        cy="44"
        r={r}
        fill="none"
        stroke="hsl(var(--muted))"
        strokeWidth="8"
      />
      <circle
        cx="44"
        cy="44"
        r={r}
        fill="none"
        stroke="#6366f1"
        strokeWidth="8"
        strokeDasharray={`${(pct / 100) * circ} ${circ}`}
        strokeLinecap="round"
        transform="rotate(-90 44 44)"
      />
      <text
        x="44"
        y="49"
        textAnchor="middle"
        fontSize="14"
        fontWeight="600"
        fill="currentColor"
      >
        {Math.round(pct)}%
      </text>
    </svg>
  );
}

// ── Source type icons ─────────────────────────────────────────────────────────

const TYPE_ICONS: Record<string, typeof BookOpen> = {
  company: Building2,
  person: User,
  project: FolderOpen,
};

function getTypeIcon(sourceType: string): typeof BookOpen {
  return TYPE_ICONS[sourceType] ?? BookOpen;
}

// ── Source card ───────────────────────────────────────────────────────────────

interface SourceCardProps {
  src: UserKnowledgeSource;
  onRefresh: (id: string) => void;
  isRefreshing: boolean;
}

function SourceCard({ src, onRefresh, isRefreshing }: SourceCardProps) {
  const Icon = getTypeIcon(src.source_type);
  const coveragePct = Math.round(src.coverage_score * 100);

  return (
    <div
      className={cn(
        'rounded-lg border border-border/60 bg-card p-3 space-y-2',
        !src.is_active && 'opacity-50'
      )}
    >
      {/* Title row */}
      <div className="flex items-start gap-2">
        <Icon className="h-4 w-4 mt-0.5 shrink-0 text-muted-foreground" />
        <div className="flex-1 min-w-0">
          <p className="text-sm font-medium leading-tight truncate">
            {src.source_title}
          </p>
          <Badge
            variant="secondary"
            className="mt-0.5 h-4 text-[10px] px-1.5 capitalize"
          >
            {src.source_type}
          </Badge>
        </div>
        {src.is_stale && (
          <div className="flex items-center gap-1 shrink-0">
            <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />
            <Button
              size="sm"
              variant="ghost"
              className="h-6 text-xs px-2 text-amber-600 hover:text-amber-700"
              onClick={() => onRefresh(src.id)}
              disabled={isRefreshing}
            >
              <RefreshCw
                className={cn('h-3 w-3 mr-1', isRefreshing && 'animate-spin')}
              />
              Refresh
            </Button>
          </div>
        )}
      </div>

      {/* Summary */}
      {src.source_summary && (
        <p className="text-xs text-muted-foreground line-clamp-2">
          {src.source_summary}
        </p>
      )}

      {/* Coverage bar */}
      <div className="space-y-1">
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Coverage</span>
          <span className="text-[10px] text-muted-foreground">
            {coveragePct}%
          </span>
        </div>
        <div className="h-1.5 rounded-full bg-muted overflow-hidden">
          <div
            className="h-full rounded-full bg-indigo-500 transition-all"
            style={{ width: `${coveragePct}%` }}
          />
        </div>
      </div>
    </div>
  );
}

// ── Empty state ───────────────────────────────────────────────────────────────

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center gap-3">
      <BookOpen className="h-10 w-10 text-muted-foreground/30" />
      <div>
        <p className="text-sm font-medium text-muted-foreground">
          No personal knowledge sources yet
        </p>
        <p className="text-xs text-muted-foreground/60 mt-1 max-w-xs">
          Knowledge sources are created automatically as you research companies,
          people, and projects.
        </p>
      </div>
    </div>
  );
}

// ── MyIntelPanel ──────────────────────────────────────────────────────────────

export function MyIntelPanel() {
  const queryClient = useQueryClient();
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [refreshingId, setRefreshingId] = useState<string | null>(null);

  const { data, isLoading, isError } = useQuery({
    queryKey: knowledgeKeys.mine(),
    queryFn: () => knowledgeApi.getMyKnowledge(),
    staleTime: 60_000,
  });

  const refreshMutation = useMutation({
    mutationFn: (id: string) => knowledgeApi.refreshMySource(id),
    onMutate: (id) => setRefreshingId(id),
    onSettled: () => setRefreshingId(null),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: knowledgeKeys.mine() }),
  });

  if (isLoading) {
    return (
      <div className="space-y-2">
        {[1, 2, 3].map((i) => (
          <div key={i} className="h-20 rounded-lg bg-muted/40 animate-pulse" />
        ))}
      </div>
    );
  }

  if (isError || !data) {
    return (
      <p className="text-sm text-muted-foreground py-4">
        Failed to load knowledge sources.
      </p>
    );
  }

  const types = Object.keys(data.by_type);
  const allSources = types.flatMap((t) => data.by_type[t] ?? []);
  const filtered =
    typeFilter === 'all' ? allSources : (data.by_type[typeFilter] ?? []);

  const avgCoverage =
    allSources.length > 0
      ? (allSources.reduce((sum, s) => sum + s.coverage_score, 0) /
          allSources.length) *
        100
      : 0;

  return (
    <div className="space-y-4">
      {/* Header row */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-sm font-semibold">My Intel</h2>
          <p className="text-xs text-muted-foreground">
            {data.total} sources · {data.active} active
          </p>
        </div>
        <CoverageRing pct={avgCoverage} />
      </div>

      {/* Type filter pills */}
      {types.length > 0 && (
        <div className="flex gap-1.5 flex-wrap">
          {['all', ...types].map((t) => (
            <Button
              key={t}
              size="sm"
              variant={typeFilter === t ? 'default' : 'outline'}
              className="h-6 text-xs px-2 capitalize"
              onClick={() => setTypeFilter(t)}
            >
              {t}
            </Button>
          ))}
        </div>
      )}

      {/* Source list */}
      {filtered.length === 0 ? (
        <EmptyState />
      ) : (
        <ScrollArea className="h-[400px]">
          <div className="space-y-2 pr-2">
            {filtered.map((src) => (
              <SourceCard
                key={src.id}
                src={src}
                onRefresh={(id) => refreshMutation.mutate(id)}
                isRefreshing={refreshingId === src.id}
              />
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
