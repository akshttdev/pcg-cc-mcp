import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { FormField } from '@/components/ui/form-field';
import { StatusBadge } from '@/components/ui/status-badge';
import { CardGrid } from '@/components/ui/card-grid';
import { EmptyState } from '@/components/ui/empty-state';
import { Loader2, Plus, Wallet, Search, X, SortAsc, SortDesc, Mail, MessageSquare } from 'lucide-react';
import { emailApi } from '@/lib/api';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { Skeleton } from '@/components/ui/skeleton';
import { useQuery } from '@tanstack/react-query';
import { agentWalletApi, agentsApi, type AgentSearchParams } from '@/lib/api';
import type {
  AgentWallet,
  AgentWithParsedFields,
  UpsertAgentWallet,
  AgentStatus,
} from 'shared/types';
import { toast } from 'sonner';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { agentKeys } from '@/lib/query-keys';

import { useProfiles } from '@/hooks/useProfiles';
import { AgentDetailDialog } from '@/components/dialogs/agent-detail-dialog';

// Status options for filter
const STATUS_OPTIONS: { value: AgentStatus | 'all'; label: string }[] = [
  { value: 'all', label: 'All Statuses' },
  { value: 'active', label: 'Active' },
  { value: 'inactive', label: 'Inactive' },
  { value: 'maintenance', label: 'Maintenance' },
  { value: 'training', label: 'Training' },
];

// Sort options
const SORT_OPTIONS = [
  { value: 'name', label: 'Name' },
  { value: 'designation', label: 'Designation' },
  { value: 'status', label: 'Status' },
  { value: 'priority', label: 'Priority' },
  { value: 'tasks_completed', label: 'Tasks Completed' },
] as const;

// Nora's agent UUID — stable, set at DB seed time
const NORA_AGENT_ID = '0907dc4f3f7f4c4093cff36a833eaa78';

function NoraCommunicationChannels() {
  const [connecting, setConnecting] = useState<string | null>(null);
  const [emailStatus, setEmailStatus] = useState<'unknown' | 'connected' | 'error'>('unknown');

  const handleConnectEmail = async () => {
    try {
      setConnecting('email');
      const result = await emailApi.initiateOAuth(
        null,
        'zoho',
        `${window.location.origin}/oauth/zoho/callback`,
        'agent',
        NORA_AGENT_ID,
      );
      window.location.href = result.auth_url;
    } catch (err) {
      console.error('Failed to initiate Nora email OAuth:', err);
      setEmailStatus('error');
    } finally {
      setConnecting(null);
    }
  };

  return (
    <div className="space-y-3">
      {/* Email */}
      <div className="flex items-center justify-between rounded-lg border p-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-[#C8202B] flex items-center justify-center text-white font-semibold">
            Z
          </div>
          <div>
            <p className="font-medium">nora@powerclubglobal.com</p>
            <p className="text-sm text-muted-foreground">Zoho Mail — Nora's email identity</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {emailStatus === 'connected' && (
            <StatusBadge status="success" label="Connected" />
          )}
          {emailStatus === 'error' && (
            <StatusBadge status="error" label="Error" />
          )}
          <Button
            size="sm"
            variant="outline"
            onClick={handleConnectEmail}
            disabled={connecting === 'email'}
          >
            {connecting === 'email' ? (
              <><Loader2 className="h-3 w-3 mr-1 animate-spin" />Connecting…</>
            ) : (
              <><Mail className="h-3 w-3 mr-1" />Connect / Reconnect</>
            )}
          </Button>
        </div>
      </div>

      {/* SMS — informational (auto-configured via env) */}
      <div className="flex items-center justify-between rounded-lg border p-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-[#F22F46] flex items-center justify-center text-white">
            <MessageSquare className="h-5 w-5" />
          </div>
          <div>
            <p className="font-medium">+14053008311</p>
            <p className="text-sm text-muted-foreground">Twilio SMS — Nora's phone identity</p>
          </div>
        </div>
        <StatusBadge status="success" label="Active" />
      </div>
    </div>
  );
}

