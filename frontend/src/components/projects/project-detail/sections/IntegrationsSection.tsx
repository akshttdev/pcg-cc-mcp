import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { cn } from '@/lib/utils';
import { AlertCircle, Loader2, Sparkles } from 'lucide-react';
import type { IntegrationCategory } from '../types';
import { integrationStatusStyles } from '../types';
import { formatStatusLabel } from '../helpers';

interface IntegrationsSectionProps {
  integrationCategories: IntegrationCategory[];
  emailIntegrationError: string;
  socialIntegrationError: string;
  integrationsRefreshing: boolean;
  onRefresh: () => void;
}

export function IntegrationsSection({
  integrationCategories,
  emailIntegrationError,
  socialIntegrationError,
  integrationsRefreshing,
  onRefresh,
}: IntegrationsSectionProps) {
  return (
    <Card>
      <CardHeader className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <CardTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            Integrations Hub
          </CardTitle>
          <CardDescription>
            Central view of the systems powering this project.
          </CardDescription>
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
        <div className="grid gap-4 lg:grid-cols-2">
          {integrationCategories.map((category) => {
            const Icon = category.icon;
            const connectedCount = category.connectors.filter(
              (connector) => connector.status === 'connected'
            ).length;
            return (
              <div
                key={category.key}
                className="rounded-2xl border bg-background/80 p-4 shadow-sm"
              >
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
          })}
        </div>
      </CardContent>
    </Card>
  );
}
