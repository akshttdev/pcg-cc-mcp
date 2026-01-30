import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import {
  Globe,
  Wifi,
  WifiOff,
  Coins,
  Cpu,
  HardDrive,
  Users,
  Activity,
  ChevronRight,
} from 'lucide-react';
import { cn } from '@/lib/utils';

interface MobileMeshCardProps {
  nodeId: string;
  status: 'online' | 'offline' | 'connecting';
  peersConnected: number;
  vibeBalance: number;
  cpuUsage: number;
  memoryUsage: number;
  activeTasks: number;
  relayConnected: boolean;
  onPress?: () => void;
}

export function MobileMeshCard({
  nodeId,
  status,
  peersConnected,
  vibeBalance,
  cpuUsage,
  memoryUsage,
  activeTasks,
  relayConnected,
  onPress,
}: MobileMeshCardProps) {
  const statusColors = {
    online: 'bg-green-500',
    offline: 'bg-red-500',
    connecting: 'bg-yellow-500',
  };

  return (
    <Card
      className={cn(
        'overflow-hidden touch-manipulation active:scale-[0.98] transition-transform',
        onPress && 'cursor-pointer'
      )}
      onClick={onPress}
    >
      <CardContent className="p-4">
        {/* Header Row */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Globe className="h-5 w-5 text-primary" />
            <div>
              <h3 className="font-semibold text-sm">Alpha Protocol</h3>
              <p className="text-xs text-muted-foreground font-mono">{nodeId}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <span className={cn('h-2 w-2 rounded-full animate-pulse', statusColors[status])} />
            <Badge variant={status === 'online' ? 'default' : 'secondary'} className="text-xs">
              {status.toUpperCase()}
            </Badge>
            {onPress && <ChevronRight className="h-4 w-4 text-muted-foreground" />}
          </div>
        </div>

        {/* Stats Grid */}
        <div className="grid grid-cols-2 gap-3 mb-4">
          <StatItem
            icon={Coins}
            label="Vibe Balance"
            value={vibeBalance.toFixed(2)}
            color="text-yellow-500"
          />
          <StatItem
            icon={Users}
            label="Peers"
            value={peersConnected.toString()}
            color="text-blue-500"
          />
          <StatItem
            icon={Activity}
            label="Active Tasks"
            value={activeTasks.toString()}
            color="text-purple-500"
          />
          <StatItem
            icon={relayConnected ? Wifi : WifiOff}
            label="Relay"
            value={relayConnected ? 'Connected' : 'Offline'}
            color={relayConnected ? 'text-green-500' : 'text-red-500'}
          />
        </div>

        {/* Resource Bars */}
        <div className="space-y-2">
          <ResourceBar
            icon={Cpu}
            label="CPU"
            value={cpuUsage}
          />
          <ResourceBar
            icon={HardDrive}
            label="Memory"
            value={memoryUsage}
          />
        </div>
      </CardContent>
    </Card>
  );
}

function StatItem({
  icon: Icon,
  label,
  value,
  color,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  color: string;
}) {
  return (
    <div className="flex items-center gap-2 p-2 bg-muted/50 rounded-lg">
      <Icon className={cn('h-4 w-4', color)} />
      <div className="min-w-0">
        <p className="text-xs text-muted-foreground truncate">{label}</p>
        <p className="text-sm font-medium truncate">{value}</p>
      </div>
    </div>
  );
}

function ResourceBar({
  icon: Icon,
  label,
  value,
}: {
  icon: React.ElementType;
  label: string;
  value: number;
}) {
  return (
    <div className="flex items-center gap-2">
      <Icon className="h-4 w-4 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="flex justify-between text-xs mb-1">
          <span className="text-muted-foreground">{label}</span>
          <span className="font-medium">{value.toFixed(0)}%</span>
        </div>
        <Progress value={value} className="h-1.5" />
      </div>
    </div>
  );
}

export default MobileMeshCard;
