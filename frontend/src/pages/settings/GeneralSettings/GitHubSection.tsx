import { useCallback } from 'react';
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { FormField } from '@/components/ui/form-field';
import { ChevronDown, Key } from 'lucide-react';
import NiceModal from '@ebay/nice-modal-react';

interface GitHubSectionProps {
  t: (key: string, opts?: Record<string, unknown>) => string;
  config: Record<string, unknown> | null;
  draft: { github: { pat?: string | null; username?: string | null; oauth_token?: string | null; primary_email?: string | null } } | null;
  updateDraft: (patch: Record<string, unknown>) => void;
  updateAndSaveConfig: (patch: Record<string, unknown>) => void;
}

export function GitHubSection({
  t,
  config,
  draft,
  updateDraft,
  updateAndSaveConfig,
}: GitHubSectionProps) {
  const github = config as Record<string, unknown> & { github?: { username?: string | null; oauth_token?: string | null; primary_email?: string | null } };
  const isAuthenticated = !!(
    github?.github?.username && github?.github?.oauth_token
  );

  const handleLogout = useCallback(async () => {
    if (!config) return;
    updateAndSaveConfig({
      github: {
        ...(github?.github || {}),
        oauth_token: null,
        username: null,
        primary_email: null,
      },
    });
  }, [config, github, updateAndSaveConfig]);

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Key className="h-5 w-5" />
          {t('settings.general.github.title')}
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">
        {isAuthenticated ? (
          <div className="space-y-4">
            <div className="flex items-center justify-between p-4 border rounded-lg">
              <div>
                <p className="font-medium">
                  {t('settings.general.github.connected', {
                    username: github?.github?.username,
                  })}
                </p>
                {github?.github?.primary_email && (
                  <p className="text-sm text-muted-foreground">
                    {github.github.primary_email}
                  </p>
                )}
              </div>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="sm">
                    {t('settings.general.github.manage')}{' '}
                    <ChevronDown className="ml-1 h-4 w-4" />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onClick={handleLogout}>
                    {t('settings.general.github.disconnect')}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <p className="text-sm text-muted-foreground">
              {t('settings.general.github.helper')}
            </p>
            <Button
              onClick={() =>
                NiceModal.show('github-login').finally(() =>
                  NiceModal.hide('github-login')
                )
              }
            >
              {t('settings.general.github.connectButton')}
            </Button>
          </div>
        )}

        <div className="flex items-center gap-4 my-6">
          <div className="flex-1 border-t border-border"></div>
          <span className="text-sm text-muted-foreground font-medium">
            {t('settings.general.github.or')}
          </span>
          <div className="flex-1 border-t border-border"></div>
        </div>

        <FormField
          label={t('settings.general.github.pat.label')}
          htmlFor="github-token"
          description={`${t('settings.general.github.pat.helper')}`}
        >
          <Input
            id="github-token"
            type="password"
            placeholder="ghp_xxxxxxxxxxxxxxxxxxxx"
            value={draft?.github.pat || ''}
            onChange={(e) =>
              updateDraft({
                github: {
                  ...draft!.github,
                  pat: e.target.value || null,
                },
              })
            }
          />
          <p className="text-xs text-muted-foreground">
            <a
              href="https://github.com/settings/tokens"
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:underline"
            >
              {t('settings.general.github.pat.createTokenLink')}
            </a>
          </p>
        </FormField>
      </CardContent>
    </Card>
  );
}
