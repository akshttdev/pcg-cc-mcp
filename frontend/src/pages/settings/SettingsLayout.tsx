import {
  Activity,
  ArrowLeft,
  Bot,
  Boxes,
  Building2,
  Cloud,
  Code2,
  Cpu,
  CreditCard,
  FolderKanban,
  HardDrive,
  Key,
  Layout,
  Network,
  Palette,
  Plug,
  Server,
  Settings,
  Shield,
  User,
  Users,
  Wallet,
} from 'lucide-react';
import { useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { NavLink, Outlet, useSearchParams } from 'react-router-dom';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/contexts/AuthContext';
import { useEffectiveRole } from '@/hooks/useEffectiveRole';
import { usePreviousPath } from '@/hooks/usePreviousPath';
import {
  ROLE_LEVEL,
  SETTINGS_SCOPE_LABELS,
  type SettingsScope,
} from '@/lib/roles';
import { cn } from '@/lib/utils';

interface SettingsNavItem {
  path: string;
  icon: typeof Settings;
  label: string;
  description: string;
  adminOnly?: boolean;
  absolutePath?: string;
  scopes: SettingsScope[];
  planned?: boolean;
}

const settingsNavigation: SettingsNavItem[] = [
  // ─── User scope ──────────────────────────────────────────────
  {
    path: 'general',
    icon: Settings,
    label: 'General',
    description: 'Theme, notifications, and preferences',
    scopes: ['user', 'system'],
  },
  {
    path: 'wallet',
    icon: Wallet,
    label: 'Wallet',
    description: 'Token balances & usage history',
    scopes: ['user', 'system'],
  },
  {
    path: 'profile',
    icon: User,
    label: 'Profile',
    description: 'Manage your personal information',
    scopes: ['user', 'system'],
  },
  {
    path: 'privacy',
    icon: Shield,
    label: 'Privacy & Security',
    description: 'Control your privacy and security settings',
    scopes: ['user', 'system'],
  },
  {
    path: 'activity',
    icon: Activity,
    label: 'Activity Log',
    description: 'View your recent account activity',
    scopes: ['user', 'system'],
  },
  {
    path: 'keys',
    icon: Key,
    label: 'API Keys',
    description: 'LLM provider API keys',
    scopes: ['user', 'org', 'system'],
  },
  {
    path: 'topsi-preferences',
    icon: Bot,
    label: 'Topsi Preferences',
    description: 'Tool confirmation and autonomy',
    scopes: ['user'],
  },
  // ─── System Admin scope ──────────────────────────────────────
  {
    path: 'users',
    icon: Users,
    label: 'Users',
    description: 'Manage team members and permissions',
    adminOnly: true,
    scopes: ['system'],
  },
  {
    path: 'organizations',
    icon: Building2,
    label: 'Organizations',
    description: 'Manage all organizations',
    adminOnly: true,
    scopes: ['system'],
  },
  {
    path: 'projects',
    icon: FolderKanban,
    label: 'Projects',
    description: 'Manage project access and permissions',
    adminOnly: true,
    scopes: ['system'],
  },
  {
    path: 'topsi',
    icon: Bot,
    label: 'Topsi',
    description: 'System prompt and agent configuration',
    adminOnly: true,
    scopes: ['system'],
  },
  // ─── Org scope ───────────────────────────────────────────────
  {
    path: 'agents',
    icon: Cpu,
    label: 'Agents',
    description: 'Autonomous agents and budgets',
    scopes: ['org', 'system'],
  },
  {
    path: 'models',
    icon: Boxes,
    label: 'Models',
    description: 'AI model configurations',
    scopes: ['org', 'system'],
  },
  {
    path: 'mcp',
    icon: Server,
    label: 'MCP Servers',
    description: 'Model Context Protocol servers',
    scopes: ['org', 'system'],
  },
  {
    path: 'network',
    icon: Network,
    label: 'Network & Mesh',
    description: 'APN identity, mesh monitoring, and capabilities',
    scopes: ['org', 'system'],
  },
  {
    path: 'apn-drive',
    icon: HardDrive,
    label: 'APN Drive',
    description:
      'Mount media files in Finder — open XML sequences directly in Premiere',
    scopes: ['user', 'org', 'system'],
  },
  {
    path: 'storage',
    icon: Cloud,
    label: 'Cloud Storage',
    description: 'OneDrive, Dropbox, and Google Drive sync',
    scopes: ['org', 'system'],
  },
  // ─── Client scope ────────────────────────────────────────────
  {
    path: 'pulse',
    icon: Activity,
    label: 'Pulse Engine',
    description: 'System health and performance metrics',
    absolutePath: '/pulse',
    scopes: ['client', 'system'],
  },
  // ─── Planned items ───────────────────────────────────────────
  {
    path: 'integrations-personal',
    icon: Plug,
    label: 'Integrations',
    description: 'Personal developer tools (GitHub, etc.)',
    scopes: ['user'],
    planned: true,
  },
  {
    path: 'integrations-org',
    icon: Plug,
    label: 'Integrations',
    description: 'Organization-level service connections',
    scopes: ['org'],
    planned: true,
  },
  {
    path: 'billing-org',
    icon: CreditCard,
    label: 'Billing & Usage',
    description: 'Organization billing and usage tracking',
    scopes: ['org'],
    planned: true,
  },
  {
    path: 'integrations-client',
    icon: Plug,
    label: 'Integrations',
    description: 'Client-facing tool connections (socials, etc.)',
    scopes: ['client'],
    planned: true,
  },
  {
    path: 'branding',
    icon: Palette,
    label: 'Branding',
    description: 'Client branding and white-label settings',
    scopes: ['client'],
    planned: true,
  },
  {
    path: 'client-portal',
    icon: Layout,
    label: 'Client Portal',
    description: 'Client portal configuration',
    scopes: ['client'],
    planned: true,
  },
  // ─── Dev-only ────────────────────────────────────────────────
  ...(import.meta.env.MODE === 'development'
    ? [
        {
          path: 'developer',
          icon: Code2,
          label: 'Developer',
          description: 'Development-only tools and bypasses',
          adminOnly: true,
          scopes: ['system'] as SettingsScope[],
        },
      ]
    : []),
];

export function SettingsLayout() {
  const { t } = useTranslation('settings');
  const goToPreviousPath = usePreviousPath();
  const { user } = useAuth();
  const { role, canSeeAdminPlatforms, canManageOrg } = useEffectiveRole();
  const [searchParams, setSearchParams] = useSearchParams();

  const level = ROLE_LEVEL[role];

  // Determine which tabs are visible
  const visibleTabs: { scope: SettingsScope; label: string }[] = [
    { scope: 'user', label: SETTINGS_SCOPE_LABELS.user },
    ...(canSeeAdminPlatforms
      ? [{ scope: 'system' as const, label: SETTINGS_SCOPE_LABELS.system }]
      : []),
    ...(canManageOrg || level >= ROLE_LEVEL.org_editor
      ? [{ scope: 'org' as const, label: SETTINGS_SCOPE_LABELS.org }]
      : []),
    ...(level >= ROLE_LEVEL.client_editor || canSeeAdminPlatforms
      ? [{ scope: 'client' as const, label: SETTINGS_SCOPE_LABELS.client }]
      : []),
  ];

  // Read scope from URL param, validate against visible tabs (permission guard)
  const urlScope = searchParams.get('scope') as SettingsScope | null;
  const effectiveScope =
    urlScope && visibleTabs.some((t) => t.scope === urlScope)
      ? urlScope
      : (visibleTabs[0]?.scope ?? 'user');

  const setActiveScope = useCallback(
    (scope: SettingsScope) => {
      setSearchParams(
        (prev) => {
          const next = new URLSearchParams(prev);
          if (scope === 'user') {
            next.delete('scope');
          } else {
            next.set('scope', scope);
          }
          return next;
        },
        { replace: true }
      );
    },
    [setSearchParams]
  );

  // Filter navigation items by active scope and admin status
  const allVisible = settingsNavigation.filter((item) => {
    if (item.adminOnly && !user?.is_admin) return false;
    return item.scopes.includes(effectiveScope);
  });
  const visibleNavigation = allVisible.filter((item) => !item.planned);
  const plannedNavigation = allVisible.filter((item) => item.planned);

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="flex flex-col lg:flex-row gap-8">
        {/* Sidebar Navigation */}
        <aside className="w-full lg:w-64 lg:shrink-0 lg:sticky lg:top-8 lg:h-fit lg:max-h-[calc(100vh-4rem)] lg:overflow-y-auto">
          <div className="space-y-1">
            <Button variant="ghost" onClick={goToPreviousPath} className="mb-4">
              <ArrowLeft className="mr-2 h-4 w-4" />
              {t('settings.layout.nav.backToApp')}
            </Button>
            <h2 className="px-3 py-2 text-lg font-semibold">
              {t('settings.layout.nav.title')}
            </h2>

            {/* Scope tabs */}
            {visibleTabs.length > 1 && (
              <div className="flex flex-wrap gap-1 px-1 pb-2">
                {visibleTabs.map(({ scope, label }) => (
                  <button
                    key={scope}
                    onClick={() => setActiveScope(scope)}
                    className={cn(
                      'px-2.5 py-1 text-xs font-medium rounded-md transition-colors whitespace-nowrap',
                      effectiveScope === scope
                        ? 'bg-primary/10 text-primary'
                        : 'text-muted-foreground hover:text-foreground hover:bg-accent/60'
                    )}
                  >
                    {label}
                  </button>
                ))}
              </div>
            )}

            <nav className="space-y-1">
              {visibleNavigation.map((item) => {
                const Icon = item.icon;
                return (
                  <NavLink
                    key={item.path}
                    to={item.absolutePath ?? item.path}
                    end
                    className={({ isActive }) =>
                      cn(
                        'flex items-start gap-3 px-3 py-2 text-sm rounded-lg transition-colors',
                        'hover:bg-accent/60 hover:text-accent-foreground',
                        isActive
                          ? 'bg-primary/[0.08] text-foreground font-medium'
                          : 'text-muted-foreground'
                      )
                    }
                  >
                    <Icon className="h-4 w-4 mt-0.5 shrink-0" />
                    <div className="flex-1 min-w-0">
                      <div className="font-medium">
                        {t(`settings.layout.nav.${item.path}`, {
                          defaultValue: item.label,
                        })}
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {t(`settings.layout.nav.${item.path}Desc`, {
                          defaultValue: item.description,
                        })}
                      </div>
                    </div>
                  </NavLink>
                );
              })}

              {plannedNavigation.length > 0 && (
                <>
                  <div className="pt-3 pb-1 px-3">
                    <div className="border-t border-border/40" />
                    <p className="text-xs font-medium text-muted-foreground/40 uppercase tracking-wider mt-2">
                      Planned
                    </p>
                  </div>
                  {plannedNavigation.map((item) => {
                    const Icon = item.icon;
                    return (
                      <div
                        key={item.path}
                        className="flex items-start gap-3 px-3 py-2 text-sm rounded-lg text-muted-foreground/40 cursor-default opacity-40"
                      >
                        <Icon className="h-4 w-4 mt-0.5 shrink-0" />
                        <div className="flex-1 min-w-0">
                          <div className="font-medium flex items-center gap-2">
                            {item.label}
                            <Badge
                              variant="outline"
                              className="text-[9px] px-1 py-0 font-normal text-muted-foreground/40 border-muted-foreground/20"
                            >
                              Coming soon
                            </Badge>
                          </div>
                          <div className="text-xs text-muted-foreground/30">
                            {item.description}
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </>
              )}
            </nav>
          </div>
        </aside>

        {/* Main Content */}
        <main className="flex-1 min-w-0">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
