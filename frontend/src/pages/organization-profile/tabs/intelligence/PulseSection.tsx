import { useMemo } from 'react';
import { useQueries } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  AlertTriangle,
  Radio,
} from 'lucide-react';
import { pulseApi } from '@/lib/api';
import { formatDate } from '../../helpers';

// ── Pulse View (deep view for pulse) ─────────────────────────────────────────

export function PulseView({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const alertQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-alerts-org', entry.id],
      queryFn: () => pulseApi.getAlerts(entry.id, 20),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const contentQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-content-org', entry.id],
      queryFn: () => pulseApi.getLatestContent(entry.id, 10),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const aggregated = useMemo(() => {
    const allAlerts: any[] = [];
    const allContent: any[] = [];

    alertQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      (q.data as any[]).forEach((a: any) => allAlerts.push({ ...a, _projectName: entry.name }));
    });

    contentQueries.forEach((q, i) => {
      if (!q.data?.items) return;
      const entry = projectEntries[i];
      q.data.items.forEach((c: any) => allContent.push({ ...c, _projectName: entry.name }));
    });

    allAlerts.sort((a, b) => new Date(b.triggered_at || b.created_at).getTime() - new Date(a.triggered_at || a.created_at).getTime());
    allContent.sort((a, b) => new Date(b.collected_at || b.created_at).getTime() - new Date(a.collected_at || a.created_at).getTime());

    const unacknowledged = allAlerts.filter(a => !a.acknowledged_at).length;
    return { alerts: allAlerts.slice(0, 20), content: allContent.slice(0, 20), unacknowledged };
  }, [alertQueries, contentQueries, projectEntries]);

  const fmtDate = (iso: string) => {
    try { return new Date(iso).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }); }
    catch { return '\u2014'; }
  };

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Unacknowledged Alerts</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.unacknowledged > 0 ? 'text-amber-600' : ''}`}>
              {aggregated.unacknowledged}
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Recent Signals</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{aggregated.content.length}</span>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-amber-500" />
              Recent Alerts
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.alerts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <AlertTriangle className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No alerts</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.alerts.map((alert: any) => (
                    <div
                      key={alert.id}
                      className={`p-3 rounded-lg border border-border/50 ${!alert.acknowledged_at ? 'bg-amber-50/30 dark:bg-amber-950/20' : ''}`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-medium truncate">{alert.rule_name || 'Alert'}</p>
                          {alert.message && (
                            <p className="text-xs text-muted-foreground line-clamp-2 mt-0.5">{alert.message}</p>
                          )}
                          <div className="flex items-center gap-2 mt-1">
                            <Badge variant="outline" className="text-[9px]">{alert._projectName}</Badge>
                            <span className="text-[10px] text-muted-foreground">
                              {fmtDate(alert.triggered_at || alert.created_at)}
                            </span>
                          </div>
                        </div>
                        {!alert.acknowledged_at && (
                          <span className="h-2 w-2 rounded-full bg-amber-500 shrink-0 mt-1" />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Radio className="h-4 w-4 text-blue-500" />
              Latest Signals
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.content.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Radio className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No signals collected yet</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.content.map((item: any) => (
                    <div key={item.id} className="p-3 rounded-lg border border-border/50">
                      <p className="text-sm font-medium line-clamp-2">{item.title || item.content_preview || 'Signal'}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <Badge variant="outline" className="text-[9px]">{item._projectName}</Badge>
                        {item.source_name && (
                          <span className="text-[10px] text-muted-foreground">{item.source_name}</span>
                        )}
                        <span className="text-[10px] text-muted-foreground ml-auto">
                          {fmtDate(item.collected_at || item.created_at)}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

// ── Pulse Section (standalone) ───────────────────────────────────────────────

export function PulseSection({ projectEntries }: { projectEntries: { id: string; name: string }[] }) {
  const alertQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-alerts-org', entry.id],
      queryFn: () => pulseApi.getAlerts(entry.id, 20),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const contentQueries = useQueries({
    queries: projectEntries.map((entry) => ({
      queryKey: ['pulse-content-org', entry.id],
      queryFn: () => pulseApi.getLatestContent(entry.id, 10),
      staleTime: 60_000,
      enabled: projectEntries.length > 0,
    })),
  });

  const aggregated = useMemo(() => {
    const allAlerts: any[] = [];
    const allContent: any[] = [];

    alertQueries.forEach((q, i) => {
      if (!q.data) return;
      const entry = projectEntries[i];
      q.data.forEach((a: any) => allAlerts.push({ ...a, _projectName: entry.name }));
    });

    contentQueries.forEach((q, i) => {
      if (!q.data?.items) return;
      const entry = projectEntries[i];
      q.data.items.forEach((c: any) => allContent.push({ ...c, _projectName: entry.name }));
    });

    allAlerts.sort((a, b) => new Date(b.triggered_at || b.created_at).getTime() - new Date(a.triggered_at || a.created_at).getTime());
    allContent.sort((a, b) => new Date(b.collected_at || b.created_at).getTime() - new Date(a.collected_at || a.created_at).getTime());

    const unacknowledged = allAlerts.filter(a => !a.acknowledged_at).length;
    return { alerts: allAlerts.slice(0, 20), content: allContent.slice(0, 20), unacknowledged };
  }, [alertQueries, contentQueries, projectEntries]);

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Unacknowledged Alerts</CardTitle>
          </CardHeader>
          <CardContent>
            <span className={`text-2xl font-bold ${aggregated.unacknowledged > 0 ? 'text-amber-600' : ''}`}>
              {aggregated.unacknowledged}
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </span>
          </CardContent>
        </Card>
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader className="pb-2">
            <CardTitle className="text-sm font-medium text-muted-foreground">Recent Signals</CardTitle>
          </CardHeader>
          <CardContent>
            <span className="text-2xl font-bold">{aggregated.content.length}</span>
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <AlertTriangle className="h-4 w-4 text-amber-500" />
              Recent Alerts
              {aggregated.unacknowledged > 0 && (
                <Badge variant="default" className="text-[10px] ml-1">{aggregated.unacknowledged} new</Badge>
              )}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.alerts.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <AlertTriangle className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No alerts</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.alerts.map((alert: any) => (
                    <div
                      key={alert.id}
                      className={`p-3 rounded-lg border border-border/50 ${!alert.acknowledged_at ? 'bg-amber-50/30 dark:bg-amber-950/20' : ''}`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="min-w-0">
                          <p className="text-sm font-medium truncate">{alert.rule_name || 'Alert'}</p>
                          {alert.message && (
                            <p className="text-xs text-muted-foreground line-clamp-2 mt-0.5">{alert.message}</p>
                          )}
                          <div className="flex items-center gap-2 mt-1">
                            <Badge variant="outline" className="text-[9px]">{alert._projectName}</Badge>
                            <span className="text-[10px] text-muted-foreground">
                              {formatDate(alert.triggered_at || alert.created_at)}
                            </span>
                          </div>
                        </div>
                        {!alert.acknowledged_at && (
                          <span className="h-2 w-2 rounded-full bg-amber-500 shrink-0 mt-1" />
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>

        <Card className="bg-card/80 backdrop-blur-sm border-border/50">
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Radio className="h-4 w-4 text-blue-500" />
              Latest Signals
            </CardTitle>
          </CardHeader>
          <CardContent>
            {aggregated.content.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                <Radio className="h-8 w-8 mx-auto mb-2 opacity-40" />
                <p>No signals collected yet</p>
              </div>
            ) : (
              <ScrollArea className="h-[300px]">
                <div className="space-y-2 pr-3">
                  {aggregated.content.map((item: any) => (
                    <div key={item.id} className="p-3 rounded-lg border border-border/50">
                      <p className="text-sm font-medium line-clamp-2">{item.title || item.content_preview || 'Signal'}</p>
                      <div className="flex items-center gap-2 mt-1">
                        <Badge variant="outline" className="text-[9px]">{item._projectName}</Badge>
                        {item.source_name && (
                          <span className="text-[10px] text-muted-foreground">{item.source_name}</span>
                        )}
                        <span className="text-[10px] text-muted-foreground ml-auto">
                          {formatDate(item.collected_at || item.created_at)}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </ScrollArea>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
