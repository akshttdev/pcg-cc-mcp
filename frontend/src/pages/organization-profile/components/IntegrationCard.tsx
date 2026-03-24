import type React from 'react';
import { Card } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { CheckCircle2, AlertCircle } from 'lucide-react';

export function IntegrationCard({
  accent,
  icon: Icon,
  name,
  description,
  status,
  statusLabel,
  actions,
  extra,
}: {
  accent: string;
  icon: React.ElementType;
  name: string;
  description: string;
  status: 'connected' | 'warning' | 'disconnected';
  statusLabel?: string;
  actions: React.ReactNode;
  extra?: React.ReactNode;
}) {
  const barColor = status === 'connected' ? '#22c55e' : status === 'warning' ? '#f59e0b' : '#94a3b830';
  return (
    <Card className="border-border/60 bg-card/80 overflow-hidden">
      <div className="flex items-stretch">
        <div className="w-1 shrink-0" style={{ backgroundColor: barColor }} />
        <div className="flex-1 p-4 space-y-3">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-3">
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl" style={{ backgroundColor: `${accent}18`, border: `1px solid ${accent}30` }}>
                <Icon className="h-5 w-5" style={{ color: accent }} />
              </div>
              <div>
                <div className="flex items-center gap-2">
                  <span className="text-sm font-semibold">{name}</span>
                  {status === 'connected' ? (
                    <Badge className="text-xs px-1.5 py-0 bg-emerald-100 text-emerald-700 border-emerald-200">
                      <CheckCircle2 className="h-3 w-3 mr-1" />{statusLabel ?? 'Connected'}
                    </Badge>
                  ) : status === 'warning' ? (
                    <Badge className="text-xs px-1.5 py-0 bg-amber-100 text-amber-700 border-amber-200">
                      <AlertCircle className="h-3 w-3 mr-1" />{statusLabel ?? 'Reauthorize'}
                    </Badge>
                  ) : (
                    <Badge variant="secondary" className="text-xs px-1.5 py-0">Not connected</Badge>
                  )}
                </div>
                <p className="text-xs text-muted-foreground mt-0.5">{description}</p>
              </div>
            </div>
            <div className="flex items-center gap-2 shrink-0">{actions}</div>
          </div>
          {extra}
        </div>
      </div>
    </Card>
  );
}
