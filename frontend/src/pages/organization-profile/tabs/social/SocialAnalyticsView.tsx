import { useQueries, useQuery } from '@tanstack/react-query';
import { BarChart2, Globe, TrendingUp } from 'lucide-react';
import { useMemo, useState } from 'react';

import { BestTimesHeatmap } from '@/components/social/BestTimesHeatmap';
import { ConnectAccountButton } from '@/components/social/ConnectAccountButton';
import { NoraInsightsPanel } from '@/components/social/NoraInsightsPanel';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { socialApi, type SocialPostRecord } from '@/lib/api';
import { socialKeys } from '@/lib/query-keys';

import { PLATFORM_COLORS, PLATFORM_ICONS } from '../../constants';

// ── Inline Sparkline (no external chart lib) ──────────────────────────────────

function Sparkline({
  data,
  color = '#6366f1',
}: {
  data: number[];
  color?: string;
}) {
  if (data.length < 2) return null;
  const max = Math.max(...data, 1);
  const w = 300,
    h = 60,
    pad = 4;
  const pts = data
    .map((v, i) => {
      const x = pad + (i / (data.length - 1)) * (w - pad * 2);
      const y = h - pad - (v / max) * (h - pad * 2);
      return `${x},${y}`;
    })
    .join(' ');
  const firstX = pad;
  const lastX = w - pad;
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="w-full h-full" aria-hidden="true">
      <polyline
        points={pts}
        fill="none"
        stroke={color}
        strokeWidth="2"
        strokeLinejoin="round"
      />
      <polyline
        points={`${firstX},${h} ${pts} ${lastX},${h}`}
        fill={color}
        fillOpacity="0.12"
        stroke="none"
      />
    </svg>
  );
}

// ── View ──────────────────────────────────────────────────────────────────────

