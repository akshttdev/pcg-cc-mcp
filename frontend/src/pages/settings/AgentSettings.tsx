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
import { Checkbox } from '@/components/ui/checkbox';
import { JSONEditor } from '@/components/ui/json-editor';
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
import { Skeleton } from '@/components/ui/skeleton';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { agentWalletApi, agentsApi } from '@/lib/api';
import type {
  AgentWallet,
  AgentWithParsedFields,
  UpsertAgentWallet,
} from 'shared/types';
import { toast } from 'sonner';

import { ExecutorConfigForm } from '@/components/ExecutorConfigForm';
import { useProfiles } from '@/hooks/useProfiles';
import { useUserSystem } from '@/components/config-provider';
import { showModal } from '@/lib/modals';

export function AgentSettings() {
  const { t } = useTranslation('settings');
  // Use profiles hook for server state
  const {
    profilesContent: serverProfilesContent,
    profilesPath,
    isLoading: profilesLoading,
    isSaving: profilesSaving,
    error: profilesError,
    save: saveProfiles,
  } = useProfiles();

  const { reloadSystem } = useUserSystem();

  // Local editor state (draft that may differ from server)
  const [localProfilesContent, setLocalProfilesContent] = useState('');
  const [profilesSuccess, setProfilesSuccess] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Form-based editor state
  const [useFormEditor, setUseFormEditor] = useState(true);
  const [selectedExecutorType, setSelectedExecutorType] =
    useState<string>('CLAUDE_CODE');
  const [selectedConfiguration, setSelectedConfiguration] =
    useState<string>('DEFAULT');
  const [localParsedProfiles, setLocalParsedProfiles] = useState<any>(null);
  const [isDirty, setIsDirty] = useState(false);

  const queryClient = useQueryClient();
  const {
    data: agentWallets = [],
    isLoading: walletsLoading,
    isFetching: walletsFetching,
  } = useQuery({
    queryKey: ['agent-wallets'],
    queryFn: agentWalletApi.list,
  });

  const {
    data: agentDirectory = [],
    isLoading: agentsLoading,
    error: agentsError,
  } = useQuery<AgentWithParsedFields[], Error>({
    queryKey: ['agents'],
    queryFn: agentsApi.list,
  });

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

  // Sync server state to local state when not dirty
  useEffect(() => {
    if (!isDirty && serverProfilesContent) {
      setLocalProfilesContent(serverProfilesContent);
      // Parse JSON inside effect to avoid object dependency
      try {
        const parsed = JSON.parse(serverProfilesContent);
        setLocalParsedProfiles(parsed);
      } catch (err) {
        console.error('Failed to parse profiles JSON:', err);
        setLocalParsedProfiles(null);
      }
    }
  }, [serverProfilesContent, isDirty]);

  useEffect(() => {
    if (!budgetModalOpen) {
      setBudgetProfileKey('');
      setBudgetDisplayName('');
      setBudgetValue('');
      setIsNewBudget(false);
      setBudgetError(null);
    }
  }, [budgetModalOpen]);

  // Sync raw profiles with parsed profiles
  const syncRawProfiles = (profiles: unknown) => {
    setLocalProfilesContent(JSON.stringify(profiles, null, 2));
  };

  // Mark profiles as dirty
  const markDirty = (nextProfiles: unknown) => {
    setLocalParsedProfiles(nextProfiles);
    syncRawProfiles(nextProfiles);
    setIsDirty(true);
  };

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

  // Open create dialog
  const openCreateDialog = async () => {
    try {
      const result = await showModal<{
        action: 'created' | 'canceled';
        configName?: string;
        cloneFrom?: string | null;
      }>('create-configuration', {
        executorType: selectedExecutorType,
        existingConfigs: Object.keys(
          localParsedProfiles?.executors?.[selectedExecutorType] || {}
        ),
      });

      if (result.action === 'created' && result.configName) {
        createConfiguration(
          selectedExecutorType,
          result.configName,
          result.cloneFrom
        );
      }
    } catch (error) {
      // User cancelled - do nothing
    }
  };

  // Create new configuration
  const createConfiguration = (
    executorType: string,
    configName: string,
    baseConfig?: string | null
  ) => {
    if (!localParsedProfiles || !localParsedProfiles.executors) return;

    const base =
      baseConfig &&
      localParsedProfiles.executors[executorType]?.[baseConfig]?.[executorType]
        ? localParsedProfiles.executors[executorType][baseConfig][executorType]
        : {};

    const updatedProfiles = {
      ...localParsedProfiles,
      executors: {
        ...localParsedProfiles.executors,
        [executorType]: {
          ...localParsedProfiles.executors[executorType],
          [configName]: {
            [executorType]: base,
          },
        },
      },
    };

    markDirty(updatedProfiles);
    setSelectedConfiguration(configName);
  };

  // Open delete dialog
  const openDeleteDialog = async (configName: string) => {
    try {
      const result = await showModal<'deleted' | 'canceled'>(
        'delete-configuration',
        {
          configName,
          executorType: selectedExecutorType,
        }
      );

      if (result === 'deleted') {
        await handleDeleteConfiguration(configName);
      }
    } catch (error) {
      // User cancelled - do nothing
    }
  };

  // Handle delete configuration
  const handleDeleteConfiguration = async (configToDelete: string) => {
    if (!localParsedProfiles) {
      return;
    }

    // Clear any previous errors
    setSaveError(null);

    try {
      // Validate that the configuration exists
      if (
        !localParsedProfiles.executors[selectedExecutorType]?.[configToDelete]
      ) {
        return;
      }

      // Check if this is the last configuration
      const currentConfigs = Object.keys(
        localParsedProfiles.executors[selectedExecutorType] || {}
      );
      if (currentConfigs.length <= 1) {
        return;
      }

      // Remove the configuration from the executor
      const remainingConfigs = {
        ...localParsedProfiles.executors[selectedExecutorType],
      };
      delete remainingConfigs[configToDelete];

      const updatedProfiles = {
        ...localParsedProfiles,
        executors: {
          ...localParsedProfiles.executors,
          [selectedExecutorType]: remainingConfigs,
        },
      };

      // If no configurations left, create a blank DEFAULT (should not happen due to check above)
      if (Object.keys(remainingConfigs).length === 0) {
        updatedProfiles.executors[selectedExecutorType] = {
          DEFAULT: { [selectedExecutorType]: {} },
        };
      }

      try {
        // Save using hook
        await saveProfiles(JSON.stringify(updatedProfiles, null, 2));

        // Update local state and reset dirty flag
        setLocalParsedProfiles(updatedProfiles);
        setLocalProfilesContent(JSON.stringify(updatedProfiles, null, 2));
        setIsDirty(false);

        // Select the next available configuration
        const nextConfigs = Object.keys(
          updatedProfiles.executors[selectedExecutorType]
        );
        const nextSelected = nextConfigs[0] || 'DEFAULT';
        setSelectedConfiguration(nextSelected);

        // Show success
        setProfilesSuccess(true);
        setTimeout(() => setProfilesSuccess(false), 3000);

        // Refresh global system so deleted configs are removed elsewhere
        reloadSystem();
      } catch (saveError: unknown) {
        console.error('Failed to save deletion to backend:', saveError);
        setSaveError(t('settings.agents.errors.deleteFailed'));
      }
    } catch (error) {
      console.error('Error deleting configuration:', error);
    }
  };

  const handleProfilesChange = (value: string) => {
    setLocalProfilesContent(value);
    setIsDirty(true);

    // Validate JSON on change
    if (value.trim()) {
      try {
        const parsed = JSON.parse(value);
        setLocalParsedProfiles(parsed);
      } catch (err) {
        // Invalid JSON, keep local content but clear parsed
        setLocalParsedProfiles(null);
      }
    }
  };

  const handleSaveProfiles = async () => {
    // Clear any previous errors
    setSaveError(null);

    try {
      const contentToSave =
        useFormEditor && localParsedProfiles
          ? JSON.stringify(localParsedProfiles, null, 2)
          : localProfilesContent;

      await saveProfiles(contentToSave);
      setProfilesSuccess(true);
      setIsDirty(false);
      setTimeout(() => setProfilesSuccess(false), 3000);

      // Update the local content if using form editor
      if (useFormEditor && localParsedProfiles) {
        setLocalProfilesContent(contentToSave);
      }

      // Refresh global system so new profiles are available elsewhere
      reloadSystem();
    } catch (err: unknown) {
      console.error('Failed to save profiles:', err);
      setSaveError(t('settings.agents.errors.saveFailed'));
    }
  };

  const handleExecutorConfigChange = (
    executorType: string,
    configuration: string,
    formData: unknown
  ) => {
    if (!localParsedProfiles || !localParsedProfiles.executors) return;

    // Update the parsed profiles with the new config
    const updatedProfiles = {
      ...localParsedProfiles,
      executors: {
        ...localParsedProfiles.executors,
        [executorType]: {
          ...localParsedProfiles.executors[executorType],
          [configuration]: {
            [executorType]: formData,
          },
        },
      },
    };

    markDirty(updatedProfiles);
  };

  const handleExecutorConfigSave = async (formData: unknown) => {
    if (!localParsedProfiles || !localParsedProfiles.executors) return;

    // Clear any previous errors
    setSaveError(null);

    // Update the parsed profiles with the saved config
    const updatedProfiles = {
      ...localParsedProfiles,
      executors: {
        ...localParsedProfiles.executors,
        [selectedExecutorType]: {
          ...localParsedProfiles.executors[selectedExecutorType],
          [selectedConfiguration]: {
            [selectedExecutorType]: formData,
          },
        },
      },
    };

    // Update state
    setLocalParsedProfiles(updatedProfiles);

    // Save the updated profiles directly
    try {
      const contentToSave = JSON.stringify(updatedProfiles, null, 2);

      await saveProfiles(contentToSave);
      setProfilesSuccess(true);
      setIsDirty(false);
      setTimeout(() => setProfilesSuccess(false), 3000);

      // Update the local content as well
      setLocalProfilesContent(contentToSave);

      // Refresh global system so new profiles are available elsewhere
      reloadSystem();
    } catch (err: unknown) {
      console.error('Failed to save profiles:', err);
      setSaveError(t('settings.agents.errors.saveConfigFailed'));
    }
  };

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

      {profilesSuccess && (
        <Alert className="border-green-200 bg-green-50 text-green-800 dark:border-green-800 dark:bg-green-950 dark:text-green-200">
          <AlertDescription className="font-medium">
            {t('settings.agents.save.success')}
          </AlertDescription>
        </Alert>
      )}

      {saveError && (
        <Alert variant="destructive">
          <AlertDescription>{saveError}</AlertDescription>
        </Alert>
      )}

      <Card>
        <CardHeader>
          <CardTitle>Autonomous Agents</CardTitle>
          <CardDescription>
            Live directory of Nora, Maci, and the social command specialists.
          </CardDescription>
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
            <div className="grid gap-4 md:grid-cols-2">
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
                    className="rounded-lg border border-dashed p-4 space-y-3"
                  >
                    <div className="flex items-center gap-3">
                      {agent.avatar_url ? (
                        <img
                          src={agent.avatar_url}
                          alt={agent.short_name}
                          className="h-10 w-10 rounded-full object-cover"
                        />
                      ) : (
                        <div className="h-10 w-10 rounded-full bg-muted flex items-center justify-center text-sm font-semibold">
                          {initials}
                        </div>
                      )}
                      <div className="min-w-0 flex-1">
                        <p className="font-medium truncate">{agent.short_name}</p>
                        <p className="text-sm text-muted-foreground truncate">
                          {agent.designation || 'Specialist Agent'}
                        </p>
                      </div>
                      <Badge
                        variant="outline"
                        className={`text-xs capitalize ${statusClass}`}
                      >
                        {agent.status}
                      </Badge>
                    </div>
                    <div className="flex flex-wrap items-center gap-3 text-xs text-muted-foreground">
                      <span>
                        Autonomy: {agent.autonomy_level?.replace('_', ' ') || 'manual'}
                      </span>
                      {agent.default_model && <span>Model: {agent.default_model}</span>}
                    </div>
                    {agent.capabilities && agent.capabilities.length > 0 && (
                      <div className="flex flex-wrap gap-2">
                        {agent.capabilities.slice(0, 4).map((capability) => (
                          <Badge key={capability} variant="secondary">
                            {capability}
                          </Badge>
                        ))}
                        {agent.capabilities.length > 4 && (
                          <Badge variant="outline">
                            +{agent.capabilities.length - 4}
                          </Badge>
                        )}
                      </div>
                    )}
                    {agent.tools && agent.tools.length > 0 && (
                      <p className="text-xs text-muted-foreground">
                        Tools: {agent.tools.slice(0, 4).join(', ')}
                        {agent.tools.length > 4 && '…'}
                      </p>
                    )}
                    {agent.description && (
                      <p className="text-sm text-muted-foreground line-clamp-2">
                        {agent.description}
                      </p>
                    )}
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
                    <TableHead className="text-right">Budget</TableHead>
                    <TableHead className="text-right">Spent</TableHead>
                    <TableHead className="text-right">Available</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sortedWallets.map((wallet) => {
                    const option = profileOptions.find(
                      (opt) => opt.profileKey === wallet.profile_key
                    );
                    const available = wallet.budget_limit - wallet.spent_amount;
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

      <Card>
        <CardHeader>
          <CardTitle>{t('settings.agents.title')}</CardTitle>
          <CardDescription>{t('settings.agents.description')}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Editor type toggle */}
          <div className="flex items-center space-x-2">
            <Checkbox
              id="use-form-editor"
              checked={!useFormEditor}
              onCheckedChange={(checked) => setUseFormEditor(!checked)}
              disabled={profilesLoading || !localParsedProfiles}
            />
            <Label htmlFor="use-form-editor">
              {t('settings.agents.editor.formLabel')}
            </Label>
          </div>

          {useFormEditor &&
          localParsedProfiles &&
          localParsedProfiles.executors ? (
            // Form-based editor
            <div className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-2">
                  <Label htmlFor="executor-type">
                    {t('settings.agents.editor.agentLabel')}
                  </Label>
                  <Select
                    value={selectedExecutorType}
                    onValueChange={(value) => {
                      setSelectedExecutorType(value);
                      // Reset configuration selection when executor type changes
                      setSelectedConfiguration('DEFAULT');
                    }}
                  >
                    <SelectTrigger id="executor-type">
                      <SelectValue
                        placeholder={t(
                          'settings.agents.editor.agentPlaceholder'
                        )}
                      />
                    </SelectTrigger>
                    <SelectContent>
                      {Object.keys(localParsedProfiles.executors).map(
                        (type) => (
                          <SelectItem key={type} value={type}>
                            {type}
                          </SelectItem>
                        )
                      )}
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label htmlFor="configuration">
                    {t('settings.agents.editor.configLabel')}
                  </Label>
                  <div className="flex gap-2">
                    <Select
                      value={selectedConfiguration}
                      onValueChange={(value) => {
                        if (value === '__create__') {
                          openCreateDialog();
                        } else {
                          setSelectedConfiguration(value);
                        }
                      }}
                      disabled={
                        !localParsedProfiles.executors[selectedExecutorType]
                      }
                    >
                      <SelectTrigger id="configuration">
                        <SelectValue
                          placeholder={t(
                            'settings.agents.editor.configPlaceholder'
                          )}
                        />
                      </SelectTrigger>
                      <SelectContent>
                        {Object.keys(
                          localParsedProfiles.executors[selectedExecutorType] ||
                            {}
                        ).map((configuration) => (
                          <SelectItem key={configuration} value={configuration}>
                            {configuration}
                          </SelectItem>
                        ))}
                        <SelectItem value="__create__">
                          {t('settings.agents.editor.createNew')}
                        </SelectItem>
                      </SelectContent>
                    </Select>
                    <Button
                      variant="destructive"
                      size="sm"
                      className="h-10"
                      onClick={() => openDeleteDialog(selectedConfiguration)}
                      disabled={
                        profilesSaving ||
                        !localParsedProfiles.executors[selectedExecutorType] ||
                        Object.keys(
                          localParsedProfiles.executors[selectedExecutorType] ||
                            {}
                        ).length <= 1
                      }
                      title={
                        Object.keys(
                          localParsedProfiles.executors[selectedExecutorType] ||
                            {}
                        ).length <= 1
                          ? t('settings.agents.editor.deleteTitle')
                          : t('settings.agents.editor.deleteButton', {
                              name: selectedConfiguration,
                            })
                      }
                    >
                      {t('settings.agents.editor.deleteText')}
                    </Button>
                  </div>
                </div>
              </div>

              {localParsedProfiles.executors[selectedExecutorType]?.[
                selectedConfiguration
              ]?.[selectedExecutorType] && (
                <ExecutorConfigForm
                  executor={selectedExecutorType as any}
                  value={
                    localParsedProfiles.executors[selectedExecutorType][
                      selectedConfiguration
                    ][selectedExecutorType] || {}
                  }
                  onChange={(formData) =>
                    handleExecutorConfigChange(
                      selectedExecutorType,
                      selectedConfiguration,
                      formData
                    )
                  }
                  onSave={handleExecutorConfigSave}
                  disabled={profilesSaving}
                  isSaving={profilesSaving}
                  isDirty={isDirty}
                />
              )}
            </div>
          ) : (
            // Raw JSON editor
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="profiles-editor">
                  {t('settings.agents.editor.jsonLabel')}
                </Label>
                <JSONEditor
                  id="profiles-editor"
                  placeholder={t('settings.agents.editor.jsonPlaceholder')}
                  value={
                    profilesLoading
                      ? t('settings.agents.editor.jsonLoading')
                      : localProfilesContent
                  }
                  onChange={handleProfilesChange}
                  disabled={profilesLoading}
                  minHeight={300}
                />
              </div>

              {!profilesError && profilesPath && (
                <div className="space-y-2">
                  <p className="text-sm text-muted-foreground">
                    <span className="font-medium">
                      {t('settings.agents.editor.pathLabel')}
                    </span>{' '}
                    <span className="font-mono text-xs">{profilesPath}</span>
                  </p>
                </div>
              )}
            </div>
          )}
        </CardContent>
      </Card>

      {!useFormEditor && (
        <div className="sticky bottom-0 z-10 bg-background/80 backdrop-blur-sm border-t py-4">
          <div className="flex justify-end">
            <Button
              onClick={handleSaveProfiles}
              disabled={!isDirty || profilesSaving || !!profilesError}
            >
              {profilesSaving && (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              )}
              {t('settings.agents.save.button')}
            </Button>
          </div>
        </div>
      )}
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
              <div className="rounded-md border bg-muted/40 p-3 text-sm">
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
