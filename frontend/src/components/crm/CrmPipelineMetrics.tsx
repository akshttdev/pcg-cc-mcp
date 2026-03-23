import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CardGrid } from '@/components/ui/card-grid';
import { Skeleton } from '@/components/ui/skeleton';
import {
  DollarSign,
  TrendingUp,
  Target,
  BarChart3,
  Trophy,
} from 'lucide-react';
import { crmDealsApi } from '@/lib/api';
import { crmKeys } from '@/lib/query-keys';
import type { PipelineMetricsRecord } from '@/lib/api';
import { formatCurrencyFull } from '@/lib/formatters';

interface CrmPipelineMetricsProps {
  projectId: string;
  pipelineId?: string;
}

export function CrmPipelineMetrics({
  projectId,
  pipelineId,
}: CrmPipelineMetricsProps) {
  const { data: metrics, isLoading } = useQuery<PipelineMetricsRecord>({
    queryKey: crmKeys.metrics(projectId, pipelineId),
    queryFn: () => crmDealsApi.getMetrics(projectId, pipelineId),
    enabled: !!projectId,
    staleTime: 60 * 1000,
  });

  if (isLoading) {
    return (
      <CardGrid columns={{ sm: 2, md: 3, lg: 5 }} gap={4}>
        {[1, 2, 3, 4, 5].map((i) => (
          <Skeleton key={i} className="h-24" />
        ))}
      </CardGrid>
    );
  }

  if (!metrics) return null;

  return (
    <div className="space-y-6">
      {/* Summary Cards */}
      <CardGrid columns={{ sm: 2, md: 3, lg: 5 }} gap={4}>
        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-blue-100 rounded-lg">
                <BarChart3 className="h-5 w-5 text-blue-600" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Total Deals</p>
                <p className="text-2xl font-bold">{metrics.total_deals}</p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-green-100 rounded-lg">
                <DollarSign className="h-5 w-5 text-green-600" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Pipeline Value</p>
                <p className="text-2xl font-bold">
                  {formatCurrencyFull(metrics.total_value)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-purple-100 rounded-lg">
                <TrendingUp className="h-5 w-5 text-purple-600" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Weighted Value</p>
                <p className="text-2xl font-bold">
                  {formatCurrencyFull(metrics.weighted_value)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-amber-100 rounded-lg">
                <Target className="h-5 w-5 text-amber-600" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Avg Deal Size</p>
                <p className="text-2xl font-bold">
                  {formatCurrencyFull(metrics.avg_deal_size)}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardContent className="pt-6">
            <div className="flex items-center gap-3">
              <div className="p-2 bg-teal-100 rounded-lg">
                <Trophy className="h-5 w-5 text-teal-600" />
              </div>
              <div>
                <p className="text-xs text-muted-foreground">Win Rate</p>
                <p className="text-2xl font-bold">
                  {Math.round(metrics.win_rate * 100)}%
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      </CardGrid>

      {/* Pipeline Funnel */}
      {metrics.deals_by_stage.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Deals by Stage</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-3">
              {metrics.deals_by_stage.map((stage) => {
                const maxCount = Math.max(
                  ...metrics.deals_by_stage.map((s) => s.count)
                );
                const widthPct = maxCount > 0 ? (stage.count / maxCount) * 100 : 0;

                return (
                  <div key={stage.stage_id} className="space-y-1">
                    <div className="flex items-center justify-between text-sm">
                      <span>{stage.stage_name}</span>
                      <span className="text-muted-foreground">
                        {stage.count} deals &middot;{' '}
                        {formatCurrencyFull(stage.total_value)}
                      </span>
                    </div>
                    <div className="h-2 bg-muted rounded-full overflow-hidden">
                      <div
                        className="h-full bg-primary rounded-full transition-all"
                        style={{ width: `${widthPct}%` }}
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
