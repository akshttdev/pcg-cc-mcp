import { RefreshCw, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { EmptyState } from '@/components/ui/empty-state';
import { type CompanyRecord } from '@/lib/api';
import { IntelBadge } from '../components/helpers';

export function IntelligenceTab({
  company,
  intel,
  onRun,
  isPolling,
}: {
  company: CompanyRecord;
  intel: { status: string; summary?: string; confidence: number; agent?: string; last_run_at?: string } | null;
  onRun: () => void;
  isPolling: boolean;
}) {
  const rawData = (() => {
    try { return JSON.parse(company.intelligence_raw ?? '{}'); } catch { return {}; }
  })();

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <IntelBadge status={intel?.status ?? 'idle'} />
          {intel?.last_run_at && (
            <span className="text-xs text-muted-foreground">
              Last run: {new Date(intel.last_run_at).toLocaleDateString()}
            </span>
          )}
          {intel?.agent && (
            <span className="text-xs text-muted-foreground">by {intel.agent}</span>
          )}
        </div>
        <Button
          size="sm"
          variant="outline"
          onClick={onRun}
          disabled={isPolling || intel?.status === 'running' || intel?.status === 'queued'}
        >
          {isPolling ? (
            <RefreshCw className="h-3.5 w-3.5 mr-1 animate-spin" />
          ) : (
            <Sparkles className="h-3.5 w-3.5 mr-1" />
          )}
          Run Research
        </Button>
      </div>

      {(intel?.confidence ?? 0) > 0 && (
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Confidence</span>
            <span>{Math.round((intel?.confidence ?? 0) * 100)}%</span>
          </div>
          <div className="h-1.5 bg-muted rounded-full overflow-hidden">
            <div className="h-full bg-green-500 rounded-full" style={{ width: `${(intel?.confidence ?? 0) * 100}%` }} />
          </div>
        </div>
      )}

      {intel?.summary ? (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium flex items-center gap-1.5">
              <Sparkles className="h-4 w-4 text-yellow-500" />
              AI Summary
            </CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground leading-relaxed">{intel.summary}</p>
          </CardContent>
        </Card>
      ) : (
        <EmptyState
          icon={Sparkles}
          title="No intelligence gathered yet"
          description="Run research to populate"
          className="h-24 border rounded-lg"
        />
      )}

      {/* Raw intel fields (populated by research) */}
      {Object.keys(rawData).length > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium">Research Data</CardTitle>
          </CardHeader>
          <CardContent>
            <pre className="text-xs text-muted-foreground overflow-auto max-h-64 bg-muted rounded p-2">
              {JSON.stringify(rawData, null, 2)}
            </pre>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
