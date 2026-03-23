import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Bot, ShieldCheck, Zap } from 'lucide-react';
import type { CrmPipelineStage, StageConfig, StageAction, StageValidation } from '@/types/crm';

function parseStageConfigSafe(json?: string): StageConfig | null {
  if (!json) return null;
  try { return JSON.parse(json) as StageConfig; } catch { return null; }
}

function formatStageAction(action: StageAction): string {
  switch (action.type) {
    case 'trigger_agent': return `Trigger ${action.agent} (${action.flow_type})`;
    case 'create_review_task': return `Review task: "${action.description}"`;
    case 'create_delivery_deal': return 'Auto-create delivery deal';
    default: return JSON.stringify(action);
  }
}

function formatStageValidation(v: StageValidation): string {
  switch (v.type) {
    case 'require_field': return `Require ${v.field}`;
    case 'require_intel': return `${v.entity} intel = "${v.status}"`;
    case 'require_pending_tasks': return `Max ${v.count} pending tasks`;
    default: return JSON.stringify(v);
  }
}

interface PipelineSettingsAutomationsTabProps {
  stages: CrmPipelineStage[];
}

export function PipelineSettingsAutomationsTab({ stages }: PipelineSettingsAutomationsTabProps) {
  return (
    <ScrollArea className="h-[480px]">
      <div className="space-y-4 p-4">
        {stages.map((stage) => {
          const config = parseStageConfigSafe(stage.stage_config);
          if (!config) return (
            <div key={stage.id} className="rounded-lg border bg-card px-4 py-2.5">
              <div className="flex items-center gap-2">
                <span className="inline-flex h-2.5 w-2.5 rounded-full" style={{ backgroundColor: stage.color }} />
                <p className="font-medium text-sm">{stage.name}</p>
                <Badge variant="outline" className="text-xs text-muted-foreground">No config</Badge>
              </div>
            </div>
          );
          const owner = config.stage_owner;
          const actions = config.on_enter_actions ?? [];
          const validations = config.on_exit_validations ?? [];
          const hasAgent = !!config.assigned_agent;
          return (
            <div key={stage.id} className="rounded-lg border bg-card px-4 py-3 space-y-2">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <span className="inline-flex h-2.5 w-2.5 rounded-full" style={{ backgroundColor: stage.color }} />
                  <p className="font-medium text-sm">{stage.name}</p>
                  <Badge variant="secondary" className="text-xs">{stage.probability}%</Badge>
                </div>
                {owner && (
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Badge variant="outline" className="text-xs">{owner.type}</Badge>
                    {owner.label}
                  </div>
                )}
              </div>
              {/* Agent + trigger */}
              {hasAgent && (
                <div className="flex items-center gap-2 text-xs">
                  <Bot className="h-3 w-3 text-primary" />
                  <span className="font-medium capitalize">{config.assigned_agent}</span>
                  {config.auto_trigger ? (
                    <Badge className="text-[9px] bg-blue-500/15 text-blue-600 border-blue-200 dark:text-blue-400 dark:border-blue-800">auto-trigger {config.cancel_window_secs}s</Badge>
                  ) : (
                    <Badge variant="outline" className="text-[9px]">manual</Badge>
                  )}
                </div>
              )}
              {/* On enter actions */}
              {actions.length > 0 && (
                <div className="space-y-1">
                  {actions.map((action, i) => (
                    <div key={i} className="flex items-start gap-1.5 text-xs text-muted-foreground">
                      <Zap className="h-3 w-3 text-blue-500 mt-0.5 shrink-0" />
                      <span>{formatStageAction(action)}</span>
                    </div>
                  ))}
                </div>
              )}
              {/* Exit validations */}
              {validations.length > 0 && (
                <div className="space-y-1">
                  {validations.map((v, i) => (
                    <div key={i} className="flex items-start gap-1.5 text-xs text-muted-foreground">
                      <ShieldCheck className="h-3 w-3 text-amber-500 mt-0.5 shrink-0" />
                      <span>{formatStageValidation(v)}</span>
                    </div>
                  ))}
                </div>
              )}
              {/* Required fields */}
              {(config.required_fields ?? []).length > 0 && (
                <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                  <ShieldCheck className="h-3 w-3 text-amber-500 shrink-0" />
                  <span>Required: {config.required_fields!.join(', ')}</span>
                </div>
              )}
              {/* No automations */}
              {!hasAgent && actions.length === 0 && validations.length === 0 && (config.required_fields ?? []).length === 0 && (
                <p className="text-xs text-muted-foreground/50 italic">No automations configured</p>
              )}
            </div>
          );
        })}
      </div>
    </ScrollArea>
  );
}
