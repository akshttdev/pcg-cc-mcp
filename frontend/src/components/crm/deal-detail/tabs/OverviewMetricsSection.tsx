import { CheckSquare, DollarSign, FileText, TrendingUp } from 'lucide-react';

import { CardGrid } from '@/components/ui/card-grid';
import { MetricCard } from '@/components/ui/metric-card';
import { cn } from '@/lib/utils';
import type { CrmDealWithContact } from '@/types/crm';

interface OverviewMetricsSectionProps {
  deal: CrmDealWithContact;
  stageColor: string;
}

export function OverviewMetricsSection({ deal, stageColor }: OverviewMetricsSectionProps) {
  const taskTotal = deal.task_total ?? 0;
  const taskDone = deal.task_done ?? 0;
  const taskPct = taskTotal > 0 ? Math.round((taskDone / taskTotal) * 100) : 0;

  return (
    <CardGrid columns={{ sm: 2 }} gap={3}>
      <MetricCard
        label="Probability"
        value={deal.probability > 0 ? `${deal.probability}%` : '\u2014'}
        icon={TrendingUp}
        accent={stageColor}
        sub={
          deal.probability > 0 ? (
            <div className="mt-1.5 h-1 bg-muted rounded-full overflow-hidden">
              <div
                className="h-full rounded-full"
                style={{
                  width: `${deal.probability}%`,
                  backgroundColor: stageColor,
                }}
              />
            </div>
          ) : undefined
        }
      />
      <MetricCard
        label="Deal Value"
        value={
          deal.amount
            ? deal.amount.toLocaleString('en-US', {
                style: 'currency',
                currency: deal.currency || 'USD',
                maximumFractionDigits: 0,
              })
            : '\u2014'
        }
        icon={DollarSign}
        accent={stageColor}
      />
      {taskTotal > 0 && (
        <MetricCard
          label="Task Progress"
          value={`${taskDone} / ${taskTotal}`}
          icon={CheckSquare}
          accent={stageColor}
          sub={
            <div className="mt-1.5 h-1 bg-muted rounded-full overflow-hidden">
              <div
                className={cn(
                  'h-full rounded-full',
                  taskPct === 100 ? 'bg-green-500' : 'bg-blue-500'
                )}
                style={{ width: `${taskPct}%` }}
              />
            </div>
          }
        />
      )}
      {(deal.deliverable_count ?? 0) > 0 && (
        <MetricCard
          label="Deliverables"
          value={String(deal.deliverable_count)}
          icon={FileText}
          accent={stageColor}
        />
      )}
    </CardGrid>
  );
}
