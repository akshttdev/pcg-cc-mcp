import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Boxes,
  CheckCircle2,
  DollarSign,
  ExternalLink,
  FileText,
  Globe,
  Loader2,
  Mail,
  Plug,
  Plus,
  RefreshCw,
  Share2,
  Trash2,
} from 'lucide-react';
import { useState } from 'react';
import { Link } from 'react-router-dom';

import { useUserSystem } from '@/components/config-provider';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  airtableApi,
  type EmailAccountRecord,
  emailApi,
  quickbooksApi,
  socialApi,
} from '@/lib/api';
import { type StorageAccountView, storageApi } from '@/lib/api/storage';
import { integrationKeys, socialKeys } from '@/lib/query-keys';

import { IntegrationCard } from '../../components/IntegrationCard';
import { PLATFORM_BG, PLATFORM_COLORS, PLATFORM_ICONS } from '../../constants';
import { CommunicationSection } from './CommunicationSection';
import { DevelopmentSection } from './DevelopmentSection';
import { QbFinancialSummary } from './QbFinancialSummary';

// ─── Storage providers (OneDrive / Dropbox) ───────────────────────────────

const STORAGE_PROVIDERS = [
  {
    id: 'onedrive' as const,
    label: 'OneDrive',
    accent: '#0078D4',
    description:
      'Access Microsoft OneDrive files as knowledge sources and shared asset storage.',
  },
  {
    id: 'dropbox' as const,
    label: 'Dropbox',
    accent: '#0061FF',
    description:
      'Connect Dropbox folders as knowledge sources and asset storage for projects.',
  },
];

function StorageProviderCards({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [connecting, setConnecting] = useState<string | null>(null);

  const { data: accounts = [] } = useQuery<StorageAccountView[]>({
    queryKey: ['storage-accounts', orgId],
    queryFn: () => storageApi.list(orgId),
    staleTime: 30_000,
  });

  const syncMut = useMutation({
    mutationFn: (id: string) => storageApi.syncNow(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['storage-accounts', orgId] }),
  });

  const disconnectMut = useMutation({
    mutationFn: (id: string) => storageApi.disconnect(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ['storage-accounts', orgId] }),
  });

  const handleConnect = (provider: 'onedrive' | 'dropbox') => {
    setConnecting(provider);
    window.location.href = storageApi.connectUrl({
      provider,
      organization_id: orgId,
      redirect_after: window.location.pathname + window.location.search,
    });
  };

  return (
    <>
      {STORAGE_PROVIDERS.map((p) => {
        const connected = accounts.filter((a) => a.provider === p.id);
        const primary = connected[0];
        const isActive = primary?.status === 'active';
        const isError =
          primary?.status === 'error' || primary?.status === 'expired';
        return (
          <IntegrationCard
            key={p.id}
            accent={p.accent}
            icon={FileText}
            name={p.label}
            description={
              primary
                ? (primary.account_email ??
                  primary.display_name ??
                  p.description)
                : p.description
            }
            status={
              primary
                ? isActive
                  ? 'connected'
                  : isError
                    ? 'warning'
                    : 'connected'
                : 'disconnected'
            }
            actions={
              primary ? (
                <>
                  <button
                    className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1 disabled:opacity-50"
                    onClick={() => syncMut.mutate(primary.id)}
                    disabled={syncMut.isPending}
                  >
                    {syncMut.isPending ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <RefreshCw className="h-3 w-3" />
                    )}
                    Sync
                  </button>
                  <button
                    className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1 disabled:opacity-50"
                    onClick={() =>
                      confirm(`Disconnect ${p.label}?`) &&
                      disconnectMut.mutate(primary.id)
                    }
                    disabled={disconnectMut.isPending}
                  >
                    <Trash2 className="h-3 w-3" />
                    Disconnect
                  </button>
                </>
              ) : (
                <button
                  className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                  onClick={() => handleConnect(p.id)}
                  disabled={connecting === p.id}
                >
                  {connecting === p.id ? (
                    <Loader2 className="h-3 w-3 animate-spin" />
                  ) : (
                    <Plug className="h-3 w-3" />
                  )}
                  Connect
                </button>
              )
            }
            extra={
              primary?.last_sync_at ? (
                <p className="text-xs text-muted-foreground">
                  Last sync {new Date(primary.last_sync_at).toLocaleString()}
                  {primary.total_files_synced > 0 &&
                    ` · ${primary.total_files_synced} files`}
                </p>
              ) : primary?.last_error ? (
                <p className="text-xs text-destructive">{primary.last_error}</p>
              ) : undefined
            }
          />
        );
      })}
    </>
  );
}

