import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Coins,
  TrendingUp,
  ArrowUpRight,
  ArrowDownLeft,
  RefreshCw,
  Clock,
  Zap,
  Target,
  Bot,
  FileText,
  CheckCircle,
} from 'lucide-react';
import { MobileLayout } from '@/components/mobile';
import { useMobile } from '@/hooks/useMobile';
import { useProjectList } from '@/hooks/api/useProjectList';
import { format } from 'date-fns';

interface VibeStats {
  balance: number;
  budget_limit: number;
  earned_today: number;
  earned_this_week: number;
  total_earned: number;
  total_spent: number;
  pending_rewards: number;
}

interface VibeTransaction {
  id: string;
  source_type: string;
  amount_vibe: number;
  input_tokens: number | null;
  output_tokens: number | null;
  model: string | null;
  provider: string | null;
  calculated_cost_cents: number | null;
  aptos_tx_status: string | null;
  task_id: string | null;
  description: string | null;
  metadata: string | null;
  created_at: string;
}

export default function VibePage() {
  const { isMobile } = useMobile();
  const { data: projects = [] } = useProjectList();
  const projectId = projects[0]?.id;
  const [stats, setStats] = useState<VibeStats>({
    balance: 0,
    budget_limit: 0,
    earned_today: 0,
    earned_this_week: 0,
    total_earned: 0,
    total_spent: 0,
    pending_rewards: 0,
  });
  const [transactions, setTransactions] = useState<VibeTransaction[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      try {
        // Fetch mesh stats for network earnings
        const meshResp = await fetch('/api/mesh/stats');
        let meshBalance = 0;
        let pendingRewards = 0;
        if (meshResp.ok) {
          const meshResult = await meshResp.json();
          if (meshResult.success && meshResult.data) {
            meshBalance = meshResult.data.vibe_balance || meshResult.data.vibeBalance || 0;
            pendingRewards = (meshResult.data.active_tasks || meshResult.data.activeTasks || 0) * 10;
          }
        }

        // Fetch project VIBE balance & transactions if we have a project
        let totalSpent = 0;
        let budgetLimit = 0;
        if (projectId) {
          try {
            const balResp = await fetch(`/api/projects/${projectId}/vibe/balance`);
            if (balResp.ok) {
              const balResult = await balResp.json();
              if (balResult.success && balResult.data) {
                totalSpent = balResult.data.total_spent || 0;
                budgetLimit = balResult.data.total_deposited || 0;
              }
            }
          } catch {}

          try {
            const txResp = await fetch(`/api/projects/${projectId}/vibe/transactions?limit=50`);
            if (txResp.ok) {
              const txResult = await txResp.json();
              if (txResult.success && txResult.data) {
                setTransactions(txResult.data);
              }
            }
          } catch {}
        }

        setStats({
          balance: meshBalance > 0 ? meshBalance : (budgetLimit - totalSpent),
          budget_limit: budgetLimit,
          earned_today: meshBalance,
          earned_this_week: meshBalance,
          total_earned: meshBalance + budgetLimit,
          total_spent: totalSpent,
          pending_rewards: pendingRewards,
        });
      } catch (err) {
        console.error('Failed to fetch vibe stats:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    const interval = setInterval(fetchData, 10000);
    return () => clearInterval(interval);
  }, [projectId]);

  const content = (
    <div className="space-y-4">
      {/* Balance Card */}
      <Card className="bg-gradient-to-br from-yellow-500/10 to-orange-500/10 border-yellow-500/20">
        <CardContent className="pt-6">
          <div className="text-center">
            <Coins className="h-12 w-12 mx-auto mb-2 text-yellow-500" />
            <p className="text-sm text-muted-foreground mb-1">Available Balance</p>
            <p className="text-4xl font-bold text-yellow-500">
              {(stats.budget_limit > 0 ? stats.budget_limit - stats.total_spent : stats.balance).toFixed(0)}
            </p>
            <p className="text-sm text-muted-foreground mt-1">VIBE</p>
          </div>

          <div className="grid grid-cols-3 gap-3 mt-6">
            <div className="text-center p-3 bg-background/50 rounded-lg">
              <TrendingUp className="h-4 w-4 mx-auto mb-1 text-green-500" />
              <p className="text-xs text-muted-foreground">Budget</p>
              <p className="text-lg font-semibold">{stats.budget_limit}</p>
            </div>
            <div className="text-center p-3 bg-background/50 rounded-lg">
              <Zap className="h-4 w-4 mx-auto mb-1 text-orange-500" />
              <p className="text-xs text-muted-foreground">Spent</p>
              <p className="text-lg font-semibold">{stats.total_spent}</p>
            </div>
            <div className="text-center p-3 bg-background/50 rounded-lg">
              <Clock className="h-4 w-4 mx-auto mb-1 text-blue-500" />
              <p className="text-xs text-muted-foreground">Pending</p>
              <p className="text-lg font-semibold">{stats.pending_rewards}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Budget Usage */}
      {stats.budget_limit > 0 && (
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base flex items-center gap-2">
              <Target className="h-4 w-4" />
              Budget Usage
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-muted-foreground">
                  {stats.total_spent} / {stats.budget_limit} VIBE
                </span>
                <span className="font-medium">
                  {((stats.total_spent / stats.budget_limit) * 100).toFixed(1)}%
                </span>
              </div>
              <Progress
                value={(stats.total_spent / stats.budget_limit) * 100}
                className="h-2"
              />
            </div>
          </CardContent>
        </Card>
      )}

      {/* Transaction History */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <FileText className="h-4 w-4" />
            Transaction History ({transactions.length})
          </CardTitle>
        </CardHeader>
        <CardContent>
          {transactions.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-4">
              No transactions yet
            </p>
          ) : (
            <ScrollArea className="h-[400px]">
              <div className="space-y-2">
                {transactions.map((tx) => {
                  const meta = tx.metadata ? JSON.parse(tx.metadata) : {};
                  return (
                    <div
                      key={tx.id}
                      className="flex items-start gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
                    >
                      <div className="mt-0.5">
                        <Bot className="h-5 w-5 text-purple-500" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center justify-between gap-2">
                          <p className="text-sm font-medium truncate">
                            {tx.description || 'VIBE Transaction'}
                          </p>
                          <Badge
                            variant="outline"
                            className="text-xs shrink-0 text-orange-500 border-orange-500/30"
                          >
                            -{tx.amount_vibe} VIBE
                          </Badge>
                        </div>
                        <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
                          {tx.model && (
                            <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                              {tx.model}
                            </Badge>
                          )}
                          {meta.phase && (
                            <Badge variant="secondary" className="text-[10px] px-1.5 py-0">
                              {meta.phase}
                            </Badge>
                          )}
                          {tx.input_tokens && tx.output_tokens && (
                            <span>
                              {((tx.input_tokens + tx.output_tokens) / 1000).toFixed(1)}k tokens
                            </span>
                          )}
                          {tx.calculated_cost_cents != null && (
                            <span>${(tx.calculated_cost_cents / 100).toFixed(2)}</span>
                          )}
                        </div>
                        <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
                          <span>{format(new Date(tx.created_at), 'MMM d, h:mm a')}</span>
                          {tx.aptos_tx_status === 'confirmed' && (
                            <CheckCircle className="h-3 w-3 text-green-500" />
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </ScrollArea>
          )}
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <div className="grid grid-cols-2 gap-3">
        <Button variant="outline" className="h-auto py-4 flex-col gap-2">
          <ArrowUpRight className="h-5 w-5 text-green-500" />
          <span>Send</span>
        </Button>
        <Button variant="outline" className="h-auto py-4 flex-col gap-2">
          <ArrowDownLeft className="h-5 w-5 text-blue-500" />
          <span>Receive</span>
        </Button>
      </div>
    </div>
  );

  if (loading) {
    return (
      <MobileLayout title="Vibe">
        <div className="flex items-center justify-center h-64">
          <RefreshCw className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      </MobileLayout>
    );
  }

  if (isMobile) {
    return <MobileLayout title="Vibe">{content}</MobileLayout>;
  }

  return (
    <div className="container mx-auto py-6 max-w-2xl">
      <h1 className="text-2xl font-bold mb-6 flex items-center gap-2">
        <Coins className="h-6 w-6 text-yellow-500" />
        Vibe Treasury
      </h1>
      {content}
    </div>
  );
}
