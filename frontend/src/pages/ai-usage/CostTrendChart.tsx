import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader } from '@/components/ui/loader';
import { formatCost, formatTokens } from '@/lib/format';

export interface DailyTrendEntry {
  date: string;
  tokens: number;
  cost: number;
  requests: number;
}

interface CostTrendChartProps {
  dailyTrend: DailyTrendEntry[];
  maxDailyTokens: number;
  isLoading: boolean;
}

export function CostTrendChart({ dailyTrend, maxDailyTokens, isLoading }: CostTrendChartProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-lg">Daily Usage Trend</CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex justify-center py-8">
            <Loader message="Loading trend data..." />
          </div>
        ) : dailyTrend.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">
            No usage data for this period
          </p>
        ) : (
          <div className="space-y-2">
            {dailyTrend.map((day) => (
              <div key={day.date} className="flex items-center gap-3">
                <span className="text-xs text-muted-foreground w-20 shrink-0">
                  {new Date(day.date).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
                </span>
                <div className="flex-1 h-6 bg-muted rounded-sm overflow-hidden">
                  <div
                    className="h-full bg-primary/80 rounded-sm transition-all"
                    style={{ width: `${(day.tokens / maxDailyTokens) * 100}%` }}
                  />
                </div>
                <span className="text-xs font-medium w-16 text-right">{formatTokens(day.tokens)}</span>
                <span className="text-xs text-muted-foreground w-16 text-right">{formatCost(day.cost)}</span>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
