import { useQuery } from '@tanstack/react-query';
import {
  BarChart2,
  BookmarkIcon,
  Eye,
  Heart,
  MousePointerClick,
  Share2,
  TrendingUp,
  Users,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { socialApi } from '@/lib/api';
import { socialKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';

// ── Types ─────────────────────────────────────────────────────────────────────

interface AnalyticsDashboardProps {
  projectId: string;
  className?: string;
}

type DailyMetric = 'impressions' | 'likes';

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmt(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
  return n.toString();
}

function fmtDate(dateStr: string): string {
  return dateStr.slice(5); // MM-DD
}

// ── Sparkline SVG ─────────────────────────────────────────────────────────────

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
    <svg viewBox={`0 0 ${w} ${h}`} className="w-full h-full">
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
        fillOpacity="0.1"
        stroke="none"
      />
    </svg>
  );
}

// ── KPI Card ──────────────────────────────────────────────────────────────────

function KpiCard({
  label,
  value,
  sub,
  icon: Icon,
  iconColor,
}: {
  label: string;
  value: string | number;
  sub?: string;
  icon: React.ElementType;
  iconColor: string;
}) {
  const displayValue = typeof value === 'number' ? fmt(value) : value;
  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start justify-between gap-2">
          <div className="min-w-0">
            <p className="text-xs text-muted-foreground truncate">{label}</p>
            <p className="text-xl font-semibold mt-0.5">{displayValue}</p>
            {sub && (
              <p className="text-xs text-muted-foreground mt-0.5">{sub}</p>
            )}
          </div>
          <div className={cn('p-2 rounded-lg shrink-0', iconColor)}>
            <Icon className="h-3.5 w-3.5 text-white" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

// ── Horizontal bar ────────────────────────────────────────────────────────────

function HBar({
  value,
  max,
  colorClass = 'bg-primary',
}: {
  value: number;
  max: number;
  colorClass?: string;
}) {
  const pct = max > 0 ? Math.round((value / max) * 100) : 0;
  return (
    <div className="h-1.5 bg-muted rounded-full overflow-hidden">
      <div
        className={cn('h-full rounded-full', colorClass)}
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

// ── Main Component ────────────────────────────────────────────────────────────

export function AnalyticsDashboard({
  projectId,
  className,
}: AnalyticsDashboardProps) {
  const [days, setDays] = useState(30);
  const [dailyMetric, setDailyMetric] = useState<DailyMetric>('impressions');

  const { data, isLoading } = useQuery({
    queryKey: socialKeys.analytics(projectId, days),
    queryFn: () => socialApi.getAnalytics(projectId, days),
    staleTime: 5 * 60_000,
  });

  if (isLoading) {
    return (
      <div className={cn('space-y-4', className)}>
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
          {[...Array(6)].map((_, i) => (
            <Skeleton key={i} className="h-20" />
          ))}
        </div>
        <Skeleton className="h-48" />
        <Skeleton className="h-32" />
      </div>
    );
  }

  if (!data) return null;

  const maxImpressions = Math.max(
    ...data.by_platform.map((p) => p.total_impressions),
    1
  );

  const dailyValues = data.daily.map((d) =>
    dailyMetric === 'impressions' ? d.impressions : d.likes
  );

  const PLATFORM_BAR_COLORS: Record<string, string> = {
    linkedin: 'bg-[#0A66C2]',
    instagram: 'bg-[#E4405F]',
    twitter: 'bg-foreground',
    facebook: 'bg-[#1877F2]',
    youtube: 'bg-[#FF0000]',
    tiktok: 'bg-foreground',
  };

  // top posts from daily data (already sorted by impressions in analytics)
  const topPostsFromDaily = data.daily
    .slice()
    .sort((a, b) => b.impressions - a.impressions)
    .slice(0, 5);

  return (
    <div className={cn('space-y-5', className)}>
      {/* Header with period selector */}
      <div className="flex items-center gap-2">
        <BarChart2 className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">Analytics</span>
        <div className="ml-auto flex gap-1.5">
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
      </div>

      {/* KPI row — 6 cards */}
      <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
        <KpiCard
          label="Total Impressions"
          value={data.total_impressions}
          sub={`${data.posts_published} posts`}
          icon={Eye}
          iconColor="bg-blue-500"
        />
        <KpiCard
          label="Total Reach"
          value={data.total_reach}
          sub="unique accounts"
          icon={Users}
          iconColor="bg-sky-500"
        />
        <KpiCard
          label="Likes + Comments"
          value={data.total_likes + data.total_comments}
          sub={`${fmt(data.total_likes)} likes · ${fmt(data.total_comments)} comments`}
          icon={Heart}
          iconColor="bg-pink-500"
        />
        <KpiCard
          label="Shares + Saves"
          value={data.total_shares + data.total_saves}
          sub={`${fmt(data.total_shares)} shares · ${fmt(data.total_saves)} saves`}
          icon={Share2}
          iconColor="bg-green-500"
        />
        <KpiCard
          label="Avg Engagement Rate"
          value={`${(data.avg_engagement_rate * 100).toFixed(2)}%`}
          icon={TrendingUp}
          iconColor="bg-violet-500"
        />
        <KpiCard
          label="Clicks"
          value={data.total_clicks}
          sub={`${data.posts_published} posts published`}
          icon={MousePointerClick}
          iconColor="bg-amber-500"
        />
      </div>

      {/* Engagement over time chart */}
      {data.daily.length > 0 && (
        <Card>
          <CardHeader className="pb-2 pt-3 px-4">
            <div className="flex items-center justify-between">
              <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                Engagement Over Time — Last {days} days
              </CardTitle>
              <div className="flex gap-1">
                <Button
                  size="sm"
                  variant={dailyMetric === 'impressions' ? 'default' : 'ghost'}
                  className="h-5 text-xs px-2"
                  onClick={() => setDailyMetric('impressions')}
                >
                  Impressions
                </Button>
                <Button
                  size="sm"
                  variant={dailyMetric === 'likes' ? 'default' : 'ghost'}
                  className="h-5 text-xs px-2"
                  onClick={() => setDailyMetric('likes')}
                >
                  Likes
                </Button>
              </div>
            </div>
          </CardHeader>
          <CardContent className="px-4 pb-4">
            {/* SVG sparkline */}
            <div className="h-24 w-full mb-3">
              <Sparkline
                data={dailyValues}
                color={dailyMetric === 'impressions' ? '#6366f1' : '#ec4899'}
              />
            </div>
            {/* Mini bar grid for last 14 days */}
            <div className="space-y-1">
              {data.daily.slice(-14).map((d) => {
                const v =
                  dailyMetric === 'impressions' ? d.impressions : d.likes;
                const maxV = Math.max(...dailyValues.slice(-14), 1);
                return (
                  <div
                    key={d.date}
                    className="grid grid-cols-[68px,1fr,52px] gap-2 items-center"
                  >
                    <span className="text-xs text-muted-foreground font-mono">
                      {fmtDate(d.date)}
                    </span>
                    <HBar
                      value={v}
                      max={maxV}
                      colorClass={
                        dailyMetric === 'impressions'
                          ? 'bg-violet-500'
                          : 'bg-pink-500'
                      }
                    />
                    <span className="text-xs text-right text-muted-foreground tabular-nums">
                      {fmt(v)}
                    </span>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Platform breakdown */}
      {data.by_platform.length > 0 && (
        <Card>
          <CardHeader className="pb-2 pt-3 px-4">
            <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Platform Breakdown
            </CardTitle>
          </CardHeader>
          <CardContent className="px-4 pb-4 space-y-4">
            {data.by_platform
              .slice()
              .sort((a, b) => b.total_impressions - a.total_impressions)
              .map((p) => (
                <div key={p.platform}>
                  <div className="flex items-center justify-between mb-1 gap-2">
                    <Badge
                      variant="outline"
                      className="text-xs capitalize shrink-0"
                    >
                      {p.platform}
                    </Badge>
                    <div className="flex gap-3 text-xs text-muted-foreground">
                      <span>{p.posts_published} posts</span>
                      <span className="font-medium text-foreground">
                        {fmt(p.total_impressions)} imp.
                      </span>
                      <span>
                        {(p.avg_engagement_rate * 100).toFixed(1)}% eng.
                      </span>
                    </div>
                  </div>
                  <HBar
                    value={p.total_impressions}
                    max={maxImpressions}
                    colorClass={PLATFORM_BAR_COLORS[p.platform] ?? 'bg-primary'}
                  />
                </div>
              ))}
          </CardContent>
        </Card>
      )}

      {/* Top posts summary (from daily aggregates) */}
      {topPostsFromDaily.length > 0 && (
        <Card>
          <CardHeader className="pb-2 pt-3 px-4">
            <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wide flex items-center gap-1.5">
              <BookmarkIcon className="h-3.5 w-3.5" />
              Daily Peaks
            </CardTitle>
          </CardHeader>
          <CardContent className="px-4 pb-4 space-y-2">
            {topPostsFromDaily.map((d, i) => (
              <div
                key={d.date}
                className="flex items-center gap-3 text-xs p-2 rounded-md hover:bg-muted/30 transition-colors"
              >
                <span className="text-muted-foreground font-mono w-4 shrink-0">
                  {i + 1}
                </span>
                <span className="text-muted-foreground font-mono">
                  {fmtDate(d.date)}
                </span>
                <div className="flex gap-2 ml-auto text-muted-foreground">
                  <span className="flex items-center gap-0.5 text-foreground font-medium">
                    <Eye className="h-3 w-3" />
                    {fmt(d.impressions)}
                  </span>
                  {d.likes > 0 && (
                    <span className="flex items-center gap-0.5">
                      <Heart className="h-3 w-3 text-pink-500" />
                      {d.likes}
                    </span>
                  )}
                  {d.posts > 0 && (
                    <span className="text-muted-foreground">
                      {d.posts} post{d.posts !== 1 ? 's' : ''}
                    </span>
                  )}
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
