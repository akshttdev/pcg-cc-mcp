import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  AlertCircle,
  CheckCircle2,
  Cloud,
  CloudOff,
  HardDrive,
  Loader2,
  RefreshCw,
  Trash2,
  XCircle,
} from 'lucide-react';
import { useMemo, useState } from 'react';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { useOrganization } from '@/contexts/organization-context';
import {
  type StorageAccountView,
  storageApi,
  type StorageSyncResult,
} from '@/lib/api/storage';
import { cn } from '@/lib/utils';

// ─── Provider catalog ──────────────────────────────────────────────────────
// One row per supported provider. Drives the "Connect a drive" cards plus
// the small badge logic in the connected list. Order matters — OneDrive is
// the priority integration for PCG so it goes first.

interface ProviderInfo {
  id: 'onedrive' | 'dropbox' | 'gdrive';
  label: string;
  description: string;
  brandColor: string;
}

const PROVIDERS: ProviderInfo[] = [
  {
    id: 'onedrive',
    label: 'OneDrive',
    description:
      'Sync files from OneDrive (personal or work) into the knowledge graph and media library.',
    brandColor: 'text-blue-600 dark:text-blue-400',
  },
  {
    id: 'dropbox',
    label: 'Dropbox',
    description:
      'Sync files from a Dropbox account or app folder. Cursor-based, picks up new and deleted files.',
    brandColor: 'text-blue-500 dark:text-blue-300',
  },
  {
    id: 'gdrive',
    label: 'Google Drive',
    description:
      'Sync files from My Drive. Requires the same Google account used for Gmail / Calendar.',
    brandColor: 'text-yellow-600 dark:text-yellow-400',
  },
];

// ─── Page ──────────────────────────────────────────────────────────────────

export function StorageSettings() {
  const { effectiveOrgId } = useOrganization();
  const queryClient = useQueryClient();

  const accountsQuery = useQuery({
    queryKey: ['storage', 'accounts', effectiveOrgId],
    queryFn: () => storageApi.list(effectiveOrgId!),
    enabled: !!effectiveOrgId,
  });

  const accountsByProvider = useMemo(() => {
    const map = new Map<string, StorageAccountView[]>();
    for (const a of accountsQuery.data ?? []) {
      const list = map.get(a.provider) ?? [];
      list.push(a);
      map.set(a.provider, list);
    }
    return map;
  }, [accountsQuery.data]);

  const syncMutation = useMutation({
    mutationFn: (id: string) => storageApi.syncNow(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['storage', 'accounts'] });
    },
  });

  const disconnectMutation = useMutation({
    mutationFn: (id: string) => storageApi.disconnect(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['storage', 'accounts'] });
    },
  });

  if (!effectiveOrgId) {
    return (
      <Alert>
        <AlertCircle className="h-4 w-4" />
        <AlertTitle>No organization selected</AlertTitle>
        <AlertDescription>
          Pick an organization from the sidebar to manage cloud storage.
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <div className="space-y-6" data-testid="storage-settings">
      <div>
        <h2 className="text-2xl font-semibold tracking-tight">Cloud Storage</h2>
        <p className="text-muted-foreground">
          Connect OneDrive, Dropbox, and Google Drive accounts. Files are synced
          every 15 minutes and indexed for the knowledge graph.
        </p>
      </div>

      {accountsQuery.isError && (
        <Alert variant="destructive">
          <XCircle className="h-4 w-4" />
          <AlertTitle>Failed to load storage accounts</AlertTitle>
          <AlertDescription>
            {(accountsQuery.error as Error).message}
          </AlertDescription>
        </Alert>
      )}

      {/* Connected accounts */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <HardDrive className="h-5 w-5" />
            Connected drives
          </CardTitle>
          <CardDescription>
            Active connections that are being synced into ORCHA.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {accountsQuery.isLoading ? (
            <div className="flex items-center justify-center py-8 text-muted-foreground">
              <Loader2 className="mr-2 h-5 w-5 animate-spin" />
              Loading…
            </div>
          ) : (accountsQuery.data ?? []).length === 0 ? (
            <p
              className="rounded-lg border border-dashed py-8 text-center text-sm text-muted-foreground"
              data-testid="storage-empty"
            >
              No drives connected yet. Pick a provider below to get started.
            </p>
          ) : (
            <ul className="divide-y rounded-lg border">
              {(accountsQuery.data ?? []).map((account) => (
                <ConnectedAccountRow
                  key={account.id}
                  account={account}
                  onSync={() => syncMutation.mutate(account.id)}
                  onDisconnect={() => disconnectMutation.mutate(account.id)}
                  syncing={
                    syncMutation.isPending &&
                    syncMutation.variables === account.id
                  }
                  disconnecting={
                    disconnectMutation.isPending &&
                    disconnectMutation.variables === account.id
                  }
                  lastSyncResult={
                    syncMutation.isSuccess &&
                    syncMutation.variables === account.id
                      ? syncMutation.data
                      : undefined
                  }
                />
              ))}
            </ul>
          )}
        </CardContent>
      </Card>

      {/* Connect new */}
      <div className="grid gap-4 md:grid-cols-3">
        {PROVIDERS.map((provider) => (
          <ConnectProviderCard
            key={provider.id}
            provider={provider}
            organizationId={effectiveOrgId}
            existingCount={accountsByProvider.get(provider.id)?.length ?? 0}
          />
        ))}
      </div>
    </div>
  );
}

// ─── Connected drive row ───────────────────────────────────────────────────

interface ConnectedAccountRowProps {
  account: StorageAccountView;
  onSync: () => void;
  onDisconnect: () => void;
  syncing: boolean;
  disconnecting: boolean;
  lastSyncResult?: StorageSyncResult;
}

function ConnectedAccountRow({
  account,
  onSync,
  onDisconnect,
  syncing,
  disconnecting,
  lastSyncResult,
}: ConnectedAccountRowProps) {
  const provider = PROVIDERS.find((p) => p.id === account.provider);
  const label = provider?.label ?? account.provider;

  return (
    <li
      className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between"
      data-testid={`storage-account-${account.id}`}
    >
      <div className="flex items-start gap-3">
        <Cloud className={cn('mt-0.5 h-5 w-5', provider?.brandColor)} />
        <div className="space-y-1">
          <div className="flex flex-wrap items-center gap-2">
            <span className="font-medium">{label}</span>
            <StatusBadge status={account.status} />
            {account.display_name && (
              <span className="text-sm text-muted-foreground">
                {account.display_name}
              </span>
            )}
          </div>
          {account.account_email && (
            <p className="text-sm text-muted-foreground">
              {account.account_email}
            </p>
          )}
          <p className="text-xs text-muted-foreground">
            {account.last_sync_at
              ? `Last sync ${new Date(account.last_sync_at).toLocaleString()}`
              : 'Never synced'}
            {' · '}
            {account.total_files_synced.toLocaleString()} file
            {account.total_files_synced === 1 ? '' : 's'} indexed
            {' · '}
            every {Math.round(account.sync_interval_secs / 60)} min
          </p>
          {account.last_error && (
            <p className="text-xs text-red-600 dark:text-red-400">
              Last error: {account.last_error}
            </p>
          )}
          {lastSyncResult && (
            <p className="text-xs text-emerald-600 dark:text-emerald-400">
              Sync done — {lastSyncResult.files_added} added,{' '}
              {lastSyncResult.files_updated} updated,{' '}
              {lastSyncResult.files_deleted} deleted
            </p>
          )}
        </div>
      </div>
      <div className="flex items-center gap-2 self-start sm:self-center">
        <Button
          variant="outline"
          size="sm"
          onClick={onSync}
          disabled={syncing || disconnecting}
          data-testid={`storage-sync-${account.id}`}
        >
          {syncing ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <RefreshCw className="mr-2 h-4 w-4" />
          )}
          Sync now
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={onDisconnect}
          disabled={disconnecting || syncing}
          className="text-red-600 hover:bg-red-50 hover:text-red-700 dark:text-red-400 dark:hover:bg-red-950"
          data-testid={`storage-disconnect-${account.id}`}
        >
          {disconnecting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Trash2 className="h-4 w-4" />
          )}
        </Button>
      </div>
    </li>
  );
}

