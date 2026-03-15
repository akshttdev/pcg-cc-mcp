import { useMemo } from 'react';
import { useQueries } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { BarChart2, TrendingUp, Globe } from 'lucide-react';
import { socialApi, type SocialPostRecord } from '@/lib/api';
import { PLATFORM_ICONS, PLATFORM_COLORS } from '../../constants';

export function SocialAnalyticsView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const postQueries = useQueries({
    queries: projectEntries.map(e => ({
      queryKey: ['social-posts', e.id],
      queryFn: () => socialApi.listPostsFiltered({ projectId: e.id, status: 'published', limit: 100 }),
      staleTime: 120_000,
    })),
  });

  const agg = useMemo(() => {
    const posts: (SocialPostRecord & { _project: string })[] = [];
    postQueries.forEach((q, i) => {
      q.data?.forEach(p => posts.push({ ...p, _project: projectEntries[i].name }));
    });

    const totalImpressionsN = posts.reduce((s, p) => s + (p.impressions || 0), 0);
    const totalReachN = posts.reduce((s, p) => s + (p.reach || 0), 0);
    const totalLikes = posts.reduce((s, p) => s + (p.likes || 0), 0);
    const totalComments = posts.reduce((s, p) => s + (p.comments || 0), 0);
    const totalShares = posts.reduce((s, p) => s + (p.shares || 0), 0);
    const totalSaves = posts.reduce((s, p) => s + (p.saves || 0), 0);
    const avgEngagement = posts.length > 0 ? posts.reduce((s, p) => s + (p.engagement_rate || 0), 0) / posts.length : 0;

    const topPosts = [...posts]
      .sort((a, b) => (b.impressions || 0) - (a.impressions || 0))
      .slice(0, 10);

    const byPlatform: Record<string, { posts: number; impressions: number; likes: number; engagement: number }> = {};
    posts.forEach(p => {
      const platforms: string[] = (() => { try { return JSON.parse(p.platforms); } catch { return [p.platforms].filter(Boolean); } })();
      platforms.forEach(pl => {
        if (!byPlatform[pl]) byPlatform[pl] = { posts: 0, impressions: 0, likes: 0, engagement: 0 };
        byPlatform[pl].posts++;
        byPlatform[pl].impressions += p.impressions || 0;
        byPlatform[pl].likes += p.likes || 0;
        byPlatform[pl].engagement += p.engagement_rate || 0;
      });
    });
    Object.keys(byPlatform).forEach(pl => {
      if (byPlatform[pl].posts > 0) byPlatform[pl].engagement /= byPlatform[pl].posts;
    });

    return { totalImpressionsN, totalReachN, totalLikes, totalComments, totalShares, totalSaves, avgEngagement, topPosts, byPlatform, totalPosts: posts.length };
  }, [postQueries, projectEntries]);

  const fmt = (n: number) => n >= 1_000_000 ? `${(n/1_000_000).toFixed(1)}M` : n >= 1000 ? `${(n/1000).toFixed(1)}k` : n.toString();

  if (agg.totalPosts === 0) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        <BarChart2 className="h-10 w-10 mx-auto mb-3 opacity-40" />
        <p className="font-medium mb-1">No analytics data yet</p>
        <p className="text-xs">Publish posts to start tracking engagement metrics</p>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Summary KPIs */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        {[
          { label: 'Total Impressions', value: fmt(agg.totalImpressionsN), sub: `${agg.totalPosts} posts` },
          { label: 'Total Reach', value: fmt(agg.totalReachN), sub: 'unique accounts' },
          { label: 'Total Likes', value: fmt(agg.totalLikes), sub: `+ ${fmt(agg.totalComments)} comments` },
          { label: 'Avg Engagement', value: `${(agg.avgEngagement * 100).toFixed(2)}%`, sub: `${fmt(agg.totalShares)} shares` },
        ].map(({ label, value, sub }) => (
          <Card key={label} className="bg-card/80 backdrop-blur-sm border-border/50">
            <CardContent className="pt-5">
              <p className="text-xs text-muted-foreground mb-1">{label}</p>
              <p className="text-2xl font-bold">{value}</p>
              <p className="text-[11px] text-muted-foreground mt-0.5">{sub}</p>
            </CardContent>
          </Card>
        ))}
      </div>

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
            <div className="space-y-3">
              {Object.entries(agg.byPlatform).sort((a, b) => b[1].impressions - a[1].impressions).map(([platform, data]) => {
                const Icon = PLATFORM_ICONS[platform] || Globe;
                const maxImpressions = Math.max(...Object.values(agg.byPlatform).map(d => d.impressions));
                const pct = maxImpressions > 0 ? (data.impressions / maxImpressions) * 100 : 0;
                return (
                  <div key={platform}>
                    <div className="flex items-center gap-2 mb-1">
                      <Icon className={`h-3.5 w-3.5 shrink-0 ${PLATFORM_COLORS[platform] || 'text-muted-foreground'}`} />
                      <span className="text-xs capitalize flex-1">{platform}</span>
                      <span className="text-xs text-muted-foreground">{data.posts} posts</span>
                      <span className="text-xs font-medium">{fmt(data.impressions)} imp</span>
                      <span className="text-xs text-muted-foreground">{(data.engagement * 100).toFixed(1)}%</span>
                    </div>
                    <div className="h-1.5 rounded-full bg-muted overflow-hidden">
                      <div className={`h-full rounded-full ${PLATFORM_COLORS[platform]?.replace('text-', 'bg-') || 'bg-primary'}`} style={{ width: `${pct}%` }} />
                    </div>
                  </div>
                );
              })}
            </div>
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
            <ScrollArea className="h-[280px]">
              <div className="space-y-2 pr-2">
                {agg.topPosts.map((post, i) => {
                  const platforms: string[] = (() => { try { return JSON.parse(post.platforms); } catch { return [post.platforms].filter(Boolean); } })();
                  return (
                    <div key={post.id} className="flex items-start gap-2 p-2 rounded-lg hover:bg-muted/30 transition-colors">
                      <span className="text-xs text-muted-foreground font-mono w-4 shrink-0 mt-0.5">{i + 1}</span>
                      <div className="flex gap-0.5 shrink-0 mt-0.5">
                        {platforms.slice(0, 2).map(p => {
                          const Icon = PLATFORM_ICONS[p] || Globe;
                          return <Icon key={p} className={`h-3 w-3 ${PLATFORM_COLORS[p] || 'text-muted-foreground'}`} />;
                        })}
                      </div>
                      <div className="min-w-0 flex-1">
                        <p className="text-xs line-clamp-2">{post.caption || '(no caption)'}</p>
                        <div className="flex gap-2 mt-0.5 text-[10px] text-muted-foreground">
                          <span>&#128065; {fmt(post.impressions)}</span>
                          {post.likes > 0 && <span>&#9829; {post.likes}</span>}
                          {post.engagement_rate > 0 && <span>{(post.engagement_rate * 100).toFixed(1)}%</span>}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </ScrollArea>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
