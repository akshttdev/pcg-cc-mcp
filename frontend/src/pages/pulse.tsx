import { useState } from 'react';
import { useParams } from 'react-router-dom';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { pulseApi } from '@/lib/api';
import { MobileLayout } from '@/components/mobile';
import { useMobile } from '@/hooks/useMobile';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { PulseWidget } from '@/components/PulseWidget';
import {
  Activity,
  AlertTriangle,
  ExternalLink,
  FileText,
  Play,
  Radio,
  RefreshCw,
  Rss,
  Settings2,
} from 'lucide-react';

type Tab = 'overview' | 'content' | 'sources' | 'alerts' | 'config';

export default function PulsePage() {
  const { isMobile } = useMobile();
  const { projectId } = useParams<{ projectId?: string }>();
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const queryClient = useQueryClient();

  // If no project ID, show the legacy widget view
  if (!projectId) {
    const content = (
      <>
        {!isMobile && (
          <div className="mb-6">
            <h1 className="text-2xl font-bold">Pulse Engine</h1>
            <p className="text-muted-foreground">
              Monitor news feeds, content collection, and alert pipelines across all projects
            </p>
          </div>
        )}
        <PulseWidget className="w-full" />
      </>
    );

    if (isMobile) {
      return <MobileLayout title="Pulse Engine">{content}</MobileLayout>;
    }
    return <div className="container mx-auto p-6 max-w-6xl">{content}</div>;
  }

  // Project-scoped dashboard
  const { data: stats } = useQuery({
    queryKey: ['pulse', 'stats', projectId],
    queryFn: () => pulseApi.getStats(projectId),
    refetchInterval: 30000,
  });

  const { data: contentData, isLoading: contentLoading } = useQuery({
    queryKey: ['pulse', 'content', projectId],
    queryFn: () => pulseApi.getLatestContent(projectId, 50),
    enabled: activeTab === 'content' || activeTab === 'overview',
  });

  const { data: sources } = useQuery({
    queryKey: ['pulse', 'sources', projectId],
    queryFn: () => pulseApi.getSources(projectId),
    enabled: activeTab === 'sources' || activeTab === 'overview',
  });

  const { data: alerts } = useQuery({
    queryKey: ['pulse', 'alerts', projectId],
    queryFn: () => pulseApi.getAlerts(projectId),
    enabled: activeTab === 'alerts' || activeTab === 'overview',
  });

  const { data: alertRules } = useQuery({
    queryKey: ['pulse', 'alert-rules', projectId],
    queryFn: () => pulseApi.getAlertRules(projectId),
    enabled: activeTab === 'alerts',
  });

  const { data: trackingConfig } = useQuery({
    queryKey: ['pulse', 'tracking', projectId],
    queryFn: () => pulseApi.getTrackingConfig(projectId),
    enabled: activeTab === 'config',
  });

  const collectMutation = useMutation({
    mutationFn: () => pulseApi.triggerCollection(projectId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pulse'] });
    },
  });

  const actionMutation = useMutation({
    mutationFn: ({ contentId, action }: { contentId: string; action: { action: string } }) =>
      pulseApi.contentAction(projectId, contentId, action),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['pulse', 'content'] });
      queryClient.invalidateQueries({ queryKey: ['pulse', 'stats'] });
    },
  });

  const tabs: { id: Tab; label: string; icon: React.ReactNode }[] = [
    { id: 'overview', label: 'Overview', icon: <Activity className="w-4 h-4" /> },
    { id: 'content', label: 'Content Feed', icon: <FileText className="w-4 h-4" /> },
    { id: 'sources', label: 'Sources', icon: <Rss className="w-4 h-4" /> },
    { id: 'alerts', label: 'Alerts', icon: <AlertTriangle className="w-4 h-4" /> },
    { id: 'config', label: 'Config', icon: <Settings2 className="w-4 h-4" /> },
  ];

  function timeAgo(dateStr: string): string {
    const date = new Date(dateStr);
    const now = new Date();
    const diff = Math.floor((now.getTime() - date.getTime()) / 1000);
    if (diff < 60) return `${diff}s ago`;
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  const dashboard = (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold">Pulse Engine</h1>
          <p className="text-muted-foreground">Content monitoring dashboard</p>
        </div>
        <Button
          onClick={() => collectMutation.mutate()}
          disabled={collectMutation.isPending}
          variant="outline"
          size="sm"
        >
          {collectMutation.isPending ? (
            <RefreshCw className="w-4 h-4 mr-2 animate-spin" />
          ) : (
            <Play className="w-4 h-4 mr-2" />
          )}
          Collect Now
        </Button>
      </div>

      {/* Tabs */}
      <div className="flex gap-1 border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeTab === tab.id
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            }`}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      {activeTab === 'overview' && (
        <div className="space-y-6">
          {/* Stats Cards */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4 animate-stagger">
            <Card>
              <CardContent className="p-4">
                <div className="text-2xl font-bold">{stats?.total_content ?? 0}</div>
                <div className="text-sm text-muted-foreground">Content Items</div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-4">
                <div className="text-2xl font-bold">{stats?.active_sources ?? 0}</div>
                <div className="text-sm text-muted-foreground">Active Sources</div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-4">
                <div className="text-2xl font-bold">{stats?.total_sources ?? 0}</div>
                <div className="text-sm text-muted-foreground">Total Sources</div>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-4">
                <div className="text-2xl font-bold text-orange-500">{stats?.unacknowledged_alerts ?? 0}</div>
                <div className="text-sm text-muted-foreground">Unread Alerts</div>
              </CardContent>
            </Card>
          </div>

          {/* Recent Content */}
          <Card>
            <CardHeader>
              <CardTitle className="text-lg">Latest Content</CardTitle>
            </CardHeader>
            <CardContent>
              {contentData?.items?.length ? (
                <div className="space-y-3">
                  {contentData.items.slice(0, 5).map((item: any) => (
                    <div key={item.id} className="flex items-start gap-3 p-3 rounded-lg border">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2">
                          <a href={item.url} target="_blank" rel="noopener noreferrer" className="font-medium text-sm hover:underline truncate">
                            {item.title}
                          </a>
                          <ExternalLink className="w-3 h-3 text-muted-foreground shrink-0" />
                        </div>
                        <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
                          <Badge variant="outline" className="text-xs">{item.source_id}</Badge>
                          <span>{timeAgo(item.collected_at)}</span>
                          {item.relevance_score != null && (
                            <Badge variant={item.relevance_score > 0.7 ? 'default' : 'secondary'} className="text-xs">
                              {Math.round(item.relevance_score * 100)}%
                            </Badge>
                          )}
                        </div>
                      </div>
                      <Badge variant="outline" className="text-xs shrink-0">{item.pcg_status}</Badge>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">No content collected yet</p>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {activeTab === 'content' && (
        <div className="space-y-3">
          {contentLoading ? (
            <p className="text-sm text-muted-foreground">Loading content...</p>
          ) : contentData?.items?.length ? (
            contentData.items.map((item: any) => (
              <div key={item.id} className="flex items-start gap-3 p-4 rounded-lg border">
                <div className="flex-1 min-w-0">
                  <a href={item.url} target="_blank" rel="noopener noreferrer" className="font-medium text-sm hover:underline">
                    {item.title}
                  </a>
                  {item.summary && <p className="text-xs text-muted-foreground mt-1 line-clamp-2">{item.summary}</p>}
                  <div className="flex items-center gap-2 mt-2 text-xs text-muted-foreground">
                    <Badge variant="outline">{item.source_type}</Badge>
                    <span>{item.source_id}</span>
                    <span>{timeAgo(item.collected_at)}</span>
                    {item.author && <span>by {item.author}</span>}
                  </div>
                </div>
                <div className="flex flex-col gap-1 shrink-0">
                  <Badge variant="outline">{item.pcg_status}</Badge>
                  {item.pcg_status === 'new' && (
                    <div className="flex gap-1">
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 text-xs"
                        onClick={() => actionMutation.mutate({ contentId: item.id, action: { action: 'review' } })}
                      >
                        Review
                      </Button>
                      <Button
                        variant="ghost"
                        size="sm"
                        className="h-6 text-xs"
                        onClick={() => actionMutation.mutate({ contentId: item.id, action: { action: 'dismiss' } })}
                      >
                        Dismiss
                      </Button>
                    </div>
                  )}
                </div>
              </div>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">No content found</p>
          )}
        </div>
      )}

      {activeTab === 'sources' && (
        <div className="space-y-3">
          {sources?.length ? (
            sources.map((source: any) => (
              <Card key={source.id}>
                <CardContent className="p-4 flex items-center gap-4">
                  <Radio className={`w-5 h-5 ${source.status === 'active' ? 'text-green-500' : 'text-red-500'}`} />
                  <div className="flex-1">
                    <div className="font-medium text-sm">{source.name || source.source_id}</div>
                    <div className="text-xs text-muted-foreground">{source.url}</div>
                  </div>
                  <Badge variant={source.enabled ? 'default' : 'secondary'}>{source.source_type}</Badge>
                  <Badge variant={source.status === 'active' ? 'default' : 'destructive'}>{source.status}</Badge>
                </CardContent>
              </Card>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">No sources configured</p>
          )}
        </div>
      )}

      {activeTab === 'alerts' && (
        <div className="space-y-6">
          {alertRules?.length ? (
            <Card>
              <CardHeader><CardTitle className="text-lg">Alert Rules</CardTitle></CardHeader>
              <CardContent>
                <div className="space-y-2">
                  {alertRules.map((rule: any) => (
                    <div key={rule.id} className="flex items-center gap-3 p-3 rounded-lg border">
                      <div className="flex-1">
                        <div className="font-medium text-sm">{rule.name}</div>
                        <div className="text-xs text-muted-foreground">Priority: {rule.priority} | Triggered: {rule.trigger_count}x</div>
                      </div>
                      <Badge variant={rule.enabled ? 'default' : 'secondary'}>{rule.enabled ? 'Active' : 'Disabled'}</Badge>
                    </div>
                  ))}
                </div>
              </CardContent>
            </Card>
          ) : null}

          <Card>
            <CardHeader><CardTitle className="text-lg">Recent Alerts</CardTitle></CardHeader>
            <CardContent>
              {alerts?.length ? (
                <div className="space-y-2">
                  {alerts.map((alert: any) => (
                    <div key={alert.id} className="flex items-center gap-3 p-3 rounded-lg border">
                      <AlertTriangle className={`w-4 h-4 ${alert.priority === 'high' || alert.priority === 'critical' ? 'text-red-500' : 'text-yellow-500'}`} />
                      <div className="flex-1">
                        <div className="text-sm">Rule: {alert.rule_id}</div>
                        <div className="text-xs text-muted-foreground">{timeAgo(alert.created_at)}</div>
                      </div>
                      <Badge variant={alert.acknowledged ? 'secondary' : 'destructive'}>
                        {alert.acknowledged ? 'Acknowledged' : 'New'}
                      </Badge>
                    </div>
                  ))}
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">No alerts</p>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {activeTab === 'config' && (
        <Card>
          <CardHeader><CardTitle className="text-lg">Tracking Configuration</CardTitle></CardHeader>
          <CardContent>
            {trackingConfig ? (
              <div className="space-y-4">
                <div>
                  <div className="font-medium text-sm mb-2">Keywords</div>
                  <div className="flex flex-wrap gap-1">
                    {(() => {
                      try {
                        const kw = typeof trackingConfig.keywords === 'string' ? JSON.parse(trackingConfig.keywords) : trackingConfig.keywords;
                        return Array.isArray(kw) ? kw.map((k: string) => (
                          <Badge key={k} variant="outline">{k}</Badge>
                        )) : <span className="text-xs text-muted-foreground">None</span>;
                      } catch {
                        return <span className="text-xs text-muted-foreground">None</span>;
                      }
                    })()}
                  </div>
                </div>
                <div>
                  <div className="font-medium text-sm mb-2">LLM Processing</div>
                  <Badge variant={trackingConfig.llm_enabled ? 'default' : 'secondary'}>
                    {trackingConfig.llm_enabled ? `Enabled (${trackingConfig.llm_model || 'llama3.2'})` : 'Disabled'}
                  </Badge>
                </div>
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">No tracking configuration set. Content will be collected without keyword filtering.</p>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );

  if (isMobile) {
    return <MobileLayout title="Pulse Engine">{dashboard}</MobileLayout>;
  }

  return <div className="container mx-auto p-6 max-w-6xl">{dashboard}</div>;
}
