import { useQuery } from '@tanstack/react-query';
import { Clock, Upload, Wifi, Bot, RefreshCw } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { orgCloudApi, type CloudContribution } from '@/lib/api/org-cloud';
import { orgCloudKeys } from '@/lib/query-keys';

interface CloudActivityProps {
  orgId: string;
}

const CHANNEL_ICONS: Record<string, typeof Upload> = {
  http: Upload,
  nats: Wifi,
  agent: Bot,
  pcg_sync: RefreshCw,
};

function formatRelativeTime(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

export function CloudActivity({ orgId }: CloudActivityProps) {
  const { data: contributions = [], isLoading } = useQuery({
    queryKey: orgCloudKeys.contributions(orgId),
    queryFn: () => orgCloudApi.getContributions(orgId, 100),
    enabled: !!orgId,
    staleTime: 30_000,
  });

  if (isLoading) {
    return (
      <div className="space-y-3 mt-4">
        {Array.from({ length: 5 }).map((_, i) => (
          <Skeleton key={i} className="h-12 rounded-lg" />
        ))}
      </div>
    );
  }

  if (contributions.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-16 text-muted-foreground mt-4">
        <Clock className="h-10 w-10 mb-3 opacity-40" />
        <p className="text-sm">No contributions yet</p>
        <p className="text-xs mt-1">Upload files or run the indexer to see activity</p>
      </div>
    );
  }

  return (
    <div className="space-y-2 mt-4">
      {contributions.map((c: CloudContribution) => {
        const ChannelIcon = CHANNEL_ICONS[c.channel] ?? Upload;
        return (
          <div key={c.id} className="flex items-center gap-3 p-2.5 bg-muted/30 rounded-lg">
            <div className="p-1.5 rounded bg-muted">
              <ChannelIcon className="h-3.5 w-3.5 text-muted-foreground" />
            </div>
            <div className="min-w-0 flex-1">
              <p className="text-xs font-medium truncate">
                {c.description || `${c.contribution_type} via ${c.channel}`}
              </p>
              <p className="text-[10px] text-muted-foreground">
                {c.user_id ? `User ${c.user_id.slice(0, 8)}...` : 'System'}
                {c.wallet_address && ` · ${c.wallet_address.slice(0, 10)}...`}
              </p>
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              <Badge variant="outline" className="text-[10px] h-4 py-0 capitalize">
                {c.contribution_type}
              </Badge>
              <span className="text-[10px] text-muted-foreground">
                {formatRelativeTime(c.created_at)}
              </span>
            </div>
          </div>
        );
      })}
    </div>
  );
}