// ─── Inline social section ────────────────────────────────────────────────

const SOCIAL_PLATFORMS = [
  { key: 'linkedin', label: 'LinkedIn' },
  { key: 'instagram', label: 'Instagram' },
  { key: 'twitter', label: 'X / Twitter' },
  { key: 'tiktok', label: 'TikTok' },
  { key: 'youtube', label: 'YouTube' },
  { key: 'facebook', label: 'Facebook' },
];

function SocialConnectDialog({
  open,
  onClose,
  orgId,
}: {
  open: boolean;
  onClose: () => void;
  orgId: string;
}) {
  const { data: platformStatus = [] } = useQuery({
    queryKey: ['social-platform-status'],
    queryFn: () => socialApi.getPlatformStatus(),
    staleTime: 300_000,
    enabled: open,
  });

  const configuredSet = new Set(
    platformStatus.filter((p) => p.configured).map((p) => p.platform)
  );

  const handleConnect = (platform: string) => {
    socialApi.connectAccount(platform, { orgId });
    onClose();
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-md">
        <DialogHeader>
          <DialogTitle>Connect Social Account</DialogTitle>
        </DialogHeader>
        <div className="grid grid-cols-2 gap-2 mt-1">
          {SOCIAL_PLATFORMS.map(({ key, label }) => {
            const Icon = PLATFORM_ICONS[key] || Globe;
            const configured =
              configuredSet.size === 0 || configuredSet.has(key);
            return (
              <button
                key={key}
                disabled={!configured}
                onClick={() => configured && handleConnect(key)}
                className={`flex items-center gap-3 p-3 rounded-lg border text-left transition-colors ${
                  configured
                    ? 'border-border hover:bg-muted/60 cursor-pointer'
                    : 'border-border/40 opacity-40 cursor-not-allowed'
                }`}
              >
                <div
                  className={`h-8 w-8 rounded-full flex items-center justify-center shrink-0 ${PLATFORM_BG[key] || 'bg-muted'}`}
                >
                  <Icon
                    className={`h-4 w-4 ${PLATFORM_COLORS[key] || 'text-muted-foreground'}`}
                  />
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium">{label}</p>
                  <p className="text-[10px] text-muted-foreground">
                    {configured ? 'Click to connect' : 'Not configured'}
                  </p>
                </div>
              </button>
            );
          })}
        </div>
      </DialogContent>
    </Dialog>
  );
}

function InlineSocialSection({ orgId }: { orgId: string }) {
  const [connectOpen, setConnectOpen] = useState(false);
  const queryClient = useQueryClient();

  const { data: accounts = [], isLoading } = useQuery({
    queryKey: socialKeys.accounts(orgId),
    queryFn: () => socialApi.listAccounts({ organizationId: orgId }),
    staleTime: 60_000,
  });

  const deleteMut = useMutation({
    mutationFn: (id: string) => socialApi.deleteAccount(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: socialKeys.accounts(orgId) }),
  });

  return (
    <section className="space-y-3">
      <SocialConnectDialog
        open={connectOpen}
        onClose={() => setConnectOpen(false)}
        orgId={orgId}
      />

      <div className="flex items-center justify-between">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Social Media
        </h3>
        <button
          onClick={() => setConnectOpen(true)}
          className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5"
        >
          <Plus className="h-3 w-3" />
          Connect Account
        </button>
      </div>

      {isLoading ? (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Loading…
        </div>
      ) : accounts.length === 0 ? (
        <div className="rounded-lg border border-dashed border-border/60 p-6 text-center text-muted-foreground">
          <Share2 className="h-6 w-6 mx-auto mb-2 opacity-30" />
          <p className="text-sm mb-3">
            No social accounts connected to this organization.
          </p>
          <button
            onClick={() => setConnectOpen(true)}
            className="h-7 px-3 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 mx-auto"
          >
            <Plus className="h-3 w-3" />
            Connect Account
          </button>
        </div>
      ) : (
        <div className="space-y-2">
          {accounts.map((account) => {
            const Icon = PLATFORM_ICONS[account.platform] || Globe;
            return (
              <IntegrationCard
                key={account.id}
                accent=""
                icon={Icon}
                name={
                  account.display_name || account.username || account.platform
                }
                description={`${account.platform} · ${account.follower_count?.toLocaleString() ?? 0} followers`}
                status={
                  account.status === 'active'
                    ? 'connected'
                    : account.status === 'error'
                      ? 'warning'
                      : 'disconnected'
                }
                actions={
                  <>
                    {account.profile_url && (
                      <a
                        href={account.profile_url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1"
                      >
                        <ExternalLink className="h-3 w-3" />
                        View
                      </a>
                    )}
                    <button
                      className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1 disabled:opacity-50"
                      onClick={() =>
                        confirm('Disconnect this account?') &&
                        deleteMut.mutate(account.id)
                      }
                      disabled={deleteMut.isPending}
                    >
                      <Trash2 className="h-3 w-3" />
                      Disconnect
                    </button>
                  </>
                }
              />
            );
          })}
          <button
            onClick={() => setConnectOpen(true)}
            className="w-full h-8 text-xs text-muted-foreground border border-dashed rounded-md hover:bg-muted/40 hover:text-foreground flex items-center justify-center gap-1.5 transition-colors"
          >
            <Plus className="h-3 w-3" />
            Add another account
          </button>
        </div>
      )}
    </section>
  );
}

