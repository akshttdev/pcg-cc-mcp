import { useState, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Cpu,
  RefreshCw,
  Activity,
  Server,
  Layers,
  FolderKanban,
  Bot,
} from 'lucide-react';
import { costsApi } from '@/lib/api';
import { costKeys } from '@/lib/query-keys';
import { useOrganization } from '@/contexts/organization-context';
import { cn } from '@/lib/utils';
import { formatCost, formatTokens } from '@/lib/format';
import { CostSummaryCards } from './CostSummaryCards';
import { CostTrendChart } from './CostTrendChart';
import type { DailyTrendEntry } from './CostTrendChart';

type Tab = 'overview' | 'providers' | 'models' | 'projects' | 'agents';

export function AIUsagePage() {
  const { effectiveOrgId } = useOrganization();
  const [activeTab, setActiveTab] = useState<Tab>('overview');
  const [days, setDays] = useState<number>(7);

  const { data: costSummary, isLoading: summaryLoading, refetch: refetchSummary } = useQuery({
    queryKey: costKeys.orgSummary(effectiveOrgId ?? '', days),
    queryFn: () => costsApi.orgSummary(effectiveOrgId!, days),
    enabled: !!effectiveOrgId,
  });

  const { data: dailyUsage, isLoading: dailyLoading, refetch: refetchDaily } = useQuery({
    queryKey: costKeys.orgDaily(effectiveOrgId ?? '', days),
    queryFn: () => costsApi.orgDaily(effectiveOrgId!, days),
    enabled: !!effectiveOrgId,
  });

  const { data: providerUsage, isLoading: providerLoading, refetch: refetchProvider } = useQuery({
    queryKey: costKeys.orgByProvider(effectiveOrgId ?? '', days),
    queryFn: () => costsApi.orgByProvider(effectiveOrgId!, days),
    enabled: !!effectiveOrgId,
  });

  const { data: modelUsage, isLoading: modelLoading, refetch: refetchModel } = useQuery({
    queryKey: costKeys.orgByModel(effectiveOrgId ?? '', days),
    queryFn: () => costsApi.orgByModel(effectiveOrgId!, days),
    enabled: !!effectiveOrgId,
  });

  const { data: projectUsage, isLoading: projectLoading, refetch: refetchProject } = useQuery({
    queryKey: costKeys.orgByProject(effectiveOrgId ?? '', days),
    queryFn: () => costsApi.orgByProject(effectiveOrgId!, days),
    enabled: !!effectiveOrgId,
  });

  const { data: agentUsage, isLoading: agentLoading, refetch: refetchAgent } = useQuery({
    queryKey: costKeys.orgByAgent(effectiveOrgId ?? '', days),
    queryFn: () => costsApi.orgByAgent(effectiveOrgId!, days),
    enabled: !!effectiveOrgId,
  });

  const handleRefresh = () => {
    refetchSummary();
    refetchDaily();
    refetchProvider();
    refetchModel();
    refetchProject();
    refetchAgent();
  };

  // Aggregate daily usage by date for trend chart
  const dailyTrend = useMemo((): DailyTrendEntry[] => {
    if (!dailyUsage) return [];
    const byDate: Record<string, DailyTrendEntry> = {};
    for (const entry of dailyUsage) {
      if (!byDate[entry.date]) {
        byDate[entry.date] = { date: entry.date, tokens: 0, cost: 0, requests: 0 };
      }
      byDate[entry.date].tokens += entry.input_tokens + entry.output_tokens;
      byDate[entry.date].cost += entry.total_cost_cents || 0;
      byDate[entry.date].requests += entry.transaction_count;
    }
    return Object.values(byDate).sort((a, b) => a.date.localeCompare(b.date));
  }, [dailyUsage]);

  // Calculate period totals from summary
  const periodTotals = useMemo(() => {
    if (!costSummary) return { tokens: 0, input: 0, output: 0, cost: 0, requests: 0 };
    return {
      tokens: costSummary.total_input_tokens + costSummary.total_output_tokens,
      input: costSummary.total_input_tokens,
      output: costSummary.total_output_tokens,
      cost: costSummary.total_cost_cents,
      requests: costSummary.transaction_count,
    };
  }, [costSummary]);

  const maxDailyTokens = useMemo(() => {
    return Math.max(...dailyTrend.map((d) => d.tokens), 1);
  }, [dailyTrend]);

  const tabs: { id: Tab; label: string; icon: React.ReactNode }[] = [
    { id: 'overview', label: 'Overview', icon: <Activity className="w-4 h-4" /> },
    { id: 'providers', label: 'Providers', icon: <Server className="w-4 h-4" /> },
    { id: 'models', label: 'Models', icon: <Layers className="w-4 h-4" /> },
    { id: 'projects', label: 'Projects', icon: <FolderKanban className="w-4 h-4" /> },
    { id: 'agents', label: 'Agents', icon: <Bot className="w-4 h-4" /> },
  ];

  const isLoading = summaryLoading || dailyLoading || providerLoading || modelLoading || projectLoading || agentLoading;

  if (!effectiveOrgId) {
    return (
      <div className="container mx-auto p-6 max-w-6xl">
        <div className="flex items-center gap-2 mb-4">
          <Cpu className="w-6 h-6 text-primary" />
          <h1 className="text-2xl font-bold">AI Usage Dashboard</h1>
        </div>
        <Card>
          <CardContent className="p-8 text-center">
            <p className="text-muted-foreground">Select an organization to view costs.</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6 max-w-6xl space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Cpu className="w-6 h-6 text-primary" />
            AI Usage Dashboard
          </h1>
          <p className="text-muted-foreground">
            Track token usage and costs across all AI providers
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Select value={String(days)} onValueChange={(v) => setDays(Number(v))}>
            <SelectTrigger className="w-32">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="1">Today</SelectItem>
              <SelectItem value="7">Last 7 days</SelectItem>
              <SelectItem value="30">Last 30 days</SelectItem>
              <SelectItem value="90">Last 90 days</SelectItem>
            </SelectContent>
          </Select>
          <Button variant="outline" size="sm" onClick={handleRefresh} disabled={isLoading}>
            <RefreshCw className={cn("w-4 h-4 mr-2", isLoading && "animate-spin")} />
            Refresh
          </Button>
        </div>
      </div>

      {/* Summary Stats */}
      <CostSummaryCards periodTotals={periodTotals} isLoading={summaryLoading} />

      {/* Tabs */}
      <div className="flex gap-1 border-b">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={cn(
              'flex items-center gap-2 px-4 py-2 text-sm font-medium border-b-2 transition-colors',
              activeTab === tab.id
                ? 'border-primary text-primary'
                : 'border-transparent text-muted-foreground hover:text-foreground'
            )}
          >
            {tab.icon}
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab Content */}
      {activeTab === 'overview' && (
        <div className="space-y-6">
          {/* Daily Trend Chart */}
          <CostTrendChart
            dailyTrend={dailyTrend}
            maxDailyTokens={maxDailyTokens}
            isLoading={dailyLoading}
          />

          {/* Top Providers & Models Side by Side */}
          <div className="grid md:grid-cols-2 gap-6">
            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Top Providers</CardTitle>
              </CardHeader>
              <CardContent>
                {providerLoading ? (
                  <Loader message="Loading..." />
                ) : !providerUsage?.length ? (
                  <p className="text-sm text-muted-foreground">No provider data</p>
                ) : (
                  <div className="space-y-3">
                    {providerUsage.slice(0, 5).map((p) => (
                      <div key={p.provider} className="flex items-center justify-between">
                        <div className="flex items-center gap-2">
                          <Server className="w-4 h-4 text-muted-foreground" />
                          <span className="font-medium capitalize">{p.provider}</span>
                        </div>
                        <div className="text-right">
                          <div className="font-medium">{formatTokens(p.input_tokens + p.output_tokens)}</div>
                          <div className="text-xs text-muted-foreground">{formatCost(p.total_cost_cents)}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="text-lg">Top Models</CardTitle>
              </CardHeader>
              <CardContent>
                {modelLoading ? (
                  <Loader message="Loading..." />
                ) : !modelUsage?.length ? (
                  <p className="text-sm text-muted-foreground">No model data</p>
                ) : (
                  <div className="space-y-3">
                    {modelUsage.slice(0, 5).map((m) => (
                      <div key={`${m.provider}-${m.model}`} className="flex items-center justify-between">
                        <div>
                          <div className="font-medium text-sm">{m.model}</div>
                          <div className="text-xs text-muted-foreground capitalize">{m.provider}</div>
                        </div>
                        <div className="text-right">
                          <div className="font-medium">{formatTokens(m.input_tokens + m.output_tokens)}</div>
                          <div className="text-xs text-muted-foreground">{formatCost(m.total_cost_cents)}</div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </div>
      )}

      {activeTab === 'providers' && (
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Usage by Provider</CardTitle>
          </CardHeader>
          <CardContent>
            {providerLoading ? (
              <div className="flex justify-center py-8">
                <Loader message="Loading provider data..." />
              </div>
            ) : !providerUsage?.length ? (
              <p className="text-sm text-muted-foreground text-center py-8">No provider data available</p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Provider</TableHead>
                    <TableHead className="text-right">Input Tokens</TableHead>
                    <TableHead className="text-right">Output Tokens</TableHead>
                    <TableHead className="text-right">Total Tokens</TableHead>
                    <TableHead className="text-right">Cost</TableHead>
                    <TableHead className="text-right">Requests</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {providerUsage.map((p) => (
                    <TableRow key={p.provider}>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Server className="w-4 h-4 text-muted-foreground" />
                          <span className="font-medium capitalize">{p.provider}</span>
                        </div>
                      </TableCell>
                      <TableCell className="text-right">{formatTokens(p.input_tokens)}</TableCell>
                      <TableCell className="text-right">{formatTokens(p.output_tokens)}</TableCell>
                      <TableCell className="text-right font-medium">{formatTokens(p.input_tokens + p.output_tokens)}</TableCell>
                      <TableCell className="text-right">{formatCost(p.total_cost_cents)}</TableCell>
                      <TableCell className="text-right">{p.transaction_count.toLocaleString()}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}

      {activeTab === 'models' && (
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Usage by Model</CardTitle>
          </CardHeader>
          <CardContent>
            {modelLoading ? (
              <div className="flex justify-center py-8">
                <Loader message="Loading model data..." />
              </div>
            ) : !modelUsage?.length ? (
              <p className="text-sm text-muted-foreground text-center py-8">No model data available</p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Model</TableHead>
                    <TableHead>Provider</TableHead>
                    <TableHead className="text-right">Input Tokens</TableHead>
                    <TableHead className="text-right">Output Tokens</TableHead>
                    <TableHead className="text-right">Total Tokens</TableHead>
                    <TableHead className="text-right">Cost</TableHead>
                    <TableHead className="text-right">Requests</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {modelUsage.map((m) => (
                    <TableRow key={`${m.provider}-${m.model}`}>
                      <TableCell className="font-medium">{m.model}</TableCell>
                      <TableCell>
                        <Badge variant="outline" className="capitalize">{m.provider}</Badge>
                      </TableCell>
                      <TableCell className="text-right">{formatTokens(m.input_tokens)}</TableCell>
                      <TableCell className="text-right">{formatTokens(m.output_tokens)}</TableCell>
                      <TableCell className="text-right font-medium">{formatTokens(m.input_tokens + m.output_tokens)}</TableCell>
                      <TableCell className="text-right">{formatCost(m.total_cost_cents)}</TableCell>
                      <TableCell className="text-right">{m.transaction_count.toLocaleString()}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}

      {activeTab === 'projects' && (
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Usage by Project</CardTitle>
          </CardHeader>
          <CardContent>
            {projectLoading ? (
              <div className="flex justify-center py-8">
                <Loader message="Loading project data..." />
              </div>
            ) : !projectUsage?.length ? (
              <p className="text-sm text-muted-foreground text-center py-8">No project data available</p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Project</TableHead>
                    <TableHead className="text-right">Cost</TableHead>
                    <TableHead className="text-right">Requests</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {projectUsage.map((p) => (
                    <TableRow key={p.project_id}>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <FolderKanban className="w-4 h-4 text-muted-foreground" />
                          <span className="font-medium">{p.project_name || 'Unknown Project'}</span>
                        </div>
                      </TableCell>
                      <TableCell className="text-right font-medium">{formatCost(p.total_cost_cents)}</TableCell>
                      <TableCell className="text-right">{p.transaction_count.toLocaleString()}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}

      {activeTab === 'agents' && (
        <Card>
          <CardHeader>
            <CardTitle className="text-lg">Usage by Agent</CardTitle>
          </CardHeader>
          <CardContent>
            {agentLoading ? (
              <div className="flex justify-center py-8">
                <Loader message="Loading agent data..." />
              </div>
            ) : !agentUsage?.length ? (
              <p className="text-sm text-muted-foreground text-center py-8">No agent data available</p>
            ) : (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Agent</TableHead>
                    <TableHead className="text-right">Cost</TableHead>
                    <TableHead className="text-right">Requests</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {agentUsage.map((a) => (
                    <TableRow key={a.agent_id}>
                      <TableCell>
                        <div className="flex items-center gap-2">
                          <Bot className="w-4 h-4 text-muted-foreground" />
                          <span className="font-medium">{a.agent_name || 'Unknown Agent'}</span>
                        </div>
                      </TableCell>
                      <TableCell className="text-right font-medium">{formatCost(a.total_cost_cents)}</TableCell>
                      <TableCell className="text-right">{a.transaction_count.toLocaleString()}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}
