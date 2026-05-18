import { useQuery } from '@tanstack/react-query';
import {
  BarChart2,
  Eye,
  Heart,
  MessageSquare,
  Share2,
  TrendingUp,
} from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Skeleton } from '@/components/ui/skeleton';
import { socialApi } from '@/lib/api';
import { cn } from '@/lib/utils';

interface AnalyticsDashboardProps {
  projectId: string;
  className?: string;
}

function MetricCard({
  label,
  value,
  icon: Icon,
  color,
}: {
  label: string;
  value: number | string;
  icon: React.ElementType;
  color: string;
}) {
  return (
    <Card>
      <CardContent className="p-4 flex items-center gap-3">
        <div className={cn('p-2 rounded-lg', color)}>
          <Icon className="h-4 w-4 text-white" />
        </div>
        <div>
          <p className="text-xs text-muted-foreground">{label}</p>
          <p className="text-lg font-semibold">
            {typeof value === 'number' ? value.toLocaleString() : value}
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

function MiniBar({ value, max }: { value: number; max: number }) {
  const pct = max > 0 ? Math.round((value / max) * 100) : 0;
  return (
    <div className="h-1.5 bg-muted rounded-full overflow-hidden">
      <div
        className="h-full bg-primary rounded-full"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

export function AnalyticsDashboard({
  projectId,
  className,
}: AnalyticsDashboardProps) {
  const [days, setDays] = useState(30);

  const { data, isLoading } = useQuery({
    queryKey: ['social-analytics', projectId, days],
    queryFn: () => socialApi.getAnalytics(projectId, days),
    staleTime: 5 * 60_000,
  });

  if (isLoading) {
    return (
      <div className={cn('space-y-4', className)}>
        <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
          {[...Array(4)].map((_, i) => (
            <Skeleton key={i} className="h-20" />
          ))}
        </div>
        <Skeleton className="h-48" />
      </div>
    );
  }

  if (!data) return null;

  const maxImpressions = Math.max(...data.daily.map((d) => d.impressions), 1);

  return (
    <div className={cn('space-y-5', className)}>
      {/* Period selector */}
      <div className="flex items-center gap-2">
        <BarChart2 className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">Analytics</span>
        <div className="ml-auto flex gap-1.5">
          {[7, 30, 90].map((d) => (
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

      {/* Summary cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3">
        <MetricCard
          label="Impressions"
          value={data.total_impressions}
          icon={Eye}
          color="bg-blue-500"
        />
        <MetricCard
          label="Likes"
          value={data.total_likes}
          icon={Heart}
          color="bg-pink-500"
        />
        <MetricCard
          label="Comments"
          value={data.total_comments}
          icon={MessageSquare}
          color="bg-purple-500"
        />
        <MetricCard
          label="Shares"
          value={data.total_shares}
          icon={Share2}
          color="bg-green-500"
        />
      </div>

      {/* Engagement rate + post count */}
      <div className="grid grid-cols-2 gap-3">
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-2 mb-1">
              <TrendingUp className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-xs text-muted-foreground">
                Avg Engagement Rate
              </span>
            </div>
            <p className="text-2xl font-bold">
              {(data.avg_engagement_rate * 100).toFixed(2)}%
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-4">
            <div className="flex items-center gap-2 mb-1">
              <BarChart2 className="h-3.5 w-3.5 text-muted-foreground" />
              <span className="text-xs text-muted-foreground">
                Posts Published
              </span>
            </div>
            <p className="text-2xl font-bold">{data.posts_published}</p>
          </CardContent>
        </Card>
      </div>

      {/* Daily impressions sparkline */}
      {data.daily.length > 0 && (
        <Card>
          <CardHeader className="pb-2 pt-3 px-4">
            <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              Daily Impressions — Last {days} days
            </CardTitle>
          </CardHeader>
          <CardContent className="px-4 pb-4">
            <div className="space-y-1.5">
              {data.daily.slice(-14).map((d) => (
                <div
                  key={d.date}
                  className="grid grid-cols-[80px,1fr,56px] gap-2 items-center"
                >
                  <span className="text-xs text-muted-foreground">
                    {d.date.slice(5)}
                  </span>
                  <MiniBar value={d.impressions} max={maxImpressions} />
                  <span className="text-xs text-right text-muted-foreground">
                    {d.impressions.toLocaleString()}
                  </span>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* By platform */}
      {data.by_platform.length > 0 && (
        <Card>
          <CardHeader className="pb-2 pt-3 px-4">
            <CardTitle className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
              By Platform
            </CardTitle>
          </CardHeader>
          <CardContent className="px-4 pb-4 space-y-3">
            {data.by_platform.map((p) => (
              <div
                key={p.platform}
                className="flex items-center justify-between gap-3"
              >
                <Badge variant="outline" className="text-xs capitalize">
                  {p.platform}
                </Badge>
                <div className="flex-1 grid grid-cols-3 gap-2 text-xs text-muted-foreground text-right">
                  <span>{p.posts_published} posts</span>
                  <span>{p.total_impressions.toLocaleString()} impr.</span>
                  <span>{(p.avg_engagement_rate * 100).toFixed(1)}% eng.</span>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
