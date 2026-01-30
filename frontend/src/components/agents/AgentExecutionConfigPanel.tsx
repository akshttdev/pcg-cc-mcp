/**
 * Agent Execution Configuration Panel
 *
 * Scalable UI for configuring execution behavior for any agent.
 * Supports Ralph Wiggum loop methodology and other execution patterns.
 */

import { useState, useEffect, useCallback, useMemo } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { Separator } from '@/components/ui/separator';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Loader2,
  Save,
  RotateCcw,
  ChevronDown,
  ChevronUp,
  Zap,
  GitBranch,
  Settings2,
  Info,
  Play,
  RefreshCw,
} from 'lucide-react';
import {
  agentExecutionConfigApi,
  type AgentExecutionConfig,
  type ExecutionMode,
  type CreateAgentExecutionConfig,
  type UpdateAgentExecutionConfig,
} from '@/lib/api';

interface AgentExecutionConfigPanelProps {
  agentId: string;
  agentName: string;
  onConfigChange?: (config: AgentExecutionConfig | null) => void;
}

const EXECUTION_MODE_OPTIONS: { value: ExecutionMode; label: string; description: string }[] = [
  {
    value: 'standard',
    label: 'Standard',
    description: 'Single execution without looping (default behavior)',
  },
  {
    value: 'ralph',
    label: 'Ralph Loop',
    description: 'Iterative execution until completion with validation',
  },
  {
    value: 'parallel',
    label: 'Parallel',
    description: 'Concurrent execution of multiple sub-tasks',
  },
  {
    value: 'pipeline',
    label: 'Pipeline',
    description: 'Sequential stages with handoff between phases',
  },
];

