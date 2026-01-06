import { useState, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Linkedin,
  Instagram,
  Twitter,
  Facebook,
  Youtube,
  Link2,
  Unlink,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Clock,
  Users,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { SocialAccount, SocialPlatform } from '@/types/social';

interface SocialAccountConnectProps {
  accounts: SocialAccount[];
  onConnect?: (platform: SocialPlatform) => void;
  onDisconnect?: (accountId: string) => void;
  onRefresh?: (accountId: string) => void;
  isConnecting?: SocialPlatform | null;
  className?: string;
}

const platformConfig: Record<SocialPlatform, {
  name: string;
  icon: React.ReactNode;
  color: string;
  bgColor: string;
  description: string;
}> = {
  linkedin: {
    name: 'LinkedIn',
    icon: <Linkedin className="h-5 w-5" />,
    color: 'text-[#0A66C2]',
    bgColor: 'bg-[#0A66C2]',
    description: 'Professional networking & B2B content',
  },
  instagram: {
    name: 'Instagram',
    icon: <Instagram className="h-5 w-5" />,
    color: 'text-[#E4405F]',
    bgColor: 'bg-gradient-to-r from-[#F58529] via-[#DD2A7B] to-[#8134AF]',
    description: 'Visual content, stories & reels',
  },
  twitter: {
    name: 'X (Twitter)',
    icon: <Twitter className="h-5 w-5" />,
    color: 'text-black',
    bgColor: 'bg-black',
    description: 'Real-time updates & engagement',
  },
  facebook: {
    name: 'Facebook',
    icon: <Facebook className="h-5 w-5" />,
    color: 'text-[#1877F2]',
    bgColor: 'bg-[#1877F2]',
    description: 'Community building & events',
  },
  tiktok: {
    name: 'TikTok',
    icon: <span className="text-lg font-bold">TT</span>,
    color: 'text-black',
    bgColor: 'bg-black',
    description: 'Short-form video content',
  },
  youtube: {
    name: 'YouTube',
    icon: <Youtube className="h-5 w-5" />,
    color: 'text-[#FF0000]',
    bgColor: 'bg-[#FF0000]',
    description: 'Long-form video & shorts',
  },
  bluesky: {
    name: 'Bluesky',
    icon: <span className="text-lg font-bold">🦋</span>,
    color: 'text-[#0085FF]',
    bgColor: 'bg-[#0085FF]',
    description: 'Decentralized social',
  },
  pinterest: {
    name: 'Pinterest',
    icon: <span className="text-lg font-bold">P</span>,
    color: 'text-[#E60023]',
    bgColor: 'bg-[#E60023]',
    description: 'Visual discovery & pins',
  },
  threads: {
    name: 'Threads',
    icon: <span className="text-lg font-bold">@</span>,
    color: 'text-black',
    bgColor: 'bg-black',
    description: 'Text-based conversations',
  },
};

const statusConfig: Record<SocialAccount['status'], {
  label: string;
  icon: React.ReactNode;
  color: string;
}> = {
  active: {
    label: 'Connected',
    icon: <CheckCircle2 className="h-3 w-3" />,
    color: 'bg-green-100 text-green-700 border-green-200',
  },
  disconnected: {
    label: 'Disconnected',
    icon: <Unlink className="h-3 w-3" />,
    color: 'bg-gray-100 text-gray-700 border-gray-200',
  },
  expired: {
    label: 'Token Expired',
    icon: <AlertCircle className="h-3 w-3" />,
    color: 'bg-red-100 text-red-700 border-red-200',
  },
  pending: {
    label: 'Pending',
    icon: <Clock className="h-3 w-3" />,
    color: 'bg-yellow-100 text-yellow-700 border-yellow-200',
  },
};

function ConnectedAccountCard({
  account,
  onDisconnect,
  onRefresh,
}: {
  account: SocialAccount;
  onDisconnect?: () => void;
  onRefresh?: () => void;
}) {
  const config = platformConfig[account.platform];
  const status = statusConfig[account.status];
  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleRefresh = useCallback(async () => {
    if (!onRefresh) return;
    setIsRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setIsRefreshing(false);
    }
  }, [onRefresh]);

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  const formatFollowers = (count?: number) => {
    if (!count) return null;
    if (count >= 1000000) return `${(count / 1000000).toFixed(1)}M`;
    if (count >= 1000) return `${(count / 1000).toFixed(1)}K`;
    return count.toString();
  };

  return (
    <Card>
      <CardContent className="p-4">
        <div className="flex items-start gap-3">
          {/* Avatar/Icon */}
          <div
            className={cn(
              'w-12 h-12 rounded-full flex items-center justify-center text-white',
              config.bgColor
            )}
          >
            {account.avatar_url ? (
              <img
                src={account.avatar_url}
                alt={account.account_name}
                className="w-full h-full rounded-full object-cover"
              />
            ) : (
              config.icon
            )}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-medium truncate">{account.account_name}</h4>
              <Badge variant="outline" className={cn('text-xs', status.color)}>
                {status.icon}
                <span className="ml-1">{status.label}</span>
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground flex items-center gap-2">
              <span className={config.color}>{config.name}</span>
              {account.follower_count && (
                <>
                  <span>•</span>
                  <span className="flex items-center gap-1">
                    <Users className="h-3 w-3" />
                    {formatFollowers(account.follower_count)}
                  </span>
                </>
              )}
            </p>
            <p className="text-xs text-muted-foreground mt-1">
              Connected {formatDate(account.connected_at)}
              {account.last_synced_at && (
                <> • Last synced {formatDate(account.last_synced_at)}</>
              )}
            </p>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1">
            {onRefresh && (
              <Button
                variant="ghost"
                size="icon"
                onClick={handleRefresh}
                disabled={isRefreshing}
              >
                <RefreshCw
                  className={cn('h-4 w-4', isRefreshing && 'animate-spin')}
                />
              </Button>
            )}
            {onDisconnect && (
              <Button
                variant="ghost"
                size="icon"
                className="text-destructive hover:text-destructive"
                onClick={onDisconnect}
              >
                <Unlink className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      </CardContent>
    </Card>
  );
}

function PlatformConnectButton({
  platform,
  isConnected,
  isConnecting,
  onConnect,
}: {
  platform: SocialPlatform;
  isConnected: boolean;
  isConnecting: boolean;
  onConnect: () => void;
}) {
  const config = platformConfig[platform];

  return (
    <Button
      variant="outline"
      className={cn(
        'h-auto p-4 flex flex-col items-center gap-2 transition-all',
        isConnected && 'opacity-50 cursor-not-allowed',
        !isConnected && 'hover:border-2'
      )}
      style={!isConnected ? { borderColor: 'transparent' } : undefined}
      disabled={isConnected || isConnecting}
      onClick={onConnect}
    >
      <div className={cn('p-2 rounded-full text-white', config.bgColor)}>
        {config.icon}
      </div>
      <span className="font-medium text-sm">{config.name}</span>
      {isConnecting ? (
        <RefreshCw className="h-3 w-3 animate-spin" />
      ) : isConnected ? (
        <CheckCircle2 className="h-3 w-3 text-green-500" />
      ) : (
        <Link2 className="h-3 w-3 text-muted-foreground" />
      )}
    </Button>
  );
}

export function SocialAccountConnect({
  accounts,
  onConnect,
  onDisconnect,
  onRefresh,
  isConnecting,
  className,
}: SocialAccountConnectProps) {
  const connectedPlatforms = new Set(accounts.map(a => a.platform));

  const priorityPlatforms: SocialPlatform[] = [
    'linkedin', 'instagram', 'twitter', 'facebook', 'tiktok', 'youtube'
  ];

  const otherPlatforms: SocialPlatform[] = ['bluesky', 'pinterest', 'threads'];

  return (
    <div className={cn('space-y-6', className)}>
      {/* Connected accounts */}
      {accounts.length > 0 && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-lg">Connected Accounts</CardTitle>
            <CardDescription>
              Manage your connected social media accounts
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {accounts.map((account) => (
              <ConnectedAccountCard
                key={account.id}
                account={account}
                onDisconnect={onDisconnect ? () => onDisconnect(account.id) : undefined}
                onRefresh={onRefresh ? () => onRefresh(account.id) : undefined}
              />
            ))}
          </CardContent>
        </Card>
      )}

      {/* Connect new accounts */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-lg">Connect Account</CardTitle>
          <CardDescription>
            Add a new social media account to manage
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {/* Primary platforms */}
            <div className="grid grid-cols-3 sm:grid-cols-6 gap-2">
              {priorityPlatforms.map((platform) => (
                <PlatformConnectButton
                  key={platform}
                  platform={platform}
                  isConnected={connectedPlatforms.has(platform)}
                  isConnecting={isConnecting === platform}
                  onConnect={() => onConnect?.(platform)}
                />
              ))}
            </div>

            {/* Other platforms */}
            <div className="pt-2 border-t">
              <p className="text-xs text-muted-foreground mb-2">More platforms</p>
              <div className="grid grid-cols-3 gap-2">
                {otherPlatforms.map((platform) => (
                  <PlatformConnectButton
                    key={platform}
                    platform={platform}
                    isConnected={connectedPlatforms.has(platform)}
                    isConnecting={isConnecting === platform}
                    onConnect={() => onConnect?.(platform)}
                  />
                ))}
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
