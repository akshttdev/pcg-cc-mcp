import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { workflowKeys } from '@/lib/query-keys';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Switch } from '@/components/ui/switch';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Plus,
  Trash2,
  Zap,
  ZapOff,
  Loader2,
  Clock,
  Activity,
  Globe,
  Copy,
  AlertTriangle,
} from 'lucide-react';
import { toast } from 'sonner';
import { cn } from '@/lib/utils';
import {
  triggersApi,
  workflowsApi,
  DATA_TYPE_OPTIONS,
} from '@/lib/api';
import type {
  CreateWorkflowTrigger,
  AvailableModel,
} from '@/lib/api';

const TRIGGER_TYPE_OPTIONS = [
  { value: 'data_source_created', label: 'Data Source Created' },
  { value: 'data_source_updated', label: 'Data Source Updated' },
  { value: 'schedule' as const, label: 'Schedule (Recurring)' },
  { value: 'webhook' as const, label: 'Webhook (External)' },
] as const;

const SCHEDULE_INTERVAL_OPTIONS = [
  { value: 'every_5m', label: 'Every 5 minutes' },
  { value: 'every_15m', label: 'Every 15 minutes' },
  { value: 'every_30m', label: 'Every 30 minutes' },
  { value: 'hourly', label: 'Hourly' },
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
] as const;

interface WorkflowTriggersPanelProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  workflowId: string;
  workflowName?: string;
}

