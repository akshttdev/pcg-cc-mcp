import { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import {
  Coins,
  TrendingUp,
  ArrowUpRight,
  ArrowDownLeft,
  RefreshCw,
  Clock,
  Zap,
  Target,
} from 'lucide-react';
import { MobileLayout } from '@/components/mobile';
import { useMobile } from '@/hooks/useMobile';

interface VibeStats {
  balance: number;
  earned_today: number;
  earned_this_week: number;
  total_earned: number;
  total_spent: number;
  pending_rewards: number;
}

export default function VibePage() {
  const { isMobile } = useMobile();
  const [stats, setStats] = useState<VibeStats>({
    balance: 0,
    earned_today: 0,
    earned_this_week: 0,
    total_earned: 0,
    total_spent: 0,
    pending_rewards: 0,
  });
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Fetch from mesh stats API
    const fetchStats = async () => {
      try {
        const response = await fetch('/api/mesh/stats');
        if (response.ok) {
          const result = await response.json();
          if (result.success && result.data) {
            const completedToday = result.data.completed_tasks_today || result.data.completedTasksToday || 0;
            const balance = result.data.vibe_balance || result.data.vibeBalance || 0;
            setStats({
              balance,
              earned_today: completedToday * 10,
              earned_this_week: balance,
              total_earned: balance,
              total_spent: 0,
              pending_rewards: (result.data.active_tasks || result.data.activeTasks || 0) * 10,
            });
          }
        }
      } catch (err) {
        console.error('Failed to fetch vibe stats:', err);
      } finally {
        setLoading(false);
      }
    };

    fetchStats();
    const interval = setInterval(fetchStats, 5000);
    return () => clearInterval(interval);
  }, []);

  const content = (
    <div className="space-y-4">
      {/* Balance Card */}
      <Card className="bg-gradient-to-br from-yellow-500/10 to-orange-500/10 border-yellow-500/20">
        <CardContent className="pt-6">
          <div className="text-center">
            <Coins className="h-12 w-12 mx-auto mb-2 text-yellow-500" />
            <p className="text-sm text-muted-foreground mb-1">Total Balance</p>
            <p className="text-4xl font-bold text-yellow-500">
              {stats.balance.toFixed(2)}
            </p>
            <p className="text-sm text-muted-foreground mt-1">VIBE</p>
          </div>

          <div className="grid grid-cols-2 gap-4 mt-6">
            <div className="text-center p-3 bg-background/50 rounded-lg">
              <TrendingUp className="h-4 w-4 mx-auto mb-1 text-green-500" />
              <p className="text-xs text-muted-foreground">Today</p>
              <p className="text-lg font-semibold">+{stats.earned_today.toFixed(0)}</p>
            </div>
            <div className="text-center p-3 bg-background/50 rounded-lg">
              <Clock className="h-4 w-4 mx-auto mb-1 text-blue-500" />
              <p className="text-xs text-muted-foreground">Pending</p>
              <p className="text-lg font-semibold">{stats.pending_rewards.toFixed(0)}</p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Earning Stats */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Zap className="h-4 w-4" />
            Earning Summary
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex justify-between items-center">
            <span className="text-sm text-muted-foreground">This Week</span>
            <span className="font-medium text-green-500">+{stats.earned_this_week.toFixed(2)} VIBE</span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-sm text-muted-foreground">Total Earned</span>
            <span className="font-medium">{stats.total_earned.toFixed(2)} VIBE</span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-sm text-muted-foreground">Total Spent</span>
            <span className="font-medium text-orange-500">-{stats.total_spent.toFixed(2)} VIBE</span>
          </div>
        </CardContent>
      </Card>

      {/* Earning Goals */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Target className="h-4 w-4" />
            Daily Goal
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">Progress</span>
              <span className="font-medium">{stats.earned_today.toFixed(0)} / 100 VIBE</span>
            </div>
            <Progress value={Math.min(stats.earned_today, 100)} className="h-2" />
            <p className="text-xs text-muted-foreground">
              Complete tasks to earn more VIBE
            </p>
          </div>
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