export function SocialAnalyticsView({
  projectEntries,
  orgId,
}: {
  projectEntries: { id: string; name: string }[];
  orgId?: string;
}) {
  const [days, setDays] = useState(30);

  // Fetch org-owned published posts when on the org analytics view
  const orgPostsQuery = useQuery({
    queryKey: ['social', 'posts', 'org', orgId, 'published'],
    queryFn: () =>
      socialApi.listPostsFiltered({
        organizationId: orgId!,
        status: 'published',
        limit: 200,
      }),
    enabled: !!orgId,
    staleTime: 120_000,
  });

  // Fetch published posts from every project for aggregate KPIs
  const postQueries = useQueries({
    queries: projectEntries.map((e) => ({
      queryKey: socialKeys.posts(e.id),
      queryFn: () =>
        socialApi.listPostsFiltered({
          projectId: e.id,
          status: 'published',
          limit: 100,
        }),
      staleTime: 120_000,
    })),
  });

  // Fetch time-series analytics from the first project for the sparkline
  const firstProjectId = projectEntries[0]?.id;
  const { data: analyticsData } = useQuery({
    queryKey: socialKeys.analytics(firstProjectId ?? '', days),
    queryFn: () => socialApi.getAnalytics(firstProjectId!, days),
    enabled: !!firstProjectId,
    staleTime: 5 * 60_000,
  });

  const agg = useMemo(() => {
    const posts: (SocialPostRecord & { _project: string })[] = [];

    // Org-owned posts go first when orgId is present
    orgPostsQuery.data?.forEach((p) =>
      posts.push({ ...p, _project: 'Org brand' })
    );

    postQueries.forEach((q, i) => {
      q.data?.forEach((p) =>
        posts.push({ ...p, _project: projectEntries[i].name })
      );
    });

    const totalImpressionsN = posts.reduce(
      (s, p) => s + (p.impressions || 0),
      0
    );
    const totalReachN = posts.reduce((s, p) => s + (p.reach || 0), 0);
    const totalLikes = posts.reduce((s, p) => s + (p.likes || 0), 0);
    const totalComments = posts.reduce((s, p) => s + (p.comments || 0), 0);
    const totalShares = posts.reduce((s, p) => s + (p.shares || 0), 0);
    const totalSaves = posts.reduce((s, p) => s + (p.saves || 0), 0);
    const avgEngagement =
      posts.length > 0
        ? posts.reduce((s, p) => s + (p.engagement_rate || 0), 0) / posts.length
        : 0;

    const topPosts = [...posts]
      .sort((a, b) => (b.impressions || 0) - (a.impressions || 0))
      .slice(0, 10);

    const byPlatform: Record<
      string,
      { posts: number; impressions: number; likes: number; engagement: number }
    > = {};
    posts.forEach((p) => {
      const platforms: string[] = (() => {
        try {
          return JSON.parse(p.platforms);
        } catch {
          return [p.platforms].filter(Boolean);
        }
      })();
      platforms.forEach((pl) => {
        if (!byPlatform[pl])
          byPlatform[pl] = {
            posts: 0,
            impressions: 0,
            likes: 0,
            engagement: 0,
          };
        byPlatform[pl].posts++;
        byPlatform[pl].impressions += p.impressions || 0;
        byPlatform[pl].likes += p.likes || 0;
        byPlatform[pl].engagement += p.engagement_rate || 0;
      });
    });
    Object.keys(byPlatform).forEach((pl) => {
      if (byPlatform[pl].posts > 0)
        byPlatform[pl].engagement /= byPlatform[pl].posts;
    });

    return {
      totalImpressionsN,
      totalReachN,
      totalLikes,
      totalComments,
      totalShares,
      totalSaves,
      avgEngagement,
      topPosts,
      byPlatform,
      totalPosts: posts.length,
    };
  }, [orgPostsQuery.data, postQueries, projectEntries]);

  const fmt = (n: number) =>
    n >= 1_000_000
      ? `${(n / 1_000_000).toFixed(1)}M`
      : n >= 1000
        ? `${(n / 1000).toFixed(1)}k`
        : n.toString();

  // Daily impressions array for the sparkline
  const dailyImpressions = analyticsData?.daily.map((d) => d.impressions) ?? [];

  if (agg.totalPosts === 0 && !analyticsData) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        <BarChart2 className="h-10 w-10 mx-auto mb-3 opacity-40" />
        <p className="font-medium mb-1">No analytics data yet</p>
        <p className="text-xs mb-4">
          Publish posts to start tracking engagement metrics
        </p>
        {(firstProjectId || orgId) && (
          <div className="flex justify-center">
            <ConnectAccountButton projectId={firstProjectId} orgId={orgId} />
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header row with period toggle + connect button */}
      <div className="flex items-center justify-between gap-3 flex-wrap">
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-muted-foreground font-medium">
            Period:
          </span>
          {([7, 30, 90] as const).map((d) => (
            <Button
              key={d}
              size="sm"
              variant={days === d ? 'default' : 'outline'}
              className="h-6 text-xs px-2"
              onClick={() => setDays(d)}
            >
              {d}d
            </Button>
          ))}
        </div>
        {(firstProjectId || orgId) && (
          <ConnectAccountButton projectId={firstProjectId} orgId={orgId} />
        )}
      </div>

      {/* Summary KPIs */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          {
            label: 'Total Impressions',
            value: fmt(agg.totalImpressionsN),
            sub: `${agg.totalPosts} posts`,
          },
          {
            label: 'Total Reach',
            value: fmt(agg.totalReachN),
            sub: 'unique accounts',
          },
          {
            label: 'Total Likes',
            value: fmt(agg.totalLikes),
            sub: `+ ${fmt(agg.totalComments)} comments`,
          },
          {
            label: 'Avg Engagement',
            value: `${(agg.avgEngagement * 100).toFixed(2)}%`,
            sub: `${fmt(agg.totalShares)} shares · ${fmt(agg.totalSaves)} saves`,
          },
        ].map(({ label, value, sub }) => (
          <Card
            key={label}
            className="bg-card/80 backdrop-blur-sm border-border/50"
          >
            <CardContent className="pt-5">
              <p className="text-xs text-muted-foreground mb-1">{label}</p>
              <p className="text-2xl font-semibold">{value}</p>
              <p className="text-xs text-muted-foreground mt-0.5">{sub}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      {/* Daily impressions sparkline (from getAnalytics) */}
      {dailyImpressions.length >= 2 && (
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2 pt-3 px-4">
            <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Daily Impressions — Last {days} days
              {(orgId || projectEntries[0]) && (
                <span className="ml-1 normal-case font-normal">
                  (
                  {orgId && !firstProjectId
                    ? 'Org brand'
                    : projectEntries[0]?.name}
                  )
                </span>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent className="px-4 pb-4">
            <div className="h-20 w-full">
              <Sparkline data={dailyImpressions} color="#6366f1" />
            </div>
            {analyticsData && (
              <div className="mt-2 flex justify-between text-xs text-muted-foreground">
                <span>{analyticsData.daily[0]?.date?.slice(5) ?? ''}</span>
                <span className="font-medium text-foreground">
                  Peak: {fmt(Math.max(...dailyImpressions))} imp.
                </span>
                <span>
                  {analyticsData.daily[
                    analyticsData.daily.length - 1
                  ]?.date?.slice(5) ?? ''}
                </span>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        {/* Platform breakdown */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <BarChart2 className="h-4 w-4 text-blue-500" />
              Platform Performance
            </CardTitle>
          </CardHeader>
          <CardContent>
            {Object.keys(agg.byPlatform).length === 0 ? (
              <p className="text-xs text-muted-foreground text-center py-4">
                No platform data
              </p>
            ) : (
              <div className="space-y-3">
                {Object.entries(agg.byPlatform)
                  .sort((a, b) => b[1].impressions - a[1].impressions)
                  .map(([platform, data]) => {
                    const Icon = PLATFORM_ICONS[platform] || Globe;
                    const maxImpressions = Math.max(
                      ...Object.values(agg.byPlatform).map((d) => d.impressions)
                    );
                    const pct =
                      maxImpressions > 0
                        ? (data.impressions / maxImpressions) * 100
                        : 0;
                    return (
                      <div key={platform}>
                        <div className="flex items-center gap-2 mb-1">
                          <Icon
                            className={`h-3.5 w-3.5 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`}
                          />
                          <span className="text-xs capitalize flex-1">
                            {platform}
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {data.posts} posts
                          </span>
                          <span className="text-xs font-medium">
                            {fmt(data.impressions)} imp
                          </span>
                          <span className="text-xs text-muted-foreground">
                            {(data.engagement * 100).toFixed(1)}%
                          </span>
                        </div>
                        <div className="h-1.5 rounded-full bg-muted overflow-hidden">
                          <div
                            className={`h-full rounded-full ${PLATFORM_COLORS[platform]?.replace('text-', 'bg-') || 'bg-primary'}`}
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                      </div>
                    );
                  })}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Top performing posts */}
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <TrendingUp className="h-4 w-4 text-green-500" />
              Top Posts
            </CardTitle>
          </CardHeader>
          <CardContent>
            {agg.topPosts.length === 0 ? (
              <p className="text-xs text-muted-foreground text-center py-4">
                No posts yet
              </p>
            ) : (
              <ScrollArea className="h-[280px]">
                <div className="space-y-2 pr-2">
                  {agg.topPosts.map((post, i) => {
                    const platforms: string[] = (() => {
                      try {
                        return JSON.parse(post.platforms);
                      } catch {
                        return [post.platforms].filter(Boolean);
                      }
                    })();
                    return (
                      <div
                        key={post.id}
                        className="flex items-start gap-2 p-2 rounded-lg hover:bg-muted/30 transition-colors"
                      >
                        <span className="text-xs text-muted-foreground font-mono w-4 shrink-0 mt-0.5">
                          {i + 1}
                        </span>
                        <div className="flex gap-0.5 shrink-0 mt-0.5">
                          {platforms.slice(0, 2).map((p) => {
                            const Icon = PLATFORM_ICONS[p] || Globe;
                            return (
                              <Icon
                                key={p}
                                className={`h-3 w-3 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`}
                              />
                            );
                          })}
                        </div>
                        <div className="min-w-0 flex-1">
                          <p className="text-xs line-clamp-2">
                            {post.caption || '(no caption)'}
                          </p>
                          <div className="flex gap-2 mt-0.5 text-xs text-muted-foreground">
                            <span>&#128065; {fmt(post.impressions)}</span>
                            {post.likes > 0 && (
                              <span>&#9829; {post.likes}</span>
                            )}
                            {post.engagement_rate > 0 && (
                              <span>
                                {(post.engagement_rate * 100).toFixed(1)}%
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>

      {/* Best times heatmap + Nora AI insights */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <BestTimesHeatmap projectId={firstProjectId} orgId={orgId} />
        <NoraInsightsPanel
          projectId={firstProjectId}
          orgId={orgId}
          days={days}
        />
      </div>
    </div>
  );
}
