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
import { Bot, Shield, Clock } from 'lucide-react';
import type { StageConfig } from '@/types/crm';

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

export function StageConfigEditor({ config, onChange }: StageConfigEditorProps) {
  const update = (partial: Partial<StageConfig>) => {
    onChange({ ...config, ...partial });
  };

  return (
    <div className="space-y-4">
      <Separator />
      <div className="flex items-center gap-2 text-sm font-medium">
        <Bot className="h-4 w-4 text-primary" />
        Stage Configuration
      </div>

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