export function AgentSettings() {
  const { t } = useTranslation('settings');
  // Use profiles hook to get executor profiles for wallet profile options
  const {
    profilesContent: serverProfilesContent,
    isLoading: profilesLoading,
    error: profilesError,
  } = useProfiles();

  // Parsed profiles for wallet profile options
  const [localParsedProfiles, setLocalParsedProfiles] = useState<any>(null);

  const {
    data: agentWallets = [],
    isLoading: walletsLoading,
    isFetching: walletsFetching,
  } = useQuery({
    queryKey: agentKeys.wallets(),
    queryFn: agentWalletApi.list,
  });

  // Search and filter state
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<AgentStatus | 'all'>('all');
  const [sortBy, setSortBy] = useState<AgentSearchParams['sort_by']>('name');
  const [sortDir, setSortDir] = useState<'asc' | 'desc'>('asc');

  // Build search params
  const searchParams = useMemo((): AgentSearchParams => {
    const params: AgentSearchParams = {};
    if (searchQuery.trim()) params.q = searchQuery.trim();
    if (statusFilter !== 'all') params.status = statusFilter;
    params.sort_by = sortBy;
    params.sort_dir = sortDir;
    return params;
  }, [searchQuery, statusFilter, sortBy, sortDir]);

  const hasFilters = searchQuery.trim() || statusFilter !== 'all';

  const {
    data: agentDirectory = [],
    isLoading: agentsLoading,
    error: agentsError,
  } = useQuery<AgentWithParsedFields[], Error>({
    queryKey: ['agents', 'search', searchParams],
    queryFn: () => agentsApi.search(searchParams),
  });

  // Agent detail dialog state
  const [selectedAgent, setSelectedAgent] = useState<AgentWithParsedFields | null>(null);
  const [detailDialogOpen, setDetailDialogOpen] = useState(false);

  // Clear all filters
  const clearFilters = useCallback(() => {
    setSearchQuery('');
    setStatusFilter('all');
    setSortBy('name');
    setSortDir('asc');
  }, []);

  const handleAgentClick = useCallback((agent: AgentWithParsedFields) => {
    setSelectedAgent(agent);
    setDetailDialogOpen(true);
  }, []);

  const [budgetModalOpen, setBudgetModalOpen] = useState(false);
  const [budgetProfileKey, setBudgetProfileKey] = useState<string>('');
  const [budgetDisplayName, setBudgetDisplayName] = useState('');
  const [budgetValue, setBudgetValue] = useState('');
  const [isNewBudget, setIsNewBudget] = useState(false);
  const [budgetError, setBudgetError] = useState<string | null>(null);

  const extractWalletError = (error: unknown): string =>
    error instanceof Error ? error.message : 'Failed to save wallet budget';

  const upsertWalletMutation = useMutationWithToast({
    mutationFn: agentWalletApi.upsert,
    successMessage: 'Wallet budget saved',
    errorMessage: extractWalletError,
    invalidateKeys: [agentKeys.wallets()],
    onSuccess: () => {
      setBudgetModalOpen(false);
    },
    onError: (error: unknown) => {
      setBudgetError(extractWalletError(error));
    },
  });

  const walletBusy = upsertWalletMutation.isPending;

  // Parse profiles for wallet profile options
  useEffect(() => {
    if (serverProfilesContent) {
      try {
        const parsed = JSON.parse(serverProfilesContent);
        setLocalParsedProfiles(parsed);
      } catch (err) {
        console.error('Failed to parse profiles JSON:', err);
        setLocalParsedProfiles(null);
      }
    }
  }, [serverProfilesContent]);

  useEffect(() => {
    if (!budgetModalOpen) {
      setBudgetProfileKey('');
      setBudgetDisplayName('');
      setBudgetValue('');
      setIsNewBudget(false);
      setBudgetError(null);
    }
  }, [budgetModalOpen]);

  const numberFormatter = useMemo(() => new Intl.NumberFormat(), []);

  const walletMap = useMemo(() => {
    const map = new Map<string, AgentWallet>();
    agentWallets.forEach((wallet) => {
      map.set(wallet.profile_key, wallet);
    });
    return map;
  }, [agentWallets]);

  const sortedWallets = useMemo(
    () => [...agentWallets].sort((a, b) => a.profile_key.localeCompare(b.profile_key)),
    [agentWallets]
  );

  const profileOptions = useMemo(() => {
    if (!localParsedProfiles?.executors) {
      return [] as Array<{ profileKey: string; label: string }>;
    }

    const entries: Array<{ profileKey: string; label: string }> = [];
    const executors =
      localParsedProfiles.executors as Record<string, Record<string, unknown>>;
    Object.entries(executors).forEach(([executorType, configs]) => {
      Object.keys(configs || {}).forEach((configName) => {
        const profileKey =
          configName === 'DEFAULT'
            ? executorType
            : `${executorType}:${configName}`;
        const label =
          configName === 'DEFAULT'
            ? executorType
            : `${executorType} · ${configName}`;
        entries.push({ profileKey, label });
      });
    });
    return entries;
  }, [localParsedProfiles]);

  const availableProfiles = useMemo(
    () => profileOptions.filter((option) => !walletMap.has(option.profileKey)),
    [profileOptions, walletMap]
  );

  const currentWallet = budgetProfileKey
    ? walletMap.get(budgetProfileKey) ?? null
    : null;

  const currentProfileOption = useMemo(
    () =>
      profileOptions.find((option) => option.profileKey === budgetProfileKey) || null,
    [profileOptions, budgetProfileKey]
  );

  const formatAmount = useCallback(
    (value: number) => numberFormatter.format(value),
    [numberFormatter]
  );

  // Format VIBE amount with USD equivalent
  const formatVibeAmount = useCallback(
    (vibe: number) => {
      const usdValue = vibe * 0.01; // 1 VIBE = $0.01
      return (
        <span title={`$${usdValue.toFixed(4)} USD`}>
          {numberFormatter.format(vibe)}
        </span>
      );
    },
    [numberFormatter]
  );

  const handleOpenBudgetModal = useCallback(
    (profileKey: string, createNew: boolean) => {
      const wallet = walletMap.get(profileKey) ?? null;
      const option = profileOptions.find((opt) => opt.profileKey === profileKey);
      setBudgetProfileKey(profileKey);
      setBudgetDisplayName(
        wallet?.display_name || option?.label || profileKey
      );
      setBudgetValue(wallet ? String(wallet.budget_limit) : '');
      setIsNewBudget(createNew || !wallet);
      setBudgetError(null);
      setBudgetModalOpen(true);
    },
    [profileOptions, walletMap]
  );

  const handleAddBudget = useCallback(() => {
    if (!availableProfiles.length) {
      toast.info('All agent profiles already have budgets configured.');
      return;
    }
    handleOpenBudgetModal(availableProfiles[0].profileKey, true);
  }, [availableProfiles, handleOpenBudgetModal]);

  const handleManageBudget = useCallback(
    (profileKey: string) => {
      handleOpenBudgetModal(profileKey, false);
    },
    [handleOpenBudgetModal]
  );

  const handleBudgetSave = useCallback(() => {
    if (!budgetProfileKey) {
      setBudgetError('Select an agent profile.');
      return;
    }

    const parsed = Number(budgetValue);
    if (!Number.isFinite(parsed) || parsed < 0) {
      setBudgetError('Budget must be a non-negative number.');
      return;
    }

    setBudgetError(null);

    upsertWalletMutation.mutate({
      profile_key: budgetProfileKey,
      display_name:
        budgetDisplayName.trim() || currentProfileOption?.label || budgetProfileKey,
      budget_limit: Math.round(parsed),
    } as UpsertAgentWallet);
  }, [
    budgetDisplayName,
    budgetProfileKey,
    budgetValue,
    currentProfileOption,
    upsertWalletMutation,
  ]);

  const agentStatusToVariant: Record<string, 'success' | 'muted' | 'warning' | 'info'> = {
    active: 'success',
    inactive: 'muted',
    maintenance: 'warning',
    training: 'info',
  };

  if (profilesLoading) {
    return (
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <Skeleton className="h-6 w-48 mb-2" />
            <Skeleton className="h-4 w-72" />
          </CardHeader>
          <CardContent>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3, 4, 5, 6].map((i) => (
                <div key={i} className="rounded-xl border bg-card overflow-hidden">
                  <Skeleton className="aspect-square w-full" />
                  <div className="p-4 space-y-2">
                    <Skeleton className="h-5 w-32" />
                    <Skeleton className="h-4 w-24" />
                    <Skeleton className="h-3 w-full" />
                    <Skeleton className="h-3 w-3/4" />
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <>
      <div className="space-y-6">
      {!!profilesError && (
        <Alert variant="destructive">
          <AlertDescription>
            {profilesError instanceof Error
              ? profilesError.message
              : String(profilesError)}
          </AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <div className="flex flex-col gap-4">
            <div>
              <CardTitle>Autonomous Agents</CardTitle>
              <CardDescription>
                Live Directory of all Powerclub Global Agents
              </CardDescription>
            </div>

            {/* Search and Filter Controls */}
            <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
              {/* Search Input */}
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder="Search agents by name, role, or description..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 pr-9"
                />
                {searchQuery && (
                  <Button
                    variant="ghost"
                    size="sm"
                    className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 p-0"
                    onClick={() => setSearchQuery('')}
                  >
                    <X className="h-4 w-4" />
                  </Button>
                )}
              </div>

              {/* Status Filter */}
              <Select
                value={statusFilter}
                onValueChange={(v) => setStatusFilter(v as AgentStatus | 'all')}
              >
                <SelectTrigger className="w-[150px]">
                  <SelectValue placeholder="Status" />
                </SelectTrigger>
                <SelectContent>
                  {STATUS_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              {/* Sort Controls */}
              <Select value={sortBy} onValueChange={(v) => setSortBy(v as AgentSearchParams['sort_by'])}>
                <SelectTrigger className="w-[150px]">
                  <SelectValue placeholder="Sort by" />
                </SelectTrigger>
                <SelectContent>
                  {SORT_OPTIONS.map((opt) => (
                    <SelectItem key={opt.value} value={opt.value}>
                      {opt.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>

              <Button
                variant="outline"
                size="icon"
                onClick={() => setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'))}
                title={sortDir === 'asc' ? 'Sort ascending' : 'Sort descending'}
              >
                {sortDir === 'asc' ? <SortAsc className="h-4 w-4" /> : <SortDesc className="h-4 w-4" />}
              </Button>

              {/* Clear Filters */}
              {hasFilters && (
                <Button variant="ghost" size="sm" onClick={clearFilters}>
                  Clear filters
                </Button>
              )}
            </div>

            {/* Results count */}
            {!agentsLoading && (
              <p className="text-sm text-muted-foreground">
                {agentDirectory.length} agent{agentDirectory.length !== 1 ? 's' : ''} found
                {hasFilters && ' (filtered)'}
              </p>
            )}
          </div>
        </CardHeader>
        <CardContent>
          {agentsLoading ? (
            <Skeleton className="h-40 w-full" />
          ) : agentsError ? (
            <Alert variant="destructive">
              <AlertDescription>
                {agentsError instanceof Error
                  ? agentsError.message
                  : 'Unable to load agent directory.'}
              </AlertDescription>
            </Alert>
          ) : agentDirectory.length === 0 ? (
            <EmptyState
              title="No registered agents"
              description="Seed the registry to expose Nora’s team."
              className="py-8"
            />
          ) : (
            <CardGrid columns={{ sm: 2, lg: 3 }} gap={4}>
              {agentDirectory.map((agent) => {
                const initials = agent.short_name
                  .split(' ')
                  .map((part) => part[0])
                  .join('')
                  .slice(0, 2)
                  .toUpperCase();
                return (
                  <div
                    key={agent.id}
                    onClick={() => handleAgentClick(agent)}
                    className="group relative rounded-xl border bg-card overflow-hidden transition-all hover:shadow-md hover:border-primary/20 cursor-pointer"
                  >
                    {/* Status indicator */}
                    <div className="absolute top-3 right-3 z-10">
                      <StatusBadge
                        status={agentStatusToVariant[agent.status] || 'muted'}
                        label={agent.status}
                        className="capitalize backdrop-blur-sm"
                      />
                    </div>

                    {/* Agent image */}
                    <div className="aspect-square w-full bg-muted relative overflow-hidden">
                      {agent.avatar_url ? (
                        <img
                          src={agent.avatar_url}
                          alt={agent.short_name}
                          className="h-full w-full object-cover transition-transform group-hover:scale-105"
                        />
                      ) : (
                        <div className="h-full w-full flex items-center justify-center bg-gradient-to-br from-primary/10 to-primary/5">
                          <span className="text-4xl font-semibold text-primary/40">
                            {initials}
                          </span>
                        </div>
                      )}
                    </div>

                    {/* Agent info */}
                    <div className="p-4 space-y-2">
                      <div>
                        <h3 className="font-semibold text-lg leading-tight">
                          {agent.short_name}
                        </h3>
                        <p className="text-sm font-medium text-primary/80">
                          {agent.designation || 'Specialist Agent'}
                        </p>
                      </div>

                      {agent.description && (
                        <p className="text-sm text-muted-foreground line-clamp-3">
                          {agent.description}
                        </p>
                      )}
                    </div>
                  </div>
                );
              })}
            </CardGrid>
          )}
        </CardContent>
      </Card>

      {/* ── Nora Channel Integrations ─────────────────────────────────────── */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mail className="h-5 w-5" />
            Nora Channel Integrations
          </CardTitle>
          <CardDescription>
            Connect Nora's own communication channels — email and SMS identity.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <NoraCommunicationChannels />
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Wallet className="h-5 w-5" />
              {t('settings.wallet.title', { defaultValue: 'Team Wallet' })}
            </CardTitle>
            <CardDescription>
              Monitor per-agent budgets and throttle high-cost workloads.
            </CardDescription>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleAddBudget}
            disabled={!availableProfiles.length || walletBusy}
          >
            <Plus className="mr-1 h-4 w-4" />
            {availableProfiles.length
              ? 'Add Budget'
              : 'All profiles configured'}
          </Button>
        </CardHeader>
        <CardContent>
          {walletsLoading && !walletsFetching ? (
            <div className="flex items-center justify-center py-8 text-muted-foreground">
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              <span>Loading agent wallets…</span>
            </div>
          ) : sortedWallets.length === 0 ? (
            <EmptyState
              icon={Wallet}
              title="No agent budgets"
              description="Create one to cap spending for a profile."
              action={availableProfiles.length ? { label: 'Add Budget', onClick: handleAddBudget } : undefined}
              className="py-8"
            />
          ) : (
            <div className="overflow-x-auto">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Profile</TableHead>
                    <TableHead>Display name</TableHead>
                    <TableHead className="text-right">APT Budget</TableHead>
                    <TableHead className="text-right">APT Spent</TableHead>
                    <TableHead className="text-right">APT Available</TableHead>
                    <TableHead className="text-right text-primary">VIBE Budget</TableHead>
                    <TableHead className="text-right text-primary">VIBE Spent</TableHead>
                    <TableHead className="text-right text-primary">VIBE Available</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sortedWallets.map((wallet) => {
                    const option = profileOptions.find(
                      (opt) => opt.profileKey === wallet.profile_key
                    );
                    const available = wallet.budget_limit - wallet.spent_amount;
                    const vibeAvailable = wallet.vibe_budget_limit != null
                      ? wallet.vibe_budget_limit - wallet.vibe_spent_amount
                      : null;
                    return (
                      <TableRow key={wallet.id}>
                        <TableCell className="font-mono text-xs sm:text-sm">
                          {wallet.profile_key}
                        </TableCell>
                        <TableCell className="text-sm">
                          {wallet.display_name || option?.label || '—'}
                        </TableCell>
                        <TableCell className="text-right font-medium">
                          {formatAmount(wallet.budget_limit)}
                        </TableCell>
                        <TableCell className="text-right text-muted-foreground">
                          {formatAmount(wallet.spent_amount)}
                        </TableCell>
                        <TableCell
                          className={
                            available <= 0
                              ? 'text-right text-destructive'
                              : 'text-right'
                          }
                        >
                          {formatAmount(available)}
                        </TableCell>
                        <TableCell className="text-right font-medium text-primary">
                          {wallet.vibe_budget_limit != null
                            ? formatVibeAmount(wallet.vibe_budget_limit)
                            : <span className="text-muted-foreground">∞</span>}
                        </TableCell>
                        <TableCell className="text-right text-muted-foreground">
                          {formatVibeAmount(wallet.vibe_spent_amount)}
                        </TableCell>
                        <TableCell
                          className={
                            vibeAvailable != null && vibeAvailable <= 0
                              ? 'text-right text-destructive'
                              : 'text-right text-primary'
                          }
                        >
                          {vibeAvailable != null
                            ? formatVibeAmount(vibeAvailable)
                            : <span className="text-muted-foreground">∞</span>}
                        </TableCell>
                        <TableCell className="text-right">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => handleManageBudget(wallet.profile_key)}
                            disabled={walletBusy}
                          >
                            Manage
                          </Button>
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}
        </CardContent>
      </Card>

      </div>

      <Dialog open={budgetModalOpen} onOpenChange={setBudgetModalOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle>
              {isNewBudget ? 'Create budget' : 'Manage budget'}
            </DialogTitle>
            <DialogDescription>
              {isNewBudget
                ? 'Set a budget cap for this agent profile.'
                : 'Adjust the spending limit for this agent profile.'}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <FormField label="Agent profile" htmlFor="budget-profile">
              <Select
                value={budgetProfileKey}
                onValueChange={setBudgetProfileKey}
                disabled={!isNewBudget || walletBusy}
              >
                <SelectTrigger id="budget-profile">
                  <SelectValue placeholder="Select a profile" />
                </SelectTrigger>
                <SelectContent>
                  {(isNewBudget ? availableProfiles : profileOptions).map(
                    (option) => (
                      <SelectItem
                        key={option.profileKey}
                        value={option.profileKey}
                      >
                        {option.label}
                      </SelectItem>
                    )
                  )}
                  {!isNewBudget && currentProfileOption && (
                    <SelectItem value={currentProfileOption.profileKey}>
                      {currentProfileOption.label}
                    </SelectItem>
                  )}
                </SelectContent>
              </Select>
            </FormField>

            <FormField label="Display name" htmlFor="budget-display-name">
              <Input
                id="budget-display-name"
                value={budgetDisplayName}
                onChange={(event) => setBudgetDisplayName(event.target.value)}
                disabled={walletBusy}
                placeholder={currentProfileOption?.label || 'Agent display name'}
              />
            </FormField>

            <FormField label="Monthly budget (credits)" htmlFor="budget-limit">
              <Input
                id="budget-limit"
                type="number"
                min={0}
                value={budgetValue}
                onChange={(event) => setBudgetValue(event.target.value)}
                disabled={walletBusy}
              />
            </FormField>

            {currentWallet && (
              <div className="space-y-3">
                {/* APT Budget Stats */}
                <div className="rounded-md border bg-muted/40 p-3 text-sm">
                  <div className="text-xs font-medium text-muted-foreground mb-2">APT Budget</div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Spent this period</span>
                    <span className="font-medium">
                      {formatAmount(currentWallet.spent_amount)}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Remaining</span>
                    <span
                      className={
                        currentWallet.budget_limit - currentWallet.spent_amount <= 0
                          ? 'font-medium text-destructive'
                          : 'font-medium'
                      }
                    >
                      {formatAmount(
                        currentWallet.budget_limit - currentWallet.spent_amount
                      )}
                    </span>
                  </div>
                </div>
                {/* VIBE Budget Stats */}
                <div className="rounded-md border border-primary/20 bg-primary/5 p-3 text-sm">
                  <div className="text-xs font-medium text-primary mb-2">VIBE Budget (1 VIBE = $0.01)</div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Budget limit</span>
                    <span className="font-medium text-primary">
                      {currentWallet.vibe_budget_limit != null
                        ? `${formatAmount(currentWallet.vibe_budget_limit)} (~$${(currentWallet.vibe_budget_limit * 0.01).toFixed(2)})`
                        : 'Unlimited'}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Spent</span>
                    <span className="font-medium">
                      {formatAmount(currentWallet.vibe_spent_amount)} (~${(currentWallet.vibe_spent_amount * 0.01).toFixed(2)})
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Remaining</span>
                    <span
                      className={
                        currentWallet.vibe_budget_limit != null &&
                        currentWallet.vibe_budget_limit - currentWallet.vibe_spent_amount <= 0
                          ? 'font-medium text-destructive'
                          : 'font-medium text-primary'
                      }
                    >
                      {currentWallet.vibe_budget_limit != null
                        ? `${formatAmount(currentWallet.vibe_budget_limit - currentWallet.vibe_spent_amount)} (~$${((currentWallet.vibe_budget_limit - currentWallet.vibe_spent_amount) * 0.01).toFixed(2)})`
                        : 'Unlimited'}
                    </span>
                  </div>
                </div>
              </div>
            )}

            {budgetError && (
              <p className="text-sm text-destructive">{budgetError}</p>
            )}
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={() => setBudgetModalOpen(false)}
              disabled={walletBusy}
            >
              Cancel
            </Button>
            <Button onClick={handleBudgetSave} disabled={walletBusy}>
              {walletBusy ? 'Saving…' : 'Save budget'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Agent Detail Dialog */}
      <AgentDetailDialog
        agent={selectedAgent}
        open={detailDialogOpen}
        onOpenChange={setDetailDialogOpen}
      />
    </>
  );
}
