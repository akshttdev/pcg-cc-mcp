import { useState, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Mail,
  Link2,
  Unlink,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Clock,
  Inbox,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { EmailAccount, EmailProvider, EmailAccountStatus } from '@/types/email';

interface EmailAccountConnectProps {
  accounts: EmailAccount[];
  onConnect?: (provider: EmailProvider) => void;
  onDisconnect?: (accountId: string) => void;
  onRefresh?: (accountId: string) => void;
  onSync?: (accountId: string) => void;
  isConnecting?: EmailProvider | null;
  className?: string;
}

const providerConfig: Record<EmailProvider, {
  name: string;
  icon: React.ReactNode;
  color: string;
  bgColor: string;
  description: string;
}> = {
  gmail: {
    name: 'Gmail',
    icon: <Mail className="h-5 w-5" />,
    color: 'text-[#EA4335]',
    bgColor: 'bg-[#EA4335]',
    description: 'Google Gmail - Use as master credentials for social platforms',
  },
  zoho: {
    name: 'Zoho Mail',
    icon: <span className="text-lg font-bold">Z</span>,
    color: 'text-[#C8202B]',
    bgColor: 'bg-[#C8202B]',
    description: 'Zoho Mail - Team operations and CRM integration',
  },
  outlook: {
    name: 'Outlook',
    icon: <span className="text-lg font-bold">O</span>,
    color: 'text-[#0078D4]',
    bgColor: 'bg-[#0078D4]',
    description: 'Microsoft Outlook / Office 365',
  },
  imap_custom: {
    name: 'Custom IMAP',
    icon: <span className="text-lg font-bold">@</span>,
    color: 'text-gray-600',
    bgColor: 'bg-gray-600',
    description: 'Connect any email provider via IMAP/SMTP',
  },
};

const statusConfig: Record<EmailAccountStatus, {
  label: string;
  icon: React.ReactNode;
  color: string;
}> = {
  active: {
    label: 'Connected',
    icon: <CheckCircle2 className="h-3 w-3" />,
    color: 'bg-green-100 text-green-700 border-green-200',
  },
  inactive: {
    label: 'Inactive',
    icon: <Unlink className="h-3 w-3" />,
    color: 'bg-gray-100 text-gray-700 border-gray-200',
  },
  expired: {
    label: 'Token Expired',
    icon: <AlertCircle className="h-3 w-3" />,
    color: 'bg-red-100 text-red-700 border-red-200',
  },
  error: {
    label: 'Error',
    icon: <AlertCircle className="h-3 w-3" />,
    color: 'bg-red-100 text-red-700 border-red-200',
  },
  pending_auth: {
    label: 'Pending',
    icon: <Clock className="h-3 w-3" />,
    color: 'bg-yellow-100 text-yellow-700 border-yellow-200',
  },
  revoked: {
    label: 'Revoked',
    icon: <Unlink className="h-3 w-3" />,
    color: 'bg-gray-100 text-gray-700 border-gray-200',
  },
};

function ConnectedAccountCard({
  account,
  onDisconnect,
  onSync,
}: {
  account: EmailAccount;
  onDisconnect?: () => void;
  onSync?: () => void;
}) {
  const config = providerConfig[account.provider];
  const status = statusConfig[account.status];
  const [isSyncing, setIsSyncing] = useState(false);

  const handleSync = useCallback(async () => {
    if (!onSync) return;
    setIsSyncing(true);
    try {
      await onSync();
    } finally {
      setIsSyncing(false);
    }
  }, [onSync]);

  const formatDate = (dateString: string) => {
    return new Date(dateString).toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  const formatRelativeTime = (dateString: string | undefined | null) => {
    if (!dateString) return 'Never';
    const date = new Date(dateString);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    return `${diffDays}d ago`;
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
                alt={account.display_name || account.email_address}
                className="w-full h-full rounded-full object-cover"
              />
            ) : (
              config.icon
            )}
          </div>

          {/* Info */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-medium truncate">
                {account.display_name || account.email_address}
              </h4>
              <Badge variant="outline" className={cn('text-xs', status.color)}>
                {status.icon}
                <span className="ml-1">{status.label}</span>
              </Badge>
            </div>
            <p className="text-sm text-muted-foreground flex items-center gap-2">
              <span className={config.color}>{config.name}</span>
              <span className="text-gray-300">|</span>
              <span className="truncate">{account.email_address}</span>
            </p>
            {account.unread_count !== null && account.unread_count !== undefined && account.unread_count > 0 && (
              <p className="text-sm text-muted-foreground mt-1 flex items-center gap-1">
                <Inbox className="h-3 w-3" />
                <span>{account.unread_count} unread</span>
              </p>
            )}
            <div className="flex items-center gap-3 mt-2 text-xs text-muted-foreground">
              <span>
                Connected {formatDate(account.created_at)}
              </span>
              {account.last_sync_at && (
                <span>
                  Synced {formatRelativeTime(account.last_sync_at)}
                </span>
              )}
            </div>
          </div>

          {/* Actions */}
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={handleSync}
              disabled={isSyncing}
              title="Sync now"
            >
              <RefreshCw className={cn('h-4 w-4', isSyncing && 'animate-spin')} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              onClick={onDisconnect}
              title="Disconnect"
            >
              <Unlink className="h-4 w-4" />
            </Button>
          </div>
        </div>

        {/* Error message */}
        {account.last_error && (
          <div className="mt-3 p-2 bg-red-50 border border-red-200 rounded text-sm text-red-700">
            {account.last_error}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function ProviderConnectButton({
  provider,
  onConnect,
  isConnecting,
  existingAccount,
}: {
  provider: EmailProvider;
  onConnect?: (provider: EmailProvider) => void;
  isConnecting: boolean;
  existingAccount?: EmailAccount;
}) {
  const config = providerConfig[provider];

  const handleConnect = () => {
    if (onConnect && !existingAccount) {
      onConnect(provider);
    }
  };

  return (
    <button
      onClick={handleConnect}
      disabled={isConnecting || !!existingAccount}
      className={cn(
        'flex items-center gap-3 p-4 rounded-lg border transition-all',
        existingAccount
          ? 'border-gray-200 bg-gray-50 cursor-not-allowed opacity-60'
          : 'border-gray-200 hover:border-gray-300 hover:bg-gray-50 cursor-pointer'
      )}
    >
      <div
        className={cn(
          'w-10 h-10 rounded-lg flex items-center justify-center text-white',
          config.bgColor
        )}
      >
        {config.icon}
      </div>
      <div className="text-left flex-1">
        <h4 className="font-medium">{config.name}</h4>
        <p className="text-sm text-muted-foreground">{config.description}</p>
      </div>
      <div className="flex items-center gap-2">
        {existingAccount ? (
          <Badge variant="outline" className="bg-green-100 text-green-700">
            <CheckCircle2 className="h-3 w-3 mr-1" />
            Connected
          </Badge>
        ) : (
          <Button
            size="sm"
            variant="outline"
            disabled={isConnecting}
            className="gap-1"
          >
            {isConnecting ? (
              <>
                <RefreshCw className="h-3 w-3 animate-spin" />
                Connecting...
              </>
            ) : (
              <>
                <Link2 className="h-3 w-3" />
                Connect
              </>
            )}
          </Button>
        )}
      </div>
    </button>
  );
}

export function EmailAccountConnect({
  accounts,
  onConnect,
  onDisconnect,
  onSync,
  isConnecting,
  className,
}: EmailAccountConnectProps) {
  const connectedAccounts = accounts.filter((a) => a.status !== 'revoked');
  const providers: EmailProvider[] = ['gmail', 'zoho', 'outlook'];

  const getExistingAccount = (provider: EmailProvider) => {
    return connectedAccounts.find((a) => a.provider === provider);
  };

  return (
    <div className={cn('space-y-6', className)}>
      {/* Connected Accounts */}
      {connectedAccounts.length > 0 && (
        <div className="space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-lg font-semibold">Connected Email Accounts</h3>
            <Badge variant="secondary">
              {connectedAccounts.length} connected
            </Badge>
          </div>
          <div className="grid gap-4">
            {connectedAccounts.map((account) => (
              <ConnectedAccountCard
                key={account.id}
                account={account}
                onDisconnect={
                  onDisconnect ? () => onDisconnect(account.id) : undefined
                }
                onSync={onSync ? () => onSync(account.id) : undefined}
              />
            ))}
          </div>
        </div>
      )}

      {/* Connect New Account */}
      <Card>
        <CardHeader>
          <CardTitle className="text-lg">Connect Email Account</CardTitle>
          <CardDescription>
            Connect your email accounts to enable master authentication for social platforms
            and unified communication management.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {providers.map((provider) => (
            <ProviderConnectButton
              key={provider}
              provider={provider}
              onConnect={onConnect}
              isConnecting={isConnecting === provider}
              existingAccount={getExistingAccount(provider)}
            />
          ))}
        </CardContent>
      </Card>

      {/* Gmail Benefits */}
      <Card className="bg-gradient-to-r from-red-50 to-orange-50 border-red-100">
        <CardContent className="p-4">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-lg bg-[#EA4335] flex items-center justify-center text-white">
              <Mail className="h-5 w-5" />
            </div>
            <div>
              <h4 className="font-medium text-gray-900">Gmail as Master Credentials</h4>
              <p className="text-sm text-gray-600 mt-1">
                Use your Gmail account as the master authentication for signing into
                social platforms. This simplifies credential management and enables
                unified access across all connected services.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Zoho Integration */}
      <Card className="bg-gradient-to-r from-red-50 to-pink-50 border-red-100">
        <CardContent className="p-4">
          <div className="flex items-start gap-3">
            <div className="w-10 h-10 rounded-lg bg-[#C8202B] flex items-center justify-center text-white">
              <span className="text-lg font-bold">Z</span>
            </div>
            <div>
              <h4 className="font-medium text-gray-900">Zoho Mail + CRM Integration</h4>
              <p className="text-sm text-gray-600 mt-1">
                Connect Zoho Mail for internal team operations. This integration also
                enables Zoho CRM sync for contact management, deal tracking, and
                automated workflow triggers.
              </p>
              <div className="flex flex-wrap gap-2 mt-2">
                <Badge variant="secondary" className="text-xs">
                  Team Email
                </Badge>
                <Badge variant="secondary" className="text-xs">
                  CRM Sync
                </Badge>
                <Badge variant="secondary" className="text-xs">
                  Contact Management
                </Badge>
                <Badge variant="secondary" className="text-xs">
                  Deal Pipeline
                </Badge>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

export default EmailAccountConnect;
