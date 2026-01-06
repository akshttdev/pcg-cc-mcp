import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import { Cpu, HardDrive, MemoryStick, Activity, Server } from 'lucide-react';
import { useSystemMetrics } from '@/hooks/useSystemMetrics';
import { cn } from '@/lib/utils';

interface SystemMetricsPanelProps {
  className?: string;
  compact?: boolean;
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

function formatUptime(seconds: number): string {
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);

  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

function getStatusColor(percent: number): string {
  if (percent < 50) return 'bg-green-500';
  if (percent < 80) return 'bg-yellow-500';
  return 'bg-red-500';
}

function MetricBar({
  label,
  value,
  max,
  icon: Icon,
  unit = '%',
}: {
  label: string;
  value: number;
  max?: number;
  icon: React.ElementType;
  unit?: string;
}) {
  const percent = max ? (value / max) * 100 : value;
  const displayValue = max ? `${formatBytes(value)} / ${formatBytes(max)}` : `${value.toFixed(1)}${unit}`;

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between text-sm">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Icon className="h-3.5 w-3.5" />
          <span>{label}</span>
        </div>
        <span className="font-medium">{displayValue}</span>
      </div>
      <Progress
        value={percent}
        className="h-2"
        indicatorClassName={getStatusColor(percent)}
      />
    </div>
  );
}

export function SystemMetricsPanel({ className, compact }: SystemMetricsPanelProps) {
  const { data: metrics, isLoading, error } = useSystemMetrics();

  if (isLoading) {
    return (
      <Card className={cn('animate-pulse', className)}>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Activity className="h-4 w-4" />
            System Metrics
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-3">
          {[1, 2, 3, 4].map((i) => (
            <div key={i} className="space-y-1">
              <div className="h-4 w-24 bg-muted rounded" />
              <div className="h-2 w-full bg-muted rounded" />
            </div>
          ))}
        </CardContent>
      </Card>
    );
  }

  if (error || !metrics) {
    return (
      <Card className={className}>
        <CardHeader className="pb-2">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Activity className="h-4 w-4" />
            System Metrics
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">Unable to load metrics</p>
        </CardContent>
      </Card>
    );
  }

  if (compact) {
    return (
      <div className={cn('flex items-center gap-4 text-xs', className)}>
        <div className="flex items-center gap-1">
          <Cpu className="h-3 w-3 text-muted-foreground" />
          <span className={cn(
            'font-medium',
            metrics.cpu_usage_percent > 80 && 'text-red-500',
            metrics.cpu_usage_percent > 50 && metrics.cpu_usage_percent <= 80 && 'text-yellow-500',
          )}>
            {metrics.cpu_usage_percent.toFixed(0)}%
          </span>
        </div>
        <div className="flex items-center gap-1">
          <MemoryStick className="h-3 w-3 text-muted-foreground" />
          <span className={cn(
            'font-medium',
            metrics.memory_usage_percent > 80 && 'text-red-500',
            metrics.memory_usage_percent > 50 && metrics.memory_usage_percent <= 80 && 'text-yellow-500',
          )}>
            {metrics.memory_usage_percent.toFixed(0)}%
          </span>
        </div>
        <div className="flex items-center gap-1">
          <Server className="h-3 w-3 text-muted-foreground" />
          <span>{metrics.process_count}</span>
        </div>
      </div>
    );
  }

  return (
    <Card className={className}>
      <CardHeader className="pb-2">
        <div className="flex items-center justify-between">
          <CardTitle className="text-sm font-medium flex items-center gap-2">
            <Activity className="h-4 w-4" />
            System Metrics
          </CardTitle>
          <Badge variant="outline" className="text-xs">
            Uptime: {formatUptime(Number(metrics.uptime_seconds))}
          </Badge>
        </div>
      </CardHeader>
      <CardContent className="space-y-4">
        <MetricBar
          label="CPU"
          value={metrics.cpu_usage_percent}
          icon={Cpu}
        />
        <MetricBar
          label="Memory"
          value={metrics.memory_used_bytes}
          max={metrics.memory_total_bytes}
          icon={MemoryStick}
        />
        <MetricBar
          label="Disk"
          value={metrics.disk_used_bytes}
          max={metrics.disk_total_bytes}
          icon={HardDrive}
        />

        <div className="pt-2 border-t">
          <div className="flex items-center justify-between text-xs">
            <span className="text-muted-foreground">Active Processes</span>
            <span className="font-medium">{metrics.process_count}</span>
          </div>
          <div className="flex items-center justify-between text-xs mt-1">
            <span className="text-muted-foreground">Load Average</span>
            <span className="font-medium">
              {metrics.load_average.one_minute.toFixed(2)} / {metrics.load_average.five_minutes.toFixed(2)} / {metrics.load_average.fifteen_minutes.toFixed(2)}
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
