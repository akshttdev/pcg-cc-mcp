import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Separator } from '@/components/ui/separator';
import { Badge } from '@/components/ui/badge';
import { Shield, Clock, Zap, ShieldCheck, Info } from 'lucide-react';
import type { StageConfig, StageAction, StageValidation } from '@/types/crm';

// ── Available agents (hardcoded for v1, will come from DB later) ─────────

const AVAILABLE_AGENTS = [
  { value: 'scout', label: 'Scout (Research)' },
  { value: 'astra', label: 'Astra (Analysis)' },
  { value: 'cash', label: 'Cash (Proposals)' },
  { value: 'lux', label: 'Lux (Presentations)' },
  { value: 'nora', label: 'Nora (Assistant)' },
] as const;

const DEAL_FIELDS = [
  { value: 'description', label: 'Description (operator context)' },
  { value: 'amount', label: 'Deal Amount' },
  { value: 'crm_contact_id', label: 'Contact' },
  { value: 'proposal_text', label: 'Proposal' },
  { value: 'deck_url', label: 'Deck' },
] as const;

// ── Props ────────────────────────────────────────────────────────────────────

interface StageConfigEditorProps {
  config: Partial<StageConfig>;
  onChange: (config: Partial<StageConfig>) => void;
}

// ── Component ────────────────────────────────────────────────────────────────

