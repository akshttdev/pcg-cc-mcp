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
import { Badge } from '@/components/ui/badge';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Loader2, Plus, Wallet, Search, X, SortAsc, SortDesc } from 'lucide-react';
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
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { agentWalletApi, agentsApi, type AgentSearchParams } from '@/lib/api';
import type {
  AgentWallet,
  AgentWithParsedFields,
  UpsertAgentWallet,
  AgentStatus,
} from 'shared/types';
import { toast } from 'sonner';

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

  const queryClient = useQueryClient();
  const {
    data: agentWallets = [],
    isLoading: walletsLoading,
    isFetching: walletsFetching,
  } = useQuery({
    queryKey: ['agent-wallets'],
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

  const upsertWalletMutation = useMutation({
    mutationFn: agentWalletApi.upsert,
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-wallets'] });
      toast.success('Wallet budget saved');
      setBudgetModalOpen(false);
    },
    onError: (error: unknown) => {
      const message =
        error instanceof Error
          ? error.message
          : 'Failed to save wallet budget';
      setBudgetError(message);
      toast.error(message);
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
      const usdValue = vibe * 0.001; // 1 VIBE = $0.001
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

  const agentStatusStyles: Record<string, string> = {
    active: 'bg-emerald-100 text-emerald-700 border-emerald-200',
    inactive: 'bg-gray-100 text-gray-600 border-gray-200',
    maintenance: 'bg-amber-100 text-amber-700 border-amber-200',
    training: 'bg-blue-100 text-blue-700 border-blue-200',
  };

  if (profilesLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="h-8 w-8 animate-spin" />
        <span className="ml-2">{t('settings.agents.loading')}</span>
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
            <p className="text-sm text-muted-foreground">
              No registered agents yet. Seed the registry to expose Nora’s team.
            </p>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {agentDirectory.map((agent) => {
                const initials = agent.short_name
                  .split(' ')
                  .map((part) => part[0])
                  .join('')
                  .slice(0, 2)
                  .toUpperCase();
                const statusClass =
                  agentStatusStyles[agent.status] ||
                  'bg-gray-100 text-gray-600 border-gray-200';
                return (
                  <div
                    key={agent.id}
                    onClick={() => handleAgentClick(agent)}
                    className="group relative rounded-xl border bg-card overflow-hidden transition-all hover:shadow-md hover:border-primary/20 cursor-pointer"
                  >
                    {/* Status indicator */}
                    <div className="absolute top-3 right-3 z-10">
                      <Badge
                        variant="outline"
                        className={`text-xs capitalize backdrop-blur-sm ${statusClass}`}
                      >
                        {agent.status}
                      </Badge>
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
                          <span className="text-4xl font-bold text-primary/40">
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
            </div>
          )}
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
            <div className="text-sm text-muted-foreground">
              No agent budgets yet. Create one to cap spending for a profile.
            </div>
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
            <div className="space-y-1.5">
              <Label htmlFor="budget-profile">Agent profile</Label>
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
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="budget-display-name">Display name</Label>
              <Input
                id="budget-display-name"
                value={budgetDisplayName}
                onChange={(event) => setBudgetDisplayName(event.target.value)}
                disabled={walletBusy}
                placeholder={currentProfileOption?.label || 'Agent display name'}
              />
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="budget-limit">Monthly budget (credits)</Label>
              <Input
                id="budget-limit"
                type="number"
                min={0}
                value={budgetValue}
                onChange={(event) => setBudgetValue(event.target.value)}
                disabled={walletBusy}
              />
            </div>

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
                  <div className="text-xs font-medium text-primary mb-2">VIBE Budget (1 VIBE = $0.001)</div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Budget limit</span>
                    <span className="font-medium text-primary">
                      {currentWallet.vibe_budget_limit != null
                        ? `${formatAmount(currentWallet.vibe_budget_limit)} (~$${(currentWallet.vibe_budget_limit * 0.001).toFixed(2)})`
                        : 'Unlimited'}
                    </span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-muted-foreground">Spent</span>
                    <span className="font-medium">
                      {formatAmount(currentWallet.vibe_spent_amount)} (~${(currentWallet.vibe_spent_amount * 0.001).toFixed(2)})
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
                        ? `${formatAmount(currentWallet.vibe_budget_limit - currentWallet.vibe_spent_amount)} (~$${((currentWallet.vibe_budget_limit - currentWallet.vibe_spent_amount) * 0.001).toFixed(2)})`
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
