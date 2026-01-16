import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  DollarSign,
  Wrench,
  Brain,
  Sparkles,
  Activity,
} from 'lucide-react';
import type { AgentWithParsedFields } from 'shared/types';

interface AgentDetailDialogProps {
  agent: AgentWithParsedFields | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// LLM pricing at 2x market rate (in USD per 1M tokens)
const MODEL_PRICING: Record<string, { input: number; output: number; display: string }> = {
  'gpt-4o': { input: 5.00, output: 20.00, display: 'GPT-4o' },
  'gpt-4': { input: 60.00, output: 120.00, display: 'GPT-4' },
  'gpt-5': { input: 60.00, output: 120.00, display: 'GPT-5' },
  'gpt-5-codex': { input: 60.00, output: 120.00, display: 'GPT-5 Codex' },
  'claude-sonnet-4': { input: 6.00, output: 30.00, display: 'Claude Sonnet 4' },
  'claude-opus-4': { input: 30.00, output: 150.00, display: 'Claude Opus 4' },
  'gpt-oss': { input: 0.50, output: 2.00, display: 'GPT-OSS (Local)' },
  'gemini-pro': { input: 2.50, output: 10.00, display: 'Gemini Pro' },
};

// VIBE token value
const VIBE_USD_VALUE = 0.001; // 1 VIBE = $0.001

function getModelPricing(modelName: string | null) {
  if (!modelName) return null;

  // Normalize model name for lookup
  const normalized = modelName.toLowerCase().replace(/[-_]/g, '-');

  // Try exact match first
  if (MODEL_PRICING[normalized]) {
    return MODEL_PRICING[normalized];
  }

  // Try partial matches
  for (const [key, pricing] of Object.entries(MODEL_PRICING)) {
    if (normalized.includes(key) || key.includes(normalized)) {
      return pricing;
    }
  }

  // Default pricing for unknown models
  return { input: 5.00, output: 20.00, display: modelName };
}

function calculateVibeEstimate(pricing: { input: number; output: number } | null, avgTokens = 2000) {
  if (!pricing) return null;

  // Assume 70% input, 30% output for typical request
  const inputTokens = avgTokens * 0.7;
  const outputTokens = avgTokens * 0.3;

  const inputCost = (inputTokens / 1_000_000) * pricing.input;
  const outputCost = (outputTokens / 1_000_000) * pricing.output;
  const totalUsd = inputCost + outputCost;

  return Math.ceil(totalUsd / VIBE_USD_VALUE);
}

const statusStyles: Record<string, string> = {
  active: 'bg-emerald-100 text-emerald-700 border-emerald-200',
  inactive: 'bg-gray-100 text-gray-600 border-gray-200',
  maintenance: 'bg-amber-100 text-amber-700 border-amber-200',
  training: 'bg-blue-100 text-blue-700 border-blue-200',
};

export function AgentDetailDialog({ agent, open, onOpenChange }: AgentDetailDialogProps) {
  if (!agent) return null;

  const pricing = getModelPricing(agent.default_model);
  const vibeEstimate = calculateVibeEstimate(pricing);

  const initials = agent.short_name
    .split(' ')
    .map((part) => part[0])
    .join('')
    .slice(0, 2)
    .toUpperCase();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-start gap-4">
            {/* Agent Avatar */}
            <div className="h-20 w-20 rounded-xl overflow-hidden bg-muted shrink-0">
              {agent.avatar_url ? (
                <img
                  src={agent.avatar_url}
                  alt={agent.short_name}
                  className="h-full w-full object-cover"
                />
              ) : (
                <div className="h-full w-full flex items-center justify-center bg-gradient-to-br from-primary/20 to-primary/5">
                  <span className="text-2xl font-bold text-primary/50">{initials}</span>
                </div>
              )}
            </div>

            {/* Agent Title */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <DialogTitle className="text-xl">{agent.short_name}</DialogTitle>
                <Badge
                  variant="outline"
                  className={`text-xs capitalize ${statusStyles[agent.status] || ''}`}
                >
                  {agent.status}
                </Badge>
              </div>
              <p className="text-sm font-medium text-primary">{agent.designation}</p>
              <p className="text-xs text-muted-foreground mt-1">
                Autonomy: {agent.autonomy_level?.replace('_', ' ') || 'manual'}
              </p>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4 mt-4">
          {/* Description */}
          {agent.description && (
            <div>
              <p className="text-sm text-muted-foreground leading-relaxed">
                {agent.description}
              </p>
            </div>
          )}

          <Separator />

          {/* Cost Estimation Card */}
          <Card className="border-primary/20 bg-primary/5">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2">
                <DollarSign className="h-4 w-4" />
                Cost Estimation
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">Default Model</span>
                <Badge variant="secondary">
                  {pricing?.display || agent.default_model || 'Not configured'}
                </Badge>
              </div>

              {pricing && (
                <>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground">Input cost (per 1M tokens)</span>
                    <span className="font-mono">${pricing.input.toFixed(2)}</span>
                  </div>
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground">Output cost (per 1M tokens)</span>
                    <span className="font-mono">${pricing.output.toFixed(2)}</span>
                  </div>
                </>
              )}

              <Separator />

              <div className="flex items-center justify-between">
                <span className="text-sm font-medium">Est. cost per request</span>
                <div className="text-right">
                  <div className="text-lg font-bold text-primary">
                    {vibeEstimate ? `~${vibeEstimate} VIBE` : 'N/A'}
                  </div>
                  {vibeEstimate && (
                    <div className="text-xs text-muted-foreground">
                      ~${(vibeEstimate * VIBE_USD_VALUE).toFixed(4)} USD
                    </div>
                  )}
                </div>
              </div>
              <p className="text-xs text-muted-foreground">
                Based on ~2,000 tokens per request (70% input, 30% output)
              </p>
            </CardContent>
          </Card>

          {/* Personality & Voice */}
          {agent.personality && (
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm flex items-center gap-2">
                  <Brain className="h-4 w-4" />
                  Personality
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                {agent.personality.traits && agent.personality.traits.length > 0 && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-2">Traits</p>
                    <div className="flex flex-wrap gap-1">
                      {agent.personality.traits.map((trait) => (
                        <Badge key={trait} variant="outline" className="text-xs">
                          {trait}
                        </Badge>
                      ))}
                    </div>
                  </div>
                )}

                {agent.personality.communication_style && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-1">Communication Style</p>
                    <p className="text-sm">{agent.personality.communication_style}</p>
                  </div>
                )}

                {agent.personality.emotional_baseline && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-1">Emotional Baseline</p>
                    <p className="text-sm">{agent.personality.emotional_baseline}</p>
                  </div>
                )}
              </CardContent>
            </Card>
          )}

          {/* Capabilities */}
          {agent.capabilities && agent.capabilities.length > 0 && (
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm flex items-center gap-2">
                  <Sparkles className="h-4 w-4" />
                  Capabilities
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex flex-wrap gap-2">
                  {agent.capabilities.map((capability) => (
                    <Badge key={capability} variant="secondary">
                      {capability.replace(/_/g, ' ')}
                    </Badge>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Tools */}
          {agent.tools && agent.tools.length > 0 && (
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm flex items-center gap-2">
                  <Wrench className="h-4 w-4" />
                  Tools
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="flex flex-wrap gap-2">
                  {agent.tools.map((tool) => (
                    <Badge key={tool} variant="outline" className="font-mono text-xs">
                      {tool}
                    </Badge>
                  ))}
                </div>
              </CardContent>
            </Card>
          )}

          {/* Statistics */}
          <Card>
            <CardHeader className="pb-2">
              <CardTitle className="text-sm flex items-center gap-2">
                <Activity className="h-4 w-4" />
                Statistics
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-3 gap-4 text-center">
                <div>
                  <div className="text-2xl font-bold">
                    {agent.tasks_completed?.toString() || '0'}
                  </div>
                  <div className="text-xs text-muted-foreground">Tasks Completed</div>
                </div>
                <div>
                  <div className="text-2xl font-bold">
                    {agent.tasks_failed?.toString() || '0'}
                  </div>
                  <div className="text-xs text-muted-foreground">Tasks Failed</div>
                </div>
                <div>
                  <div className="text-2xl font-bold">
                    {agent.average_rating?.toFixed(1) || '—'}
                  </div>
                  <div className="text-xs text-muted-foreground">Avg Rating</div>
                </div>
              </div>
            </CardContent>
          </Card>
        </div>
      </DialogContent>
    </Dialog>
  );
}