/** Editable automation controls (agent, trigger, gates). No read-only section. */
export function StageConfigEditor({ config, onChange }: StageConfigEditorProps) {
  const update = (partial: Partial<StageConfig>) => {
    onChange({ ...config, ...partial });
  };

  return (
    <div className="space-y-4">
      {/* Assigned Agent */}
      <div className="space-y-2">
        <Label>Assigned Agent</Label>
        <Select
          value={config.assigned_agent ?? 'none'}
          onValueChange={(v) => update({ assigned_agent: v === 'none' ? undefined : v })}
        >
          <SelectTrigger>
            <SelectValue placeholder="No agent assigned" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">No agent</SelectItem>
            {AVAILABLE_AGENTS.map((agent) => (
              <SelectItem key={agent.value} value={agent.value}>
                {agent.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      {/* Auto-trigger + Cancel window */}
      {config.assigned_agent && (
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <Checkbox
              id="auto-trigger"
              checked={config.auto_trigger ?? false}
              onCheckedChange={(checked) => update({ auto_trigger: checked === true })}
            />
            <Label htmlFor="auto-trigger" className="text-sm">
              Auto-start agent when deal enters this stage
            </Label>
          </div>

          {config.auto_trigger && (
            <div className="space-y-2 ml-6">
              <Label className="flex items-center gap-1.5 text-xs text-muted-foreground">
                <Clock className="h-3 w-3" />
                Cancel window (seconds)
              </Label>
              <Input
                type="number"
                min={0}
                max={300}
                value={config.cancel_window_secs ?? 30}
                onChange={(e) => update({ cancel_window_secs: Number(e.target.value) || 30 })}
                className="w-24"
              />
            </div>
          )}
        </div>
      )}

      {/* Required Fields (gate) */}
      <div className="space-y-2">
        <Label className="flex items-center gap-1.5">
          <Shield className="h-3.5 w-3.5 text-amber-500" />
          Required Fields (soft gate)
        </Label>
        <div className="space-y-1.5 ml-1">
          {DEAL_FIELDS.map((field) => {
            const isChecked = (config.required_fields ?? []).includes(field.value);
            return (
              <div key={field.value} className="flex items-center gap-2">
                <Checkbox
                  id={`req-${field.value}`}
                  checked={isChecked}
                  onCheckedChange={(checked) => {
                    const current = config.required_fields ?? [];
                    const updated = checked
                      ? [...current, field.value]
                      : current.filter((f) => f !== field.value);
                    update({ required_fields: updated });
                  }}
                />
                <Label htmlFor={`req-${field.value}`} className="text-sm font-normal">
                  {field.label}
                </Label>
              </div>
            );
          })}
        </div>
      </div>

      {/* Approval Gate */}
      <div className="flex items-center gap-3">
        <Checkbox
          id="approval-gate"
          checked={config.approval_gate ?? false}
          onCheckedChange={(checked) => update({ approval_gate: checked === true })}
        />
        <Label htmlFor="approval-gate" className="text-sm">
          Require approval before deal can leave this stage
        </Label>
      </div>
    </div>
  );
}

/** Read-only display of active on_enter_actions, on_exit_validations, stage_owner. */
export function StageActiveRules({ config }: { config: Partial<StageConfig> }) {
  return (
    <ActiveAutomations
      onEnterActions={config.on_enter_actions}
      onExitValidations={config.on_exit_validations}
      stageOwner={config.stage_owner}
    />
  );
}

// ── Read-only summary of existing automations from DB ─────────────────────

function formatAction(action: StageAction): string {
  switch (action.type) {
    case 'trigger_agent':
      return `Trigger ${action.agent} agent (${action.flow_type})`;
    case 'create_review_task':
      return `Create review task: "${action.description}"`;
    case 'create_delivery_deal':
      return 'Auto-create delivery pipeline deal';
    default:
      return JSON.stringify(action);
  }
}

function formatValidation(v: StageValidation): string {
  switch (v.type) {
    case 'require_field':
      return `Require: ${v.field} — ${v.message}`;
    case 'require_intel':
      return `Require ${v.entity} intelligence status = "${v.status}"`;
    case 'require_pending_tasks':
      return `Max ${v.count} pending task(s) allowed`;
    default:
      return JSON.stringify(v);
  }
}

function ActiveAutomations({
  onEnterActions,
  onExitValidations,
  stageOwner,
}: {
  onEnterActions?: StageAction[];
  onExitValidations?: StageValidation[];
  stageOwner?: { label: string; type: string };
}) {
  const hasActions = onEnterActions && onEnterActions.length > 0;
  const hasValidations = onExitValidations && onExitValidations.length > 0;
  const hasOwner = stageOwner && stageOwner.label;

  if (!hasActions && !hasValidations && !hasOwner) return null;

  return (
    <>
      <Separator />
      <div className="flex items-center gap-2 text-sm font-medium">
        <Info className="h-4 w-4 text-muted-foreground" />
        Active Automations
        <Badge variant="secondary" className="text-[10px]">read-only</Badge>
      </div>
      <p className="text-[11px] text-muted-foreground -mt-2">
        These are configured via stage data and control pipeline behavior.
        Edit simple settings above; advanced rules require database changes.
      </p>

      {hasOwner && (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span className="font-medium">Stage Owner:</span>
          <Badge variant="outline" className="text-[10px]">{stageOwner!.type}</Badge>
          {stageOwner!.label}
        </div>
      )}

      {hasActions && (
        <div className="space-y-1.5">
          <Label className="flex items-center gap-1.5 text-xs">
            <Zap className="h-3 w-3 text-blue-500" />
            On Enter Actions
          </Label>
          <div className="space-y-1 ml-1">
            {onEnterActions!.map((action, i) => (
              <div key={i} className="flex items-start gap-1.5 text-[11px] text-muted-foreground bg-muted/50 rounded px-2 py-1">
                <span className="text-blue-500 mt-0.5">→</span>
                <span>{formatAction(action)}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {hasValidations && (
        <div className="space-y-1.5">
          <Label className="flex items-center gap-1.5 text-xs">
            <ShieldCheck className="h-3 w-3 text-amber-500" />
            Exit Validations (soft gate)
          </Label>
          <div className="space-y-1 ml-1">
            {onExitValidations!.map((v, i) => (
              <div key={i} className="flex items-start gap-1.5 text-[11px] text-muted-foreground bg-muted/50 rounded px-2 py-1">
                <span className="text-amber-500 mt-0.5">⚠</span>
                <span>{formatValidation(v)}</span>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}
