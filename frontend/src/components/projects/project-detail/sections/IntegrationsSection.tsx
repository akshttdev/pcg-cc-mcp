import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import { cn } from '@/lib/utils';
import { AlertCircle, ChevronDown, ChevronRight, Loader2, Sparkles } from 'lucide-react';
import type { IntegrationCategory } from '../types';
import { integrationStatusStyles, RECOMMENDED_INTEGRATION_KEYS } from '../types';
import { formatStatusLabel } from '../helpers';

interface IntegrationsSectionProps {
  integrationCategories: IntegrationCategory[];
  emailIntegrationError: string;
  socialIntegrationError: string;
  integrationsRefreshing: boolean;
  onRefresh: () => void;
}

function ProgressRing({ connected, total }: { connected: number; total: number }) {
  const pct = total > 0 ? (connected / total) * 100 : 0;
  const radius = 18;
  const circumference = 2 * Math.PI * radius;
  const strokeDashoffset = circumference - (pct / 100) * circumference;

  return (
    <div className="flex items-center gap-2">
      <svg width="44" height="44" viewBox="0 0 44 44" className="-rotate-90">
        <circle cx="22" cy="22" r={radius} fill="none" stroke="currentColor" strokeWidth="3" className="text-muted/30" />
        <circle cx="22" cy="22" r={radius} fill="none" stroke="currentColor" strokeWidth="3" className="text-emerald-500" strokeDasharray={circumference} strokeDashoffset={strokeDashoffset} strokeLinecap="round" />
      </svg>
      <span className="text-sm font-medium text-muted-foreground">
        {connected} of {total} connected
      </span>
    </div>
  );
}

function IntegrationCategoryCard({ category }: { category: IntegrationCategory }) {
  const Icon = category.icon;
  const connectedCount = category.connectors.filter(
    (connector) => connector.status === 'connected'
  ).length;

  return (
    <div className="rounded-2xl border bg-background/80 p-4 shadow-sm">
      <div className="flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <span
            className={cn(
              'flex h-10 w-10 items-center justify-center rounded-full bg-gradient-to-br text-primary',
              category.accent
            )}
          >
            <Icon className="h-5 w-5" />
          </span>
          <div>
            <p className="text-sm font-semibold text-foreground">
              {category.label}
            </p>
            <p className="text-xs text-muted-foreground">
              {connectedCount}/{category.connectors.length} connected
            </p>
          </div>
        </div>
      </div>
      <div className="mt-4 space-y-3">
        {category.connectors.map((connector) => (
          <div
            key={connector.key}
            className="rounded-xl border bg-background/70 p-3 shadow-sm"
          >
            <div className="flex items-center justify-between gap-3">
              <p className="text-sm font-semibold text-foreground">
                {connector.label}
              </p>
              <span
                className={cn(
                  'rounded-full border px-2.5 py-0.5 text-xs font-medium',
                  integrationStatusStyles[connector.status]
                )}
              >
                {formatStatusLabel(connector.status)}
              </span>
            </div>
            <p className="text-xs text-muted-foreground">
              {connector.detail}
            </p>
            {connector.meta && (
              <p className="text-[11px] text-muted-foreground/80">
                {connector.meta}
              </p>
            )}
            {connector.onAction && (
              <div className="pt-3">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={(event) => {
                    event.stopPropagation();
                    connector.onAction?.();
                  }}
                >
                  {connector.actionLabel ?? 'Manage'}
                </Button>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export function IntegrationsSection({
  integrationCategories,
  emailIntegrationError,
  socialIntegrationError,
  integrationsRefreshing,
  onRefresh,
}: IntegrationsSectionProps) {
  const [optionalExpanded, setOptionalExpanded] = useState(false);

  // Split into recommended vs optional
  const recommended = integrationCategories.filter((cat) =>
    cat.connectors.some((c) => RECOMMENDED_INTEGRATION_KEYS.has(c.key))
  );
  const optional = integrationCategories.filter(
    (cat) => !cat.connectors.some((c) => RECOMMENDED_INTEGRATION_KEYS.has(c.key))
  );

  // Total counts
  const allConnectors = integrationCategories.flatMap((c) => c.connectors);
  const totalConnected = allConnectors.filter((c) => c.status === 'connected').length;
  const totalCount = allConnectors.length;

  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-4">
          <div>
            <CardTitle className="flex items-center gap-2">
              <Sparkles className="h-5 w-5" />
              Integrations Hub
            </CardTitle>
            <CardDescription>
              Central view of the systems powering this project.
            </CardDescription>
          </div>
          <ProgressRing connected={totalConnected} total={totalCount} />
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={onRefresh}
          disabled={integrationsRefreshing}
        >
          {integrationsRefreshing && (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          )}
          Refresh status
        </Button>
      </CardHeader>
      <CardContent className="space-y-4">
        {(emailIntegrationError || socialIntegrationError) && (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>
              {emailIntegrationError && <p>{emailIntegrationError}</p>}
              {socialIntegrationError && <p>{socialIntegrationError}</p>}
            </AlertDescription>
          </Alert>
        )}

        {/* Recommended integrations — always visible */}
        {recommended.length > 0 && (
          <div>
            <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-3">
              Recommended
            </p>
            <div className="grid gap-4 lg:grid-cols-2">
              {recommended.map((category) => (
                <IntegrationCategoryCard key={category.key} category={category} />
              ))}
            </div>
          </div>
        )}

        {/* Optional integrations — collapsed by default */}
        {optional.length > 0 && (
          <Collapsible open={optionalExpanded} onOpenChange={setOptionalExpanded}>
            <CollapsibleTrigger asChild>
              <button className="flex items-center gap-2 text-xs font-semibold text-muted-foreground uppercase tracking-wider hover:text-foreground transition-colors w-full py-2">
                {optionalExpanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
                Optional ({optional.length} categories)
              </button>
            </CollapsibleTrigger>
            <CollapsibleContent>
              <div className="grid gap-4 lg:grid-cols-2 mt-2">
                {optional.map((category) => (
                  <IntegrationCategoryCard key={category.key} category={category} />
                ))}
              </div>
            </CollapsibleContent>
          </Collapsible>
        )}
      </CardContent>
    </Card>
  );
}
