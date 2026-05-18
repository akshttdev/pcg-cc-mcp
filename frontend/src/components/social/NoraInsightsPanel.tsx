import { useQuery } from '@tanstack/react-query';
import { ChevronRight, RefreshCw, Sparkles } from 'lucide-react';
import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { socialApi } from '@/lib/api';
import { socialKeys } from '@/lib/query-keys';

export function NoraInsightsPanel({
  projectId,
  orgId,
  days = 30,
}: {
  projectId?: string;
  orgId?: string;
  days?: number;
}) {
  const [enabled, setEnabled] = useState(false);

  const { data, isLoading, refetch, isFetching } = useQuery({
    queryKey: socialKeys.insights(projectId ?? orgId ?? '', days),
    queryFn: () => socialApi.getInsights({ projectId, orgId, days }),
    enabled: enabled && !!(projectId || orgId),
    staleTime: 10 * 60_000,
    retry: 1,
  });

  return (
    <Card className="border-indigo-200/60 dark:border-indigo-800/40">
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Sparkles className="h-4 w-4 text-indigo-500" />
            Nora Insights
          </CardTitle>
          {enabled && data && (
            <Button
              variant="ghost"
              size="sm"
              className="h-6 text-xs gap-1"
              onClick={() => refetch()}
              disabled={isFetching}
            >
              <RefreshCw
                className={`h-3 w-3 ${isFetching ? 'animate-spin' : ''}`}
              />
              Refresh
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {!enabled ? (
          <div className="text-center py-4">
            <p className="text-xs text-muted-foreground mb-3">
              Get AI-powered analysis of your social performance
            </p>
            <Button
              size="sm"
              onClick={() => setEnabled(true)}
              className="gap-1.5"
            >
              <Sparkles className="h-3.5 w-3.5" />
              Generate Insights
            </Button>
          </div>
        ) : isLoading || isFetching ? (
          <div className="space-y-2 py-2">
            {[...Array(3)].map((_, i) => (
              <div
                key={i}
                className="h-3 rounded bg-muted animate-pulse"
                style={{ width: `${85 - i * 10}%` }}
              />
            ))}
          </div>
        ) : data ? (
          <div className="space-y-4">
            <p className="text-xs leading-relaxed text-muted-foreground">
              {data.summary}
            </p>
            {data.top_finding && (
              <div className="rounded-lg bg-indigo-50 dark:bg-indigo-950/30 px-3 py-2 border border-indigo-100 dark:border-indigo-900/50">
                <p className="text-xs font-medium text-indigo-700 dark:text-indigo-300">
                  {data.top_finding}
                </p>
              </div>
            )}
            {data.recommendations.length > 0 && (
              <div className="space-y-1.5">
                <p className="text-xs font-medium text-muted-foreground uppercase tracking-wide">
                  Actions
                </p>
                {data.recommendations.map((rec, i) => (
                  <div key={i} className="flex items-start gap-1.5 text-xs">
                    <ChevronRight className="h-3 w-3 text-indigo-500 shrink-0 mt-0.5" />
                    <span>{rec}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground text-center py-4">
            Failed to generate insights. Try again.
          </p>
        )}
      </CardContent>
    </Card>
  );
}
