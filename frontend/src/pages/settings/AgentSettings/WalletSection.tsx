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
import { FormField } from '@/components/ui/form-field';
import { EmptyState } from '@/components/ui/empty-state';
import { Loader2, Plus, Wallet } from 'lucide-react';
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
import { useQuery } from '@tanstack/react-query';
import { agentWalletApi } from '@/lib/api';
import type { AgentWallet, UpsertAgentWallet } from 'shared/types';
import { toast } from 'sonner';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { agentKeys } from '@/lib/query-keys';
import { useProfiles } from '@/hooks/useProfiles';

export function WalletSection() {
  const { t } = useTranslation('settings');
  const {
    profilesContent: serverProfilesContent,
    isLoading: profilesLoading,
  } = useProfiles();

  const [localParsedProfiles, setLocalParsedProfiles] = useState<any>(null);

  const {
    data: agentWallets = [],
    isLoading: walletsLoading,
    isFetching: walletsFetching,
  } = useQuery({
    queryKey: agentKeys.wallets(),
    queryFn: agentWalletApi.list,
  });

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

  if (profilesLoading) return null;

  return (
    <>
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
    </>
  );
}