// ─── Connect-a-provider card ───────────────────────────────────────────────

interface ConnectProviderCardProps {
  provider: ProviderInfo;
  organizationId: string;
  existingCount: number;
}

function ConnectProviderCard({
  provider,
  organizationId,
  existingCount,
}: ConnectProviderCardProps) {
  const [redirecting, setRedirecting] = useState(false);

  const handleConnect = () => {
    setRedirecting(true);
    const url = storageApi.connectUrl({
      provider: provider.id,
      organization_id: organizationId,
      redirect_after: window.location.pathname,
    });
    window.location.href = url;
  };

  return (
    <Card data-testid={`storage-provider-${provider.id}`}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Cloud className={cn('h-5 w-5', provider.brandColor)} />
          {provider.label}
        </CardTitle>
        <CardDescription>{provider.description}</CardDescription>
      </CardHeader>
      <CardContent>
        {existingCount > 0 && (
          <p className="mb-3 text-xs text-muted-foreground">
            {existingCount} {existingCount === 1 ? 'account' : 'accounts'}{' '}
            already connected — adding another connects a different drive.
          </p>
        )}
        <Button
          onClick={handleConnect}
          disabled={redirecting}
          className="w-full"
          data-testid={`storage-connect-${provider.id}`}
        >
          {redirecting ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : null}
          Connect {provider.label}
        </Button>
      </CardContent>
    </Card>
  );
}

// ─── Status badge ──────────────────────────────────────────────────────────

function StatusBadge({ status }: { status: StorageAccountView['status'] }) {
  switch (status) {
    case 'active':
      return (
        <Badge
          variant="outline"
          className="border-emerald-200 text-emerald-700 dark:border-emerald-800 dark:text-emerald-400"
        >
          <CheckCircle2 className="mr-1 h-3 w-3" />
          Active
        </Badge>
      );
    case 'syncing':
      return (
        <Badge variant="outline">
          <Loader2 className="mr-1 h-3 w-3 animate-spin" />
          Syncing
        </Badge>
      );
    case 'expired':
      return (
        <Badge
          variant="outline"
          className="border-amber-200 text-amber-700 dark:border-amber-800 dark:text-amber-400"
        >
          <AlertCircle className="mr-1 h-3 w-3" />
          Expired
        </Badge>
      );
    case 'error':
      return (
        <Badge variant="destructive">
          <XCircle className="mr-1 h-3 w-3" />
          Error
        </Badge>
      );
    case 'disconnected':
      return (
        <Badge variant="secondary">
          <CloudOff className="mr-1 h-3 w-3" />
          Disconnected
        </Badge>
      );
  }
}
