import { useQuery } from '@tanstack/react-query';
import { Clock } from 'lucide-react';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { socialApi } from '@/lib/api';
import { socialKeys } from '@/lib/query-keys';

const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const HOURS = Array.from({ length: 24 }, (_, i) => i);

export function BestTimesHeatmap({
  projectId,
  orgId,
}: {
  projectId?: string;
  orgId?: string;
}) {
  const { data = [], isLoading } = useQuery({
    queryKey: socialKeys.bestTimes(projectId ?? orgId ?? '', orgId),
    queryFn: () => socialApi.getBestTimes({ projectId, orgId }),
    enabled: !!(projectId || orgId),
    staleTime: 10 * 60_000,
  });

  // Build lookup: day → hour → cell
  const grid: Record<
    number,
    Record<number, { post_count: number; avg_engagement: number }>
  > = {};
  data.forEach((cell) => {
    if (!grid[cell.day_of_week]) grid[cell.day_of_week] = {};
    grid[cell.day_of_week][cell.hour_of_day] = cell;
  });

  const maxEng = Math.max(...data.map((c) => c.avg_engagement), 0.001);

  function cellColor(eng: number): string {
    const intensity = Math.min(eng / maxEng, 1);
    if (intensity === 0) return 'bg-muted/30';
    if (intensity < 0.25) return 'bg-indigo-100 dark:bg-indigo-950/40';
    if (intensity < 0.5) return 'bg-indigo-200 dark:bg-indigo-800/60';
    if (intensity < 0.75) return 'bg-indigo-400 dark:bg-indigo-600';
    return 'bg-indigo-600 dark:bg-indigo-400';
  }

  if (isLoading) {
    return (
      <Card>
        <CardContent className="h-48 flex items-center justify-center text-muted-foreground text-sm">
          Loading heatmap…
        </CardContent>
      </Card>
    );
  }

  if (data.length === 0) {
    return (
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Clock className="h-4 w-4 text-indigo-500" />
            Best Times to Post
          </CardTitle>
        </CardHeader>
        <CardContent className="text-center py-8 text-xs text-muted-foreground">
          Publish posts to see when your audience engages most
        </CardContent>
      </Card>
    );
  }

  // Only show hours that have any data ± 2 buffer, default 6am–11pm
  const activeHours = data.map((c) => c.hour_of_day);
  const minHour = Math.max(0, Math.min(...activeHours, 6) - 1);
  const maxHour = Math.min(23, Math.max(...activeHours, 22) + 1);
  const visibleHours = HOURS.filter((h) => h >= minHour && h <= maxHour);

  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-sm font-medium flex items-center gap-2">
          <Clock className="h-4 w-4 text-indigo-500" />
          Best Times to Post
        </CardTitle>
      </CardHeader>
      <CardContent>
        <div className="overflow-x-auto">
          <table className="w-full text-xs border-separate border-spacing-0.5">
            <thead>
              <tr>
                <th className="w-8 text-muted-foreground font-normal text-right pr-1.5" />
                {DAYS.map((d) => (
                  <th
                    key={d}
                    className="text-center text-muted-foreground font-normal pb-1 w-10"
                  >
                    {d}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {visibleHours.map((hour) => {
                const label =
                  hour === 0
                    ? '12am'
                    : hour < 12
                      ? `${hour}am`
                      : hour === 12
                        ? '12pm'
                        : `${hour - 12}pm`;
                return (
                  <tr key={hour}>
                    <td className="text-right pr-1.5 text-muted-foreground whitespace-nowrap">
                      {label}
                    </td>
                    {DAYS.map((_, day) => {
                      const cell = grid[day]?.[hour];
                      const eng = cell?.avg_engagement ?? 0;
                      const posts = cell?.post_count ?? 0;
                      return (
                        <td key={day} className="p-0">
                          <div
                            className={`h-5 w-full rounded-sm ${cellColor(eng)} transition-colors cursor-default`}
                            title={
                              posts > 0
                                ? `${posts} post${posts > 1 ? 's' : ''}, ${(eng * 100).toFixed(1)}% avg engagement`
                                : 'No data'
                            }
                          />
                        </td>
                      );
                    })}
                  </tr>
                );
              })}
            </tbody>
          </table>
          <div className="flex items-center gap-2 mt-3 text-xs text-muted-foreground">
            <span>Low</span>
            <div className="flex gap-0.5">
              {[
                'bg-muted/30',
                'bg-indigo-100',
                'bg-indigo-200',
                'bg-indigo-400',
                'bg-indigo-600',
              ].map((c) => (
                <div key={c} className={`h-3 w-5 rounded-sm ${c}`} />
              ))}
            </div>
            <span>High engagement</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