// ─── Main tab ─────────────────────────────────────────────────────────────

function IntegrationsTab({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [connectingEmail, setConnectingEmail] = useState<string | null>(null);
  const [qbSyncing, setQbSyncing] = useState(false);
  const [qbDisconnecting, setQbDisconnecting] = useState(false);

  // ── Email accounts (Gmail / Zoho) ─────────────────────────────────────────
  const { data: emailAccounts = [], isLoading: emailLoading } = useQuery<
    EmailAccountRecord[]
  >({
    queryKey: integrationKeys.emailAccountsOrg(orgId),
    queryFn: () =>
      emailApi.listAccounts(undefined, undefined, 'organization', orgId),
    staleTime: 30_000,
  });

  const handleEmailConnect = async (provider: string) => {
    setConnectingEmail(provider);
    try {
      const redirectUri = `${window.location.origin}/oauth/${provider}/callback`;
      const { auth_url } = await emailApi.initiateOAuth(
        null,
        provider,
        redirectUri,
        'organization',
        orgId
      );
      window.location.href = auth_url;
    } catch {
      setConnectingEmail(null);
    }
  };

  const handleEmailDisconnect = async (id: string) => {
    if (!confirm('Disconnect this email account?')) return;
    await emailApi.deleteAccount(id);
    queryClient.invalidateQueries({
      queryKey: integrationKeys.emailAccountsOrg(orgId),
    });
  };

  const handleEmailSync = async (id: string) => {
    await emailApi.triggerSync(id);
    queryClient.invalidateQueries({
      queryKey: integrationKeys.emailAccountsOrg(orgId),
    });
  };

  // ── QuickBooks ────────────────────────────────────────────────────────────
  const {
    data: qbStatus,
    isLoading: qbLoading,
    refetch: refetchQb,
  } = useQuery({
    queryKey: integrationKeys.qbStatusOrg(orgId),
    queryFn: () => quickbooksApi.getStatus(orgId),
    staleTime: 30_000,
  });

  const handleQbConnect = () => {
    window.location.href = quickbooksApi.getConnectUrl(orgId);
  };

  const handleQbDisconnect = async () => {
    if (!qbStatus?.account?.id) return;
    if (!confirm('Disconnect QuickBooks? Entity mappings will be removed.'))
      return;
    setQbDisconnecting(true);
    try {
      await quickbooksApi.disconnect(qbStatus.account.id);
      refetchQb();
    } finally {
      setQbDisconnecting(false);
    }
  };

  const handleQbSync = async () => {
    if (!qbStatus?.account?.id) return;
    setQbSyncing(true);
    try {
      await quickbooksApi.triggerSync(qbStatus?.account.id);
      refetchQb();
    } finally {
      setQbSyncing(false);
    }
  };

  const handleQbRefresh = async () => {
    if (!qbStatus?.account?.id) return;
    try {
      await quickbooksApi.refreshToken(qbStatus.account.id);
      refetchQb();
    } catch {
      /* ignore */
    }
  };

  // ── Airtable ──────────────────────────────────────────────────────────────
  const { config, updateAndSaveConfig } = useUserSystem();
  const [airtableToken, setAirtableToken] = useState(
    config?.airtable?.token ?? ''
  );
  const [airtableVerifying, setAirtableVerifying] = useState(false);
  const [airtableError, setAirtableError] = useState<string | null>(null);
  const isAirtableConnected = !!config?.airtable?.token;

  const handleAirtableSave = async () => {
    if (!airtableToken) {
      setAirtableError('Enter your Personal Access Token');
      return;
    }
    setAirtableVerifying(true);
    setAirtableError(null);
    try {
      const result = await airtableApi.verifyCredentials({
        token: airtableToken,
      });
      if (result.valid) {
        await updateAndSaveConfig({
          airtable: {
            ...(config?.airtable ?? {}),
            token: airtableToken,
            user_email: result.user_email ?? null,
          } as never,
        });
      } else {
        setAirtableError('Token is invalid. Check permissions and try again.');
      }
    } catch (e: unknown) {
      setAirtableError(e instanceof Error ? e.message : 'Verification failed');
    } finally {
      setAirtableVerifying(false);
    }
  };

  const handleAirtableDisconnect = async () => {
    if (!confirm('Remove Airtable connection?')) return;
    await updateAndSaveConfig({
      airtable: {
        ...(config?.airtable ?? {}),
        token: '',
        user_email: null,
      } as never,
    });
    setAirtableToken('');
  };

  const qbConnected = !qbLoading && !!qbStatus?.connected;
  const qbNeedsReauth = !qbLoading && !!qbStatus?.needs_reauth;

  const emailSections: {
    provider: string;
    label: string;
    accent: string;
    desc: string;
  }[] = [
    {
      provider: 'gmail',
      label: 'Gmail',
      accent: '#EA4335',
      desc: 'Unified inbox, Nora email access, and contact sync.',
    },
    {
      provider: 'zoho',
      label: 'Zoho Mail',
      accent: '#C8202B',
      desc: 'Zoho Mail + CRM sync — operations and pipeline in lock-step.',
    },
  ];

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-lg font-semibold">Organization Integrations</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Org-level connections shared across all projects. Projects, users, and
          agents have their own independently configurable integrations.
        </p>
      </div>

      {/* ── Email ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Email
        </h3>
        {emailLoading && (
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading…
          </div>
        )}
        {emailSections.map(({ provider, label, accent, desc }) => {
          const account = emailAccounts.find((a) => a.provider === provider);
          const isConn = account?.status === 'active';
          const isWarn = account?.status === 'needs_reauth';
          return (
            <IntegrationCard
              key={provider}
              accent={accent}
              icon={Mail}
              name={label}
              description={account ? account.email_address : desc}
              status={
                isConn ? 'connected' : isWarn ? 'warning' : 'disconnected'
              }
              actions={
                account ? (
                  <>
                    <button
                      className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1"
                      onClick={() => handleEmailSync(account.id)}
                    >
                      <RefreshCw className="h-3 w-3" />
                      Sync
                    </button>
                    <button
                      className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1"
                      onClick={() => handleEmailDisconnect(account.id)}
                    >
                      <Trash2 className="h-3 w-3" />
                      Remove
                    </button>
                  </>
                ) : (
                  <button
                    className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                    disabled={connectingEmail === provider}
                    onClick={() => handleEmailConnect(provider)}
                  >
                    {connectingEmail === provider ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <Plug className="h-3 w-3" />
                    )}
                    Connect
                  </button>
                )
              }
              extra={
                account?.last_sync_at ? (
                  <p className="text-xs text-muted-foreground">
                    Last sync {new Date(account.last_sync_at).toLocaleString()}
                  </p>
                ) : undefined
              }
            />
          );
        })}
      </section>

      {/* ── Accounting ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Accounting & Finance
        </h3>
        <IntegrationCard
          accent="#2CA01C"
          icon={FileText}
          name="QuickBooks Online"
          description={
            qbStatus?.account?.company_name ??
            'Sync invoices, customers, payments, and expenses with QuickBooks.'
          }
          status={
            qbConnected
              ? 'connected'
              : qbNeedsReauth
                ? 'warning'
                : 'disconnected'
          }
          statusLabel={
            qbConnected
              ? qbStatus?.account?.company_name
                ? 'Connected'
                : 'Connected'
              : undefined
          }
          actions={
            qbLoading ? (
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
            ) : qbConnected ? (
              <>
                <button
                  className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1 disabled:opacity-50"
                  onClick={handleQbSync}
                  disabled={qbSyncing}
                >
                  {qbSyncing ? (
                    <Loader2 className="h-3 w-3 animate-spin" />
                  ) : (
                    <RefreshCw className="h-3 w-3" />
                  )}
                  Sync
                </button>
                <button
                  className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1 disabled:opacity-50"
                  onClick={handleQbDisconnect}
                  disabled={qbDisconnecting}
                >
                  <Trash2 className="h-3 w-3" />
                  Disconnect
                </button>
              </>
            ) : qbNeedsReauth ? (
              <button
                className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1"
                onClick={handleQbRefresh}
              >
                <RefreshCw className="h-3 w-3" />
                Reauthorize
              </button>
            ) : (
              <button
                className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5"
                onClick={handleQbConnect}
              >
                <Plug className="h-3 w-3" />
                Connect
              </button>
            )
          }
          extra={
            qbConnected && qbStatus?.account?.last_sync_at ? (
              <p className="text-xs text-muted-foreground">
                Last sync{' '}
                {new Date(qbStatus.account.last_sync_at).toLocaleString()}
              </p>
            ) : undefined
          }
        />
        <QbFinancialSummary orgId={orgId} />
      </section>

      {/* ── Productivity ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Productivity & Data
        </h3>

        {/* Airtable */}
        <IntegrationCard
          accent="#FF0000"
          icon={FileText}
          name="Airtable"
          description={
            isAirtableConnected && config?.airtable?.user_email
              ? config.airtable.user_email
              : 'Connect Airtable with a Personal Access Token to give Nora and agents access to your bases.'
          }
          status={isAirtableConnected ? 'connected' : 'disconnected'}
          actions={
            isAirtableConnected ? (
              <button
                className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1"
                onClick={handleAirtableDisconnect}
              >
                <Trash2 className="h-3 w-3" />
                Remove
              </button>
            ) : null
          }
          extra={
            !isAirtableConnected ? (
              <div className="space-y-2 pt-1">
                <div className="flex gap-2">
                  <input
                    type="password"
                    value={airtableToken}
                    onChange={(e) => setAirtableToken(e.target.value)}
                    placeholder="patXXXXXXXXXXXXXX"
                    className="flex-1 h-8 px-3 text-xs border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                  <button
                    className="h-8 px-3 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                    onClick={handleAirtableSave}
                    disabled={airtableVerifying || !airtableToken}
                  >
                    {airtableVerifying ? (
                      <Loader2 className="h-3 w-3 animate-spin" />
                    ) : (
                      <CheckCircle2 className="h-3 w-3" />
                    )}
                    Verify & Save
                  </button>
                </div>
                {airtableError && (
                  <p className="text-xs text-destructive">{airtableError}</p>
                )}
                <p className="text-xs text-muted-foreground">
                  Create a token at{' '}
                  <span className="text-primary">
                    airtable.com/create/tokens
                  </span>{' '}
                  with{' '}
                  <code className="bg-muted px-1 rounded">
                    data.records:read
                  </code>{' '}
                  +{' '}
                  <code className="bg-muted px-1 rounded">
                    schema.bases:read
                  </code>{' '}
                  scopes.
                </p>
              </div>
            ) : undefined
          }
        />

        {/* Storage providers (OneDrive + Dropbox) */}
        <StorageProviderCards orgId={orgId} />
      </section>

      {/* ── Social ── */}
      <InlineSocialSection orgId={orgId} />

      {/* ── Communication ── */}
      <CommunicationSection />

      {/* ── Commerce ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Commerce
        </h3>
        <IntegrationCard
          accent="#635BFF"
          icon={DollarSign}
          name="Stripe"
          description="Sync payments, subscriptions, and invoices. Enable Stripe billing for clients."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <button
              disabled
              title="Coming soon"
              className="h-7 px-2.5 text-xs border rounded-md opacity-40 cursor-not-allowed flex items-center gap-1.5"
            >
              <Plug className="h-3 w-3" />
              Connect
            </button>
          }
        />
        <IntegrationCard
          accent="#96BF48"
          icon={Boxes}
          name="Shopify"
          description="Connect your Shopify store for order and product data access by agents."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <button
              disabled
              title="Coming soon"
              className="h-7 px-2.5 text-xs border rounded-md opacity-40 cursor-not-allowed flex items-center gap-1.5"
            >
              <Plug className="h-3 w-3" />
              Connect
            </button>
          }
        />
        <IntegrationCard
          accent="#7C3AED"
          icon={DollarSign}
          name="VIBE Wallet"
          description="Manage on-chain VIBE token balances and project funding via the Aptos network."
          status="connected"
          statusLabel="Active"
          actions={
            <Link
              to="/settings/wallet"
              className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5"
            >
              <ExternalLink className="h-3 w-3" />
              Wallet Settings
            </Link>
          }
        />
      </section>

      {/* ── Development ── */}
      <DevelopmentSection />
    </div>
  );
}

export default IntegrationsTab;
