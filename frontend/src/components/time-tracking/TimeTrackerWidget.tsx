import { useState, useEffect } from 'react';
import { Play, Pause, Clock } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { useTimeTrackingStore } from '@/stores/useTimeTrackingStore';
import { formatDuration } from '@/utils/timeUtils';
import { toast } from 'sonner';

interface TimeTrackerWidgetProps {
  taskId: string;
  compact?: boolean;
}

export function TimeTrackerWidget({ taskId, compact = false }: TimeTrackerWidgetProps) {
  const { activeTimer, startTimer, stopTimer, getTaskStats } = useTimeTrackingStore();
  const [elapsed, setElapsed] = useState(0);
  const isActive = activeTimer?.taskId === taskId;

  // Update elapsed time every second when timer is active
  useEffect(() => {
    if (!isActive || !activeTimer) {
      setElapsed(0);
      return;
    }

    const interval = setInterval(() => {
      const now = new Date();
      const diff = Math.floor((now.getTime() - activeTimer.startTime.getTime()) / 1000);
      setElapsed(diff);
    }, 1000);

    return () => clearInterval(interval);
  }, [isActive, activeTimer]);

  const handleStart = () => {
    startTimer(taskId);
    toast.success('Timer started');
  };

  const handleStop = () => {
    const entry = stopTimer();
    if (entry) {
      toast.success(`Logged ${formatDuration(entry.duration || 0)} to task`);
    }
  };

  const stats = getTaskStats(taskId);

  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <Button
          variant={isActive ? 'default' : 'outline'}
          size="sm"
          onClick={isActive ? handleStop : handleStart}
          className="gap-2"
        >
          {isActive ? <Pause className="h-3 w-3" /> : <Play className="h-3 w-3" />}
          {isActive ? formatDuration(elapsed) : 'Start'}
        </Button>
        {stats.totalTime > 0 && !isActive && (
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Clock className="h-3 w-3" />
            {formatDuration(stats.totalTime)}
          </span>
        )}
      </div>
    );
  }

  return (
    <Card>
      <CardContent className="p-4">
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Clock className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium">Time Tracking</span>
            </div>
            <Button
              variant={isActive ? 'default' : 'outline'}
              size="sm"
              onClick={isActive ? handleStop : handleStart}
              className="gap-2"
            >
              {isActive ? (
                <>
                  <Pause className="h-4 w-4" />
                  Stop Timer
                </>
              ) : (
                <>
                  <Play className="h-4 w-4" />
                  Start Timer
                </>
              )}
            </Button>
          </div>

          {isActive && (
            <div className="text-center">
              <div className="text-2xl font-mono font-semibold">
                {formatDuration(elapsed)}
              </div>
              <p className="text-xs text-muted-foreground mt-1">Active timer</p>
            </div>
          )}

          {stats.totalTime > 0 && (
            <div className="grid grid-cols-3 gap-2 pt-2 border-t">
              <div className="text-center">
                <div className="text-sm font-medium">
                  {formatDuration(stats.totalTime)}
                </div>
                <p className="text-xs text-muted-foreground">Total</p>
              </div>
              <div className="text-center">
                <div className="text-sm font-medium">
                  {formatDuration(stats.todayTime)}
                </div>
                <p className="text-xs text-muted-foreground">Today</p>
              </div>
              <div className="text-center">
                <div className="text-sm font-medium">{stats.entryCount}</div>
                <p className="text-xs text-muted-foreground">
                  {isActive ? 'Entries' : 'Total'}
                </p>
              </div>
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