export function AgentExecutionConfigPanel({
  agentId,
  agentName,
  onConfigChange,
}: AgentExecutionConfigPanelProps) {
  const queryClient = useQueryClient();

  // Fetch execution profiles
  const {
    data: profiles = [],
    isLoading: profilesLoading,
  } = useQuery({
    queryKey: ['execution-profiles'],
    queryFn: agentExecutionConfigApi.listProfiles,
  });

  // Fetch agent's execution config
  const {
    data: configResponse,
    isLoading: configLoading,
    error: configError,
  } = useQuery({
    queryKey: ['agent-execution-config', agentId],
    queryFn: () => agentExecutionConfigApi.getAgentConfig(agentId),
  });

  // Local form state
  const [formState, setFormState] = useState<{
    execution_profile_id: string | null;
    execution_mode_override: ExecutionMode | null;
    max_iterations_override: number | null;
    backpressure_commands_override: string;
    system_prompt_prefix: string;
    system_prompt_suffix: string;
    auto_commit_on_success: boolean;
    auto_create_pr_on_complete: boolean;
    require_tests_pass: boolean;
  }>({
    execution_profile_id: null,
    execution_mode_override: null,
    max_iterations_override: null,
    backpressure_commands_override: '',
    system_prompt_prefix: '',
    system_prompt_suffix: '',
    auto_commit_on_success: false,
    auto_create_pr_on_complete: false,
    require_tests_pass: true,
  });

  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  // Determine if config exists
  const existingConfig = useMemo(() => {
    if (!configResponse) return null;
    return configResponse.type === 'Found' ? configResponse : null;
  }, [configResponse]);

  // Selected profile
  const selectedProfile = useMemo(() => {
    if (!formState.execution_profile_id) return null;
    return profiles.find(p => p.id === formState.execution_profile_id) || null;
  }, [profiles, formState.execution_profile_id]);

  // Initialize form from existing config
  useEffect(() => {
    if (existingConfig) {
      setFormState({
        execution_profile_id: existingConfig.execution_profile_id,
        execution_mode_override: existingConfig.execution_mode_override,
        max_iterations_override: existingConfig.max_iterations_override,
        backpressure_commands_override: existingConfig.backpressure_commands_override || '',
        system_prompt_prefix: existingConfig.system_prompt_prefix || '',
        system_prompt_suffix: existingConfig.system_prompt_suffix || '',
        auto_commit_on_success: existingConfig.auto_commit_on_success ?? false,
        auto_create_pr_on_complete: existingConfig.auto_create_pr_on_complete ?? false,
        require_tests_pass: existingConfig.require_tests_pass ?? true,
      });
      setHasChanges(false);
    }
  }, [existingConfig]);

  // Notify parent of config changes
  useEffect(() => {
    onConfigChange?.(existingConfig);
  }, [existingConfig, onConfigChange]);

  // Create mutation
  const createMutation = useMutation({
    mutationFn: (data: CreateAgentExecutionConfig) =>
      agentExecutionConfigApi.createAgentConfig(agentId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-execution-config', agentId] });
      toast.success('Execution config created');
      setHasChanges(false);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : 'Failed to create config');
    },
  });

  // Update mutation
  const updateMutation = useMutation({
    mutationFn: (data: UpdateAgentExecutionConfig) =>
      agentExecutionConfigApi.updateAgentConfig(agentId, data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agent-execution-config', agentId] });
      toast.success('Execution config updated');
      setHasChanges(false);
    },
    onError: (error) => {
      toast.error(error instanceof Error ? error.message : 'Failed to update config');
    },
  });

  const isSaving = createMutation.isPending || updateMutation.isPending;

  // Handle form changes
  const updateForm = useCallback(
    <K extends keyof typeof formState>(key: K, value: typeof formState[K]) => {
      setFormState(prev => ({ ...prev, [key]: value }));
      setHasChanges(true);
    },
    []
  );

  // Handle save
  const handleSave = useCallback(() => {
    const data = {
      agent_id: agentId,
      execution_profile_id: formState.execution_profile_id || undefined,
      execution_mode_override: formState.execution_mode_override || undefined,
      max_iterations_override: formState.max_iterations_override || undefined,
      backpressure_commands_override: formState.backpressure_commands_override || undefined,
      system_prompt_prefix: formState.system_prompt_prefix || undefined,
      system_prompt_suffix: formState.system_prompt_suffix || undefined,
      auto_commit_on_success: formState.auto_commit_on_success,
      auto_create_pr_on_complete: formState.auto_create_pr_on_complete,
      require_tests_pass: formState.require_tests_pass,
    };

    if (existingConfig) {
      updateMutation.mutate(data);
    } else {
      createMutation.mutate(data);
    }
  }, [formState, existingConfig, agentId, createMutation, updateMutation]);

  // Handle reset
  const handleReset = useCallback(() => {
    if (existingConfig) {
      setFormState({
        execution_profile_id: existingConfig.execution_profile_id,
        execution_mode_override: existingConfig.execution_mode_override,
        max_iterations_override: existingConfig.max_iterations_override,
        backpressure_commands_override: existingConfig.backpressure_commands_override || '',
        system_prompt_prefix: existingConfig.system_prompt_prefix || '',
        system_prompt_suffix: existingConfig.system_prompt_suffix || '',
        auto_commit_on_success: existingConfig.auto_commit_on_success ?? false,
        auto_create_pr_on_complete: existingConfig.auto_create_pr_on_complete ?? false,
        require_tests_pass: existingConfig.require_tests_pass ?? true,
      });
    } else {
      setFormState({
        execution_profile_id: null,
        execution_mode_override: null,
        max_iterations_override: null,
        backpressure_commands_override: '',
        system_prompt_prefix: '',
        system_prompt_suffix: '',
        auto_commit_on_success: false,
        auto_create_pr_on_complete: false,
        require_tests_pass: true,
      });
    }
    setHasChanges(false);
  }, [existingConfig]);

  // Effective execution mode
  const effectiveMode = useMemo(() => {
    return formState.execution_mode_override || selectedProfile?.execution_mode || 'standard';
  }, [formState.execution_mode_override, selectedProfile]);

  // Effective max iterations
  const effectiveMaxIterations = useMemo(() => {
    return formState.max_iterations_override ?? selectedProfile?.max_iterations ?? 50;
  }, [formState.max_iterations_override, selectedProfile]);

  if (profilesLoading || configLoading) {
    return (
      <Card>
        <CardHeader>
          <Skeleton className="h-6 w-48" />
          <Skeleton className="h-4 w-64 mt-2" />
        </CardHeader>
        <CardContent>
          <Skeleton className="h-32 w-full" />
        </CardContent>
      </Card>
    );
  }

  if (configError) {
    return (
      <Alert variant="destructive">
        <AlertDescription>
          Failed to load execution config: {configError instanceof Error ? configError.message : 'Unknown error'}
        </AlertDescription>
      </Alert>
    );
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Settings2 className="h-5 w-5" />
              Execution Configuration
            </CardTitle>
            <CardDescription>
              Configure how {agentName} executes development tasks
            </CardDescription>
          </div>
          {hasChanges && (
            <Badge variant="outline" className="text-amber-600 border-amber-300">
              Unsaved Changes
            </Badge>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Execution Profile Selector */}
        <div className="space-y-2">
          <Label htmlFor="profile">Base Profile</Label>
          <Select
            value={formState.execution_profile_id || '__none__'}
            onValueChange={(v) => updateForm('execution_profile_id', v === '__none__' ? null : v)}
          >
            <SelectTrigger id="profile">
              <SelectValue placeholder="Select a profile..." />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__none__">No base profile (use defaults)</SelectItem>
              {profiles.map(profile => (
                <SelectItem key={profile.id} value={profile.id}>
                  <div className="flex items-center gap-2">
                    <span>{profile.name}</span>
                    <Badge variant="secondary" className="text-xs">
                      {profile.execution_mode}
                    </Badge>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {selectedProfile?.description && (
            <p className="text-xs text-muted-foreground">{selectedProfile.description}</p>
          )}
        </div>

        <Separator />

        {/* Execution Mode */}
        <div className="space-y-2">
          <Label htmlFor="mode">Execution Mode</Label>
          <Select
            value={formState.execution_mode_override || '__inherit__'}
            onValueChange={(v) =>
              updateForm('execution_mode_override', v === '__inherit__' ? null : (v as ExecutionMode))
            }
          >
            <SelectTrigger id="mode">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__inherit__">
                <span className="text-muted-foreground">
                  Inherit from profile ({selectedProfile?.execution_mode || 'standard'})
                </span>
              </SelectItem>
              {EXECUTION_MODE_OPTIONS.map(opt => (
                <SelectItem key={opt.value} value={opt.value}>
                  <div className="flex flex-col">
                    <span>{opt.label}</span>
                    <span className="text-xs text-muted-foreground">{opt.description}</span>
                  </div>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        {/* Ralph Mode Settings */}
        {effectiveMode === 'ralph' && (
          <Card className="border-primary/20 bg-primary/5">
            <CardHeader className="pb-3">
              <CardTitle className="text-sm flex items-center gap-2">
                <RefreshCw className="h-4 w-4" />
                Ralph Loop Settings
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="max-iter">Max Iterations</Label>
                <div className="flex items-center gap-2">
                  <Input
                    id="max-iter"
                    type="number"
                    min={1}
                    max={200}
                    value={formState.max_iterations_override ?? ''}
                    onChange={(e) =>
                      updateForm(
                        'max_iterations_override',
                        e.target.value ? parseInt(e.target.value, 10) : null
                      )
                    }
                    placeholder={`Inherit (${effectiveMaxIterations})`}
                    className="w-32"
                  />
                  <span className="text-sm text-muted-foreground">
                    Effective: {effectiveMaxIterations}
                  </span>
                </div>
              </div>

              <div className="space-y-2">
                <Label htmlFor="backpressure">Backpressure Commands (one per line)</Label>
                <Textarea
                  id="backpressure"
                  value={formState.backpressure_commands_override}
                  onChange={(e) => updateForm('backpressure_commands_override', e.target.value)}
                  placeholder={`cargo test\ncargo clippy -- -D warnings\nnpm run lint`}
                  rows={3}
                  className="font-mono text-sm"
                />
                <p className="text-xs text-muted-foreground">
                  Commands run between iterations to validate progress. Loop continues if all pass.
                </p>
              </div>

              <div className="flex items-center gap-4">
                <div className="flex items-center gap-2">
                  <Switch
                    id="require-tests"
                    checked={formState.require_tests_pass}
                    onCheckedChange={(v) => updateForm('require_tests_pass', v)}
                  />
                  <Label htmlFor="require-tests" className="text-sm">
                    Require tests pass
                  </Label>
                </div>
              </div>
            </CardContent>
          </Card>
        )}

        {/* Feature Flags */}
        <div className="space-y-4">
          <Label className="text-base">Automation Features</Label>
          <div className="grid gap-3">
            <div className="flex items-center justify-between rounded-lg border p-3">
              <div className="flex items-center gap-3">
                <GitBranch className="h-4 w-4 text-muted-foreground" />
                <div>
                  <p className="text-sm font-medium">Auto-commit on success</p>
                  <p className="text-xs text-muted-foreground">
                    Automatically commit changes when execution completes
                  </p>
                </div>
              </div>
              <Switch
                checked={formState.auto_commit_on_success}
                onCheckedChange={(v) => updateForm('auto_commit_on_success', v)}
              />
            </div>

            <div className="flex items-center justify-between rounded-lg border p-3">
              <div className="flex items-center gap-3">
                <Play className="h-4 w-4 text-muted-foreground" />
                <div>
                  <p className="text-sm font-medium">Auto-create PR on complete</p>
                  <p className="text-xs text-muted-foreground">
                    Create a pull request when task is marked complete
                  </p>
                </div>
              </div>
              <Switch
                checked={formState.auto_create_pr_on_complete}
                onCheckedChange={(v) => updateForm('auto_create_pr_on_complete', v)}
              />
            </div>
          </div>
        </div>

        {/* Advanced Settings */}
        <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
          <CollapsibleTrigger asChild>
            <Button variant="ghost" className="w-full justify-between">
              <span className="flex items-center gap-2">
                <Zap className="h-4 w-4" />
                Advanced Settings
              </span>
              {advancedOpen ? (
                <ChevronUp className="h-4 w-4" />
              ) : (
                <ChevronDown className="h-4 w-4" />
              )}
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="space-y-4 pt-4">
            <div className="space-y-2">
              <Label htmlFor="system-prefix">System Prompt Prefix</Label>
              <Textarea
                id="system-prefix"
                value={formState.system_prompt_prefix}
                onChange={(e) => updateForm('system_prompt_prefix', e.target.value)}
                placeholder="Additional context to prepend to system prompt..."
                rows={3}
              />
              <p className="text-xs text-muted-foreground">
                Custom instructions added at the start of the system prompt
              </p>
            </div>

            <div className="space-y-2">
              <Label htmlFor="system-suffix">System Prompt Suffix</Label>
              <Textarea
                id="system-suffix"
                value={formState.system_prompt_suffix}
                onChange={(e) => updateForm('system_prompt_suffix', e.target.value)}
                placeholder="Additional context to append to system prompt..."
                rows={3}
              />
              <p className="text-xs text-muted-foreground">
                Custom instructions added at the end of the system prompt
              </p>
            </div>
          </CollapsibleContent>
        </Collapsible>

        <Separator />

        {/* Action Buttons */}
        <div className="flex items-center justify-between">
          <Button
            variant="ghost"
            onClick={handleReset}
            disabled={!hasChanges || isSaving}
          >
            <RotateCcw className="h-4 w-4 mr-2" />
            Reset
          </Button>
          <Button onClick={handleSave} disabled={!hasChanges || isSaving}>
            {isSaving ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Save className="h-4 w-4 mr-2" />
            )}
            {existingConfig ? 'Update Config' : 'Create Config'}
          </Button>
        </div>

        {/* Status Info */}
        {existingConfig && (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Info className="h-3 w-3" />
            <span>
              Last updated:{' '}
              {new Date(existingConfig.updated_at).toLocaleString()}
            </span>
          </div>
        )}
      </CardContent>
    </Card>
  );
}

export default AgentExecutionConfigPanel;