export function WorkflowTriggersPanel({
  open,
  onOpenChange,
  workflowId,
  workflowName,
}: WorkflowTriggersPanelProps) {
  const [showCreateForm, setShowCreateForm] = useState(false);

  // Form state for creating a new trigger
  const [newName, setNewName] = useState('');
  const [newTriggerType, setNewTriggerType] = useState('data_source_created');
  const [newDataSourceTypes, setNewDataSourceTypes] = useState<string[]>([]);
  const [newOrgId, setNewOrgId] = useState('');
  const [newProjectId, setNewProjectId] = useState('');
  const [newTags, setNewTags] = useState('');
  const [newModelOverride, setNewModelOverride] = useState('');
  const [newAutoApprove, setNewAutoApprove] = useState(false);
  const [newScheduleInterval, setNewScheduleInterval] = useState('hourly');
  const [newCooldownSeconds, setNewCooldownSeconds] = useState(0);
  const [newMaxRetries, setNewMaxRetries] = useState(0);

  const { data: triggers = [], isLoading } = useQuery({
    queryKey: workflowKeys.triggers(workflowId),
    queryFn: () => triggersApi.list(workflowId),
    enabled: open && !!workflowId,
  });

  const { data: availableModels = [] } = useQuery<AvailableModel[]>({
    queryKey: workflowKeys.models(),
    queryFn: () => workflowsApi.listAvailableModels(),
    staleTime: 60 * 60 * 1000,
    enabled: open,
  });

  const createMutation = useMutationWithToast({
    mutationFn: (data: CreateWorkflowTrigger) => triggersApi.create(data),
    successMessage: 'Trigger created',
    errorMessage: 'Failed to create trigger',
    invalidateKeys: [workflowKeys.triggers(workflowId)],
    onSuccess: () => {
      resetForm();
    },
  });

  const toggleMutation = useMutationWithToast({
    mutationFn: (id: string) => triggersApi.toggle(id),
    successMessage: (result) => `Trigger ${result.enabled ? 'enabled' : 'disabled'}`,
    errorMessage: 'Failed to toggle trigger',
    invalidateKeys: [workflowKeys.triggers(workflowId)],
  });

  const deleteMutation = useMutationWithToast({
    mutationFn: (id: string) => triggersApi.delete(id),
    successMessage: 'Trigger deleted',
    errorMessage: 'Failed to delete trigger',
    invalidateKeys: [workflowKeys.triggers(workflowId)],
  });

  function resetForm() {
    setShowCreateForm(false);
    setNewName('');
    setNewTriggerType('data_source_created');
    setNewDataSourceTypes([]);
    setNewOrgId('');
    setNewProjectId('');
    setNewTags('');
    setNewModelOverride('');
    setNewAutoApprove(false);
    setNewScheduleInterval('hourly');
    setNewCooldownSeconds(0);
    setNewMaxRetries(0);
  }

  function handleCreate() {
    const isSchedule = newTriggerType === 'schedule';
    const isWebhook = newTriggerType === 'webhook';
    const data: CreateWorkflowTrigger = {
      workflow_id: workflowId,
      name: newName,
      trigger_type: newTriggerType,
      filter_data_source_types: (isSchedule || isWebhook) ? undefined : (newDataSourceTypes.length > 0 ? newDataSourceTypes : undefined),
      filter_organization_id: newOrgId || undefined,
      filter_project_id: newProjectId || undefined,
      filter_tags: isSchedule ? [JSON.stringify({ interval: newScheduleInterval })] : (newTags ? newTags.split(',').map((t) => t.trim()).filter(Boolean) : undefined),
      model_override: newModelOverride || undefined,
      auto_approve: newAutoApprove,
      cooldown_seconds: newCooldownSeconds > 0 ? newCooldownSeconds : undefined,
      max_retries: newMaxRetries > 0 ? newMaxRetries : undefined,
    };
    createMutation.mutate(data);
  }

  function toggleDataSourceType(type: string) {
    setNewDataSourceTypes((prev) =>
      prev.includes(type) ? prev.filter((t) => t !== type) : [...prev, type]
    );
  }

  function parseScheduleInterval(filterTags: string | null): string | null {
    if (!filterTags) return null;
    try {
      const tags = JSON.parse(filterTags);
      if (Array.isArray(tags)) {
        for (const tag of tags) {
          try {
            const parsed = typeof tag === 'string' ? JSON.parse(tag) : tag;
            if (parsed?.interval) return parsed.interval;
          } catch { /* not JSON */ }
        }
      }
    } catch { /* ignore */ }
    return null;
  }

  function parseDsTypes(jsonStr: string | null): string[] {
    if (!jsonStr) return [];
    try {
      return JSON.parse(jsonStr);
    } catch {
      return [];
    }
  }

  function formatRelativeTime(dateStr: string | null): string {
    if (!dateStr) return 'Never';
    const date = new Date(dateStr + 'Z');
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    if (diffMins < 1) return 'Just now';
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHrs = Math.floor(diffMins / 60);
    if (diffHrs < 24) return `${diffHrs}h ago`;
    const diffDays = Math.floor(diffHrs / 24);
    return `${diffDays}d ago`;
  }

  const enabledCount = triggers.filter((t) => t.enabled).length;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[85vh] flex flex-col">
        <DialogTitle className="flex items-center gap-2">
          <Zap className="h-5 w-5 text-amber-500" />
          Auto-Triggers
          {workflowName && (
            <span className="text-sm font-normal text-muted-foreground">
              for {workflowName}
            </span>
          )}
          {enabledCount > 0 && (
            <Badge variant="secondary" className="ml-2">
              {enabledCount} active
            </Badge>
          )}
        </DialogTitle>

        <ScrollArea className="flex-1 -mx-6 px-6">
          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : (
            <div className="space-y-3">
              {triggers.length === 0 && !showCreateForm && (
                <div className="text-center py-8 text-muted-foreground">
                  <ZapOff className="h-10 w-10 mx-auto mb-3 opacity-40" />
                  <p className="text-sm">No triggers configured.</p>
                  <p className="text-xs mt-1">
                    Triggers let this workflow run automatically when new data sources are created.
                  </p>
                </div>
              )}

              {/* Existing triggers */}
              {triggers.map((trigger) => {
                const dsTypes = parseDsTypes(trigger.filter_data_source_types);
                const scheduleInterval = trigger.trigger_type === 'schedule'
                  ? parseScheduleInterval(trigger.filter_tags)
                  : null;
                return (
                  <div
                    key={trigger.id}
                    className={cn(
                      'border rounded-lg p-3 transition-colors',
                      trigger.enabled
                        ? 'border-amber-500/30 bg-amber-500/5'
                        : 'border-border bg-muted/20'
                    )}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2 min-w-0">
                        <Switch
                          checked={trigger.enabled}
                          onCheckedChange={() => toggleMutation.mutate(trigger.id)}
                          disabled={toggleMutation.isPending}
                        />
                        <span className="font-medium text-sm truncate">
                          {trigger.name || 'Unnamed Trigger'}
                        </span>
                      </div>
                      <div className="flex items-center gap-2 shrink-0">
                        <Badge variant="outline" className="text-[10px]">
                          {TRIGGER_TYPE_OPTIONS.find(
                            (o) => o.value === trigger.trigger_type
                          )?.label || trigger.trigger_type}
                        </Badge>
                        <IconButton
                          variant="ghost" className="h-7 w-7 text-muted-foreground hover:text-destructive"
                          onClick={() => {
                            if (confirm('Delete this trigger?')) {
                            deleteMutation.mutate(trigger.id);
                            }
                          }}
                          icon={Trash2}
                          label="Delete trigger"
                          iconClassName="h-3.5 w-3.5"
                        />
                      </div>
                    </div>

                    {/* Filter summary */}
                    <div className="mt-2 flex flex-wrap gap-1.5">
                      {scheduleInterval && (
                        <Badge variant="secondary" className="text-[10px]">
                          {SCHEDULE_INTERVAL_OPTIONS.find((o) => o.value === scheduleInterval)?.label || scheduleInterval}
                        </Badge>
                      )}
                      {dsTypes.length > 0 ? (
                        dsTypes.map((t) => (
                          <Badge key={t} variant="secondary" className="text-[10px]">
                            {DATA_TYPE_OPTIONS.find((o) => o.value === t)?.label || t}
                          </Badge>
                        ))
                      ) : (
                        <Badge variant="secondary" className="text-[10px] opacity-60">
                          All data types
                        </Badge>
                      )}
                      {trigger.filter_organization_id && (
                        <Badge variant="secondary" className="text-[10px]">
                          Org filter
                        </Badge>
                      )}
                      {trigger.filter_project_id && (
                        <Badge variant="secondary" className="text-[10px]">
                          Project filter
                        </Badge>
                      )}
                      {trigger.model_override && (
                        <Badge variant="outline" className="text-[10px]">
                          Model: {trigger.model_override}
                        </Badge>
                      )}
                      {trigger.auto_approve && (
                        <Badge className="text-[10px] bg-green-600">Auto-approve</Badge>
                      )}
                    </div>

                    {/* Webhook URL + secret */}
                    {trigger.trigger_type === 'webhook' && trigger.webhook_url && (
                      <div className="mt-2 space-y-1.5">
                        <div className="flex items-center gap-1.5 text-[11px]">
                          <Globe className="h-3 w-3 text-muted-foreground shrink-0" />
                          <code className="bg-muted px-1.5 py-0.5 rounded text-[10px] truncate flex-1">
                            {trigger.webhook_url}
                          </code>
                          <button
                            type="button"
                            className="text-muted-foreground hover:text-foreground"
                            onClick={() => {
                              navigator.clipboard.writeText(
                                `${window.location.origin}${trigger.webhook_url}`
                              );
                              toast.success('Webhook URL copied');
                            }}
                          >
                            <Copy className="h-3 w-3" />
                          </button>
                        </div>
                        {trigger.webhook_secret && (
                          <div className="flex items-center gap-1.5 text-[11px]">
                            <span className="text-muted-foreground shrink-0">Secret:</span>
                            <code className="bg-muted px-1.5 py-0.5 rounded text-[10px] truncate flex-1">
                              {trigger.webhook_secret.slice(0, 8)}...
                            </code>
                            <button
                              type="button"
                              className="text-muted-foreground hover:text-foreground"
                              onClick={() => {
                                navigator.clipboard.writeText(trigger.webhook_secret ?? '');
                                toast.success('Webhook secret copied');
                              }}
                            >
                              <Copy className="h-3 w-3" />
                            </button>
                          </div>
                        )}
                      </div>
                    )}

                    {/* Error state */}
                    {trigger.last_error && (
                      <div className="mt-2 flex items-start gap-1.5 text-[11px] text-destructive">
                        <AlertTriangle className="h-3 w-3 mt-0.5 shrink-0" />
                        <span className="truncate">{trigger.last_error}</span>
                        {trigger.retry_count > 0 && (
                          <Badge variant="destructive" className="text-[9px] shrink-0">
                            {trigger.retry_count} retries
                          </Badge>
                        )}
                      </div>
                    )}

                    {/* Stats row */}
                    <div className="mt-2 flex items-center gap-4 text-[11px] text-muted-foreground">
                      <span className="flex items-center gap-1">
                        <Activity className="h-3 w-3" />
                        {trigger.trigger_count} runs
                      </span>
                      <span className="flex items-center gap-1">
                        <Clock className="h-3 w-3" />
                        Last: {formatRelativeTime(trigger.last_triggered_at)}
                      </span>
                      {trigger.cooldown_seconds > 0 && (
                        <span className="text-muted-foreground">
                          Cooldown: {trigger.cooldown_seconds}s
                        </span>
                      )}
                    </div>
                  </div>
                );
              })}

              {/* Create form */}
              {showCreateForm && (
                <div className="border rounded-lg p-4 space-y-3 bg-muted/10">
                  <h4 className="text-sm font-medium">New Trigger</h4>

                  <div>
                    <Label className="text-xs">Name</Label>
                    <Input
                      className="h-8 text-sm mt-1"
                      placeholder="e.g. Auto-analyze conversations"
                      value={newName}
                      onChange={(e) => setNewName(e.target.value)}
                    />
                  </div>

                  <div>
                    <Label className="text-xs">Trigger Type</Label>
                    <Select value={newTriggerType} onValueChange={setNewTriggerType}>
                      <SelectTrigger className="h-8 text-sm mt-1">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        {TRIGGER_TYPE_OPTIONS.map((opt) => (
                          <SelectItem key={opt.value} value={opt.value}>
                            {opt.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  {newTriggerType === 'webhook' ? (
                    <div className="rounded-md border border-blue-500/20 bg-blue-500/5 p-3">
                      <p className="text-xs text-blue-600 dark:text-blue-400 font-medium">
                        Webhook triggers accept external HTTP POST requests.
                      </p>
                      <p className="text-[11px] text-muted-foreground mt-1">
                        After creation, you'll receive a unique URL and HMAC secret.
                        External systems POST data to that URL, which fires this workflow
                        with the request body as content.
                      </p>
                    </div>
                  ) : newTriggerType === 'schedule' ? (
                    <div>
                      <Label className="text-xs">Schedule Interval</Label>
                      <Select value={newScheduleInterval} onValueChange={setNewScheduleInterval}>
                        <SelectTrigger className="h-8 text-sm mt-1">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {SCHEDULE_INTERVAL_OPTIONS.map((opt) => (
                            <SelectItem key={opt.value} value={opt.value}>
                              {opt.label}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <p className="text-[11px] text-muted-foreground mt-1">
                        Schedule triggers run without a data source, relying on action nodes.
                      </p>
                    </div>
                  ) : (
                    <div>
                      <Label className="text-xs">
                        Data Source Types{' '}
                        <span className="text-muted-foreground">(leave empty for all)</span>
                      </Label>
                      <div className="flex flex-wrap gap-2 mt-1.5">
                        {DATA_TYPE_OPTIONS.map((opt) => (
                          <button
                            key={opt.value}
                            type="button"
                            onClick={() => toggleDataSourceType(opt.value)}
                            className={cn(
                              'px-2.5 py-1 rounded-md text-xs border transition-colors',
                              newDataSourceTypes.includes(opt.value)
                                ? 'bg-primary text-primary-foreground border-primary'
                                : 'bg-muted/30 text-muted-foreground border-border hover:bg-muted/50'
                            )}
                          >
                            {opt.label}
                          </button>
                        ))}
                      </div>
                    </div>
                  )}

                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <Label className="text-xs">Organization ID (optional)</Label>
                      <Input
                        className="h-8 text-sm mt-1"
                        placeholder="Filter by org..."
                        value={newOrgId}
                        onChange={(e) => setNewOrgId(e.target.value)}
                      />
                    </div>
                    <div>
                      <Label className="text-xs">Project ID (optional)</Label>
                      <Input
                        className="h-8 text-sm mt-1"
                        placeholder="Filter by project..."
                        value={newProjectId}
                        onChange={(e) => setNewProjectId(e.target.value)}
                      />
                    </div>
                  </div>

                  <div>
                    <Label className="text-xs">Tags (comma-separated, optional)</Label>
                    <Input
                      className="h-8 text-sm mt-1"
                      placeholder="e.g. sales, inbound"
                      value={newTags}
                      onChange={(e) => setNewTags(e.target.value)}
                    />
                  </div>

                  <div>
                    <Label className="text-xs">Model Override (optional)</Label>
                    <Select
                      value={newModelOverride || '__none__'}
                      onValueChange={(v) => setNewModelOverride(v === '__none__' ? '' : v)}
                    >
                      <SelectTrigger className="h-8 text-sm mt-1">
                        <SelectValue placeholder="Use workflow default" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="__none__">Use workflow default</SelectItem>
                        {availableModels.map((m) => (
                          <SelectItem key={m.id} value={m.id}>
                            {m.label}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="flex items-center gap-2">
                    <Switch
                      id="auto-approve"
                      checked={newAutoApprove}
                      onCheckedChange={setNewAutoApprove}
                    />
                    <Label htmlFor="auto-approve" className="text-xs cursor-pointer">
                      Auto-approve valid (non-duplicate) records
                    </Label>
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div>
                      <Label className="text-xs">
                        Cooldown (seconds){' '}
                        <span className="text-muted-foreground">(0 = none)</span>
                      </Label>
                      <Input
                        className="h-8 text-sm mt-1"
                        type="number"
                        min={0}
                        value={newCooldownSeconds}
                        onChange={(e) => setNewCooldownSeconds(Number(e.target.value))}
                      />
                    </div>
                    <div>
                      <Label className="text-xs">
                        Max Retries{' '}
                        <span className="text-muted-foreground">(0 = none)</span>
                      </Label>
                      <Input
                        className="h-8 text-sm mt-1"
                        type="number"
                        min={0}
                        max={10}
                        value={newMaxRetries}
                        onChange={(e) => setNewMaxRetries(Number(e.target.value))}
                      />
                    </div>
                  </div>

                  <DialogFooter className="pt-1">
                    <Button variant="ghost" size="sm" onClick={resetForm}>
                      Cancel
                    </Button>
                    <Button
                      size="sm"
                      onClick={handleCreate}
                      disabled={!newName.trim() || createMutation.isPending}
                    >
                      {createMutation.isPending && (
                        <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
                      )}
                      Create Trigger
                    </Button>
                  </DialogFooter>
                </div>
              )}
            </div>
          )}
        </ScrollArea>

        {/* Footer */}
        <div className="flex justify-between items-center pt-3 border-t">
          <p className="text-[11px] text-muted-foreground">
            Triggers run workflows automatically when matching data sources are created.
            They are disabled by default.
          </p>
          {!showCreateForm && (
            <Button size="sm" variant="outline" onClick={() => setShowCreateForm(true)}>
              <Plus className="h-3.5 w-3.5 mr-1.5" />
              Add Trigger
            </Button>
          )}
        </div>
      </DialogContent>
    </Dialog>
  );
}
