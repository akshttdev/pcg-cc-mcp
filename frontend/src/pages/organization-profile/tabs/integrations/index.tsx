import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import {
  Mail,
  FileText,
  DollarSign,
  Boxes,
  Plug,
  Loader2,
  RefreshCw,
  Trash2,
  CheckCircle2,
  ExternalLink,
} from 'lucide-react';
import { IntegrationCard } from '../../components/IntegrationCard';
import { SocialSection } from './SocialSection';
import { CommunicationSection } from './CommunicationSection';
import { DevelopmentSection } from './DevelopmentSection';
import {
  emailApi,
  quickbooksApi,
  airtableApi,
  type EmailAccountRecord,
} from '@/lib/api';
import { useUserSystem } from '@/components/config-provider';
import { integrationKeys } from '@/lib/query-keys';

function IntegrationsTab({ orgId }: { orgId: string }) {
  const queryClient = useQueryClient();
  const [connectingEmail, setConnectingEmail] = useState<string | null>(null);
  const [qbSyncing, setQbSyncing] = useState(false);
  const [qbDisconnecting, setQbDisconnecting] = useState(false);

  // ── Email accounts (Gmail / Zoho) ─────────────────────────────────────────
  const { data: emailAccounts = [], isLoading: emailLoading } = useQuery<EmailAccountRecord[]>({
    queryKey: integrationKeys.emailAccountsOrg(orgId),
    queryFn: () => emailApi.listAccounts(undefined, undefined, 'organization', orgId),
    staleTime: 30_000,
  });

  const handleEmailConnect = async (provider: string) => {
    setConnectingEmail(provider);
    try {
      const redirectUri = `${window.location.origin}/oauth/${provider}/callback`;
      const { auth_url } = await emailApi.initiateOAuth(null, provider, redirectUri, 'organization', orgId);
      window.location.href = auth_url;
    } catch {
      setConnectingEmail(null);
    }
  };

  const handleEmailDisconnect = async (id: string) => {
    if (!confirm('Disconnect this email account?')) return;
    await emailApi.deleteAccount(id);
    queryClient.invalidateQueries({ queryKey: integrationKeys.emailAccountsOrg(orgId) });
  };

  const handleEmailSync = async (id: string) => {
    await emailApi.triggerSync(id);
    queryClient.invalidateQueries({ queryKey: integrationKeys.emailAccountsOrg(orgId) });
  };

  // ── QuickBooks ────────────────────────────────────────────────────────────
  const { data: qbStatus, isLoading: qbLoading, refetch: refetchQb } = useQuery({
    queryKey: integrationKeys.qbStatusOrg(orgId),
    queryFn: () => quickbooksApi.getStatus(orgId),
    staleTime: 30_000,
  });

  const handleQbConnect = () => { window.location.href = quickbooksApi.getConnectUrl(orgId); };

  const handleQbDisconnect = async () => {
    if (!qbStatus?.account?.id) return;
    if (!confirm('Disconnect QuickBooks? Entity mappings will be removed.')) return;
    setQbDisconnecting(true);
    try { await quickbooksApi.disconnect(qbStatus.account.id); refetchQb(); }
    finally { setQbDisconnecting(false); }
  };

  const handleQbSync = async () => {
    if (!qbStatus?.account?.id) return;
    setQbSyncing(true);
    try { await quickbooksApi.triggerSync(qbStatus?.account.id); refetchQb(); }
    finally { setQbSyncing(false); }
  };

  const handleQbRefresh = async () => {
    if (!qbStatus?.account?.id) return;
    try { await quickbooksApi.refreshToken(qbStatus.account.id); refetchQb(); }
    catch { /* ignore */ }
  };

  // ── Airtable ──────────────────────────────────────────────────────────────
  const { config, updateAndSaveConfig } = useUserSystem();
  const [airtableToken, setAirtableToken] = useState(config?.airtable?.token ?? '');
  const [airtableVerifying, setAirtableVerifying] = useState(false);
  const [airtableError, setAirtableError] = useState<string | null>(null);
  const isAirtableConnected = !!(config?.airtable?.token);

  const handleAirtableSave = async () => {
    if (!airtableToken) { setAirtableError('Enter your Personal Access Token'); return; }
    setAirtableVerifying(true);
    setAirtableError(null);
    try {
      const result = await airtableApi.verifyCredentials({ token: airtableToken });
      if (result.valid) {
        await updateAndSaveConfig({ airtable: { ...(config?.airtable ?? {}), token: airtableToken, user_email: result.user_email ?? null } as never });
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
    await updateAndSaveConfig({ airtable: { ...(config?.airtable ?? {}), token: '', user_email: null } as never });
    setAirtableToken('');
  };

  const qbConnected = !qbLoading && !!qbStatus?.connected;
  const qbNeedsReauth = !qbLoading && !!qbStatus?.needs_reauth;

  const emailSections: { provider: string; label: string; accent: string; desc: string }[] = [
    { provider: 'gmail',  label: 'Gmail',      accent: '#EA4335', desc: 'Unified inbox, Nora email access, and contact sync.' },
    { provider: 'zoho',   label: 'Zoho Mail',  accent: '#C8202B', desc: 'Zoho Mail + CRM sync — operations and pipeline in lock-step.' },
  ];

  return (
    <div className="space-y-8">
      <div>
        <h2 className="text-lg font-semibold">Organization Integrations</h2>
        <p className="text-sm text-muted-foreground mt-1">
          Org-level connections shared across all projects. Projects, users, and agents have their own independently configurable integrations.
        </p>
      </div>

      {/* ── Email ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Email</h3>
        {emailLoading && <div className="flex items-center gap-2 text-sm text-muted-foreground"><Loader2 className="h-4 w-4 animate-spin" />Loading…</div>}
        {emailSections.map(({ provider, label, accent, desc }) => {
          const account = emailAccounts.find(a => a.provider === provider);
          const isConn = account?.status === 'active';
          const isWarn = account?.status === 'needs_reauth';
          return (
            <IntegrationCard
              key={provider}
              accent={accent}
              icon={Mail}
              name={label}
              description={account ? account.email_address : desc}
              status={isConn ? 'connected' : isWarn ? 'warning' : 'disconnected'}
              actions={
                account ? (
                  <>
                    <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1" onClick={() => handleEmailSync(account.id)}>
                      <RefreshCw className="h-3 w-3" />Sync
                    </button>
                    <button className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1" onClick={() => handleEmailDisconnect(account.id)}>
                      <Trash2 className="h-3 w-3" />Remove
                    </button>
                  </>
                ) : (
                  <button
                    className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                    disabled={connectingEmail === provider}
                    onClick={() => handleEmailConnect(provider)}
                  >
                    {connectingEmail === provider ? <Loader2 className="h-3 w-3 animate-spin" /> : <Plug className="h-3 w-3" />}
                    Connect
                  </button>
                )
              }
              extra={account?.last_sync_at ? <p className="text-xs text-muted-foreground">Last sync {new Date(account.last_sync_at).toLocaleString()}</p> : undefined}
            />
          );
        })}
      </section>

      {/* ── Accounting ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Accounting & Finance</h3>
        <IntegrationCard
          accent="#2CA01C"
          icon={FileText}
          name="QuickBooks Online"
          description={qbStatus?.account?.company_name ?? 'Sync invoices, customers, payments, and expenses with QuickBooks.'}
          status={qbConnected ? 'connected' : qbNeedsReauth ? 'warning' : 'disconnected'}
          statusLabel={qbConnected ? qbStatus?.account?.company_name ? 'Connected' : 'Connected' : undefined}
          actions={
            qbLoading ? <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" /> :
            qbConnected ? (
              <>
                <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1 disabled:opacity-50" onClick={handleQbSync} disabled={qbSyncing}>
                  {qbSyncing ? <Loader2 className="h-3 w-3 animate-spin" /> : <RefreshCw className="h-3 w-3" />}Sync
                </button>
                <button className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1 disabled:opacity-50" onClick={handleQbDisconnect} disabled={qbDisconnecting}>
                  <Trash2 className="h-3 w-3" />Disconnect
                </button>
              </>
            ) : qbNeedsReauth ? (
              <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1" onClick={handleQbRefresh}>
                <RefreshCw className="h-3 w-3" />Reauthorize
              </button>
            ) : (
              <button className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5" onClick={handleQbConnect}>
                <Plug className="h-3 w-3" />Connect
              </button>
            )
          }
          extra={qbConnected && qbStatus?.account?.last_sync_at
            ? <p className="text-xs text-muted-foreground">Last sync {new Date(qbStatus.account.last_sync_at).toLocaleString()}</p>
            : undefined
          }
        />
      </section>

      {/* ── Productivity ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Productivity & Data</h3>

        {/* Airtable */}
        <IntegrationCard
          accent="#FF0000"
          icon={FileText}
          name="Airtable"
          description={isAirtableConnected && config?.airtable?.user_email ? config.airtable.user_email : 'Connect Airtable with a Personal Access Token to give Nora and agents access to your bases.'}
          status={isAirtableConnected ? 'connected' : 'disconnected'}
          actions={
            isAirtableConnected ? (
              <button className="h-7 px-2.5 text-xs text-muted-foreground hover:text-destructive flex items-center gap-1" onClick={handleAirtableDisconnect}>
                <Trash2 className="h-3 w-3" />Remove
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
                    onChange={e => setAirtableToken(e.target.value)}
                    placeholder="patXXXXXXXXXXXXXX"
                    className="flex-1 h-8 px-3 text-xs border rounded-md bg-background focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                  <button
                    className="h-8 px-3 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5 disabled:opacity-50"
                    onClick={handleAirtableSave}
                    disabled={airtableVerifying || !airtableToken}
                  >
                    {airtableVerifying ? <Loader2 className="h-3 w-3 animate-spin" /> : <CheckCircle2 className="h-3 w-3" />}
                    Verify & Save
                  </button>
                </div>
                {airtableError && <p className="text-xs text-destructive">{airtableError}</p>}
                <p className="text-xs text-muted-foreground">
                  Create a token at <span className="text-primary">airtable.com/create/tokens</span> with <code className="bg-muted px-1 rounded">data.records:read</code> + <code className="bg-muted px-1 rounded">schema.bases:read</code> scopes.
                </p>
              </div>
            ) : undefined
          }
        />

        {/* Dropbox */}
        <IntegrationCard
          accent="#0061FF"
          icon={FileText}
          name="Dropbox"
          description="Connect Dropbox folders as knowledge sources and asset storage for projects."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <span className="text-xs text-muted-foreground italic">Configure per-project</span>
          }
        />

        {/* OneDrive */}
        <IntegrationCard
          accent="#0078D4"
          icon={FileText}
          name="OneDrive"
          description="Access Microsoft OneDrive files as knowledge sources and shared asset storage across projects."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <span className="text-xs text-muted-foreground italic">Not yet configured</span>
          }
        />

        {/* GitHub */}
        <IntegrationCard
          accent="#24292e"
          icon={FileText}
          name="GitHub"
          description="Link repositories to projects. Agents can read code, create PRs, and browse issues."
          status="disconnected"
          statusLabel="Coming soon"
          actions={
            <span className="text-xs text-muted-foreground italic">Configure per-project</span>
          }
        />
      </section>

      {/* ── Social ── */}
      <SocialSection />

      {/* ── Communication ── */}
      <CommunicationSection />

      {/* ── Commerce ── */}
      <section className="space-y-3">
        <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Commerce</h3>
        <IntegrationCard
          accent="#635BFF"
          icon={DollarSign}
          name="Stripe"
          description="Sync payments, subscriptions, and invoices. Enable Stripe billing for clients."
          status="disconnected"
          statusLabel="Coming soon"
          actions={<span className="text-xs text-muted-foreground italic">Not yet configured</span>}
        />
        <IntegrationCard
          accent="#96BF48"
          icon={Boxes}
          name="Shopify"
          description="Connect your Shopify store for order and product data access by agents."
          status="disconnected"
          statusLabel="Coming soon"
          actions={<span className="text-xs text-muted-foreground italic">Not yet configured</span>}
        />
        <IntegrationCard
          accent="#7C3AED"
          icon={DollarSign}
          name="VIBE Wallet"
          description="Manage on-chain VIBE token balances and project funding via the Aptos network."
          status="connected"
          statusLabel="Active"
          actions={
            <Link to="/settings/wallet" className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5">
              <ExternalLink className="h-3 w-3" />Wallet Settings
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
