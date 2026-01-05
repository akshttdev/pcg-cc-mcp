import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Coins, TrendingUp, Clock, AlertTriangle } from 'lucide-react';
import { useQuery } from '@tanstack/react-query';
import { cn } from '@/lib/utils';

interface TokenUsageSummary {
  total_input_tokens: number;
  total_output_tokens: number;
  total_tokens: number;
  total_cost_cents: number | null;
  request_count: number;
}

interface TokenUsageByProject {
  project_id: string;
  project_name: string | null;
  total_tokens: number;
  request_count: number;
}

interface TokenUsageWidgetProps {
  className?: string;
  dailyLimit?: number;
  compact?: boolean;
}

async function fetchTodayUsage(): Promise<TokenUsageSummary> {
  const res = await fetch('/api/token-usage/today');
  if (!res.ok) throw new Error('Failed to fetch token usage');
  const data = await res.json();
  return data.data;
}

async function fetchUsageByProject(): Promise<TokenUsageByProject[]> {
  const res = await fetch('/api/token-usage/by-project?days=1');
  if (!res.ok) throw new Error('Failed to fetch project usage');
  const data = await res.json();
  return data.data;
}

function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
}

function getResetTime(): string {
  const now = new Date();
  const tomorrow = new Date(now);
  tomorrow.setDate(tomorrow.getDate() + 1);
  tomorrow.setHours(0, 0, 0, 0);

  const diff = tomorrow.getTime() - now.getTime();
  const hours = Math.floor(diff / (1000 * 60 * 60));
  const minutes = Math.floor((diff % (1000 * 60 * 60)) / (1000 * 60));

  return `${hours}h ${minutes}m`;
}

export function TokenUsageWidget({
  className,
  dailyLimit = 200000,
  compact,
}: TokenUsageWidgetProps) {
  const {
    data: todayUsage,
    isLoading: loadingToday,
  } = useQuery({
    queryKey: ['token-usage-today'],
    queryFn: fetchTodayUsage,
    refetchInterval: 30000, // Refresh every 30 seconds
  });

  const {
    data: projectUsage,
    isLoading: loadingProjects,
  } = useQuery({
    queryKey: ['token-usage-by-project'],
    queryFn: fetchUsageByProject,
    refetchInterval: 30000,
  });

  const isLoading = loadingToday || loadingProjects;

  if (isLoading) {
    return (
      <Card className={cn('animate-pulse', className)}>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Coins className="h-4 w-4" />
            Token Usage
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="h-4 w-32 bg-muted rounded mb-2" />
          <div className="h-2 w-full bg-muted rounded" />
        </CardContent>
      </Card>
    );
  }

  const totalTokens = todayUsage?.total_tokens ?? 0;
  const usagePercent = Math.min((totalTokens / dailyLimit) * 100, 100);
  const isNearLimit = usagePercent > 80;

  if (compact) {
    return (
      <div className={cn('flex items-center gap-2', className)}>
        <Coins className="h-4 w-4 text-muted-foreground" />
        <span className={cn(
          'text-sm font-medium',
          isNearLimit && 'text-orange-500'
        )}>
          {formatNumber(totalTokens)} / {formatNumber(dailyLimit)}
        </span>
        <Progress
          value={usagePercent}
          className="h-2 w-24"
          indicatorClassName={isNearLimit ? 'bg-orange-500' : 'bg-green-500'}
        />
      </div>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Coins className="h-4 w-4" />
            Token Usage
          </CardTitle>
          <Badge variant="outline" className="text-xs flex items-center gap-1">
            <Clock className="h-3 w-3" />
            Resets: {getResetTime()}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        {/* Today's usage */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-sm text-muted-foreground">Today</span>
            <span className={cn(
              'text-sm font-medium',
              isNearLimit && 'text-orange-500'
            )}>
              {formatNumber(totalTokens)} / {formatNumber(dailyLimit)}
            </span>
          </div>
          <Progress
            value={usagePercent}
            className="h-2"
            indicatorClassName={
              usagePercent > 90
                ? 'bg-red-500'
                : usagePercent > 80
                  ? 'bg-orange-500'
                  : 'bg-green-500'
            }
          />
          {isNearLimit && (
            <div className="flex items-center gap-1 text-xs text-orange-500">
              <AlertTriangle className="h-3 w-3" />
              <span>Approaching daily limit</span>
            </div>
          )}
        </div>

        {/* Breakdown */}
        {todayUsage && (
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="p-2 bg-muted rounded">
              <div className="text-muted-foreground">Input</div>
              <div className="font-medium">{formatNumber(todayUsage.total_input_tokens)}</div>
            </div>
            <div className="p-2 bg-muted rounded">
              <div className="text-muted-foreground">Output</div>
              <div className="font-medium">{formatNumber(todayUsage.total_output_tokens)}</div>
            </div>
          </div>
        )}

        {/* By Project */}
        {projectUsage && projectUsage.length > 0 && (
          <div className="pt-2 border-t">
            <div className="text-xs text-muted-foreground mb-2 flex items-center gap-1">
              <TrendingUp className="h-3 w-3" />
              By Project
            </div>
            <div className="space-y-1">
              {projectUsage.slice(0, 3).map((p) => (
                <div key={p.project_id} className="flex items-center justify-between text-xs">
                  <span className="truncate max-w-[120px]">
                    {p.project_name || 'Unknown'}
                  </span>
                  <span className="font-medium">{formatNumber(p.total_tokens)}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
