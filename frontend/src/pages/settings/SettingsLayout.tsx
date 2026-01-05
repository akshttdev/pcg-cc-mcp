import { NavLink, Outlet } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Settings, Cpu, Server, ArrowLeft, User, Shield, Activity, Wallet, Users, FolderKanban, Table2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { usePreviousPath } from '@/hooks/usePreviousPath';
import { useAuth } from '@/contexts/AuthContext';

const settingsNavigation = [
  {
    path: 'general',
    icon: Settings,
    label: 'General',
    description: 'Theme, notifications, and preferences',
  },
  {
    path: 'wallet',
    icon: Wallet,
    label: 'Wallet',
    description: 'Token balances & usage history',
  },
  {
    path: 'profile',
    icon: User,
    label: 'Profile',
    description: 'Manage your personal information',
  },
  {
    path: 'users',
    icon: Users,
    label: 'Users',
    description: 'Manage team members and permissions',
    adminOnly: true,
  },
  {
    path: 'projects',
    icon: FolderKanban,
    label: 'Projects',
    description: 'Manage project access and permissions',
    adminOnly: true,
  },
  {
    path: 'privacy',
    icon: Shield,
    label: 'Privacy & Security',
    description: 'Control your privacy and security settings',
  },
  {
    path: 'activity',
    icon: Activity,
    label: 'Activity Log',
    description: 'View your recent account activity',
  },
  {
    path: 'agents',
    icon: Cpu,
    label: 'Agents',
    description: 'Coding agent configurations',
  },
  {
    path: 'mcp',
    icon: Server,
    label: 'MCP Servers',
    description: 'Model Context Protocol servers',
  },
  {
    path: 'airtable',
    icon: Table2,
    label: 'Airtable',
    description: 'Connect Airtable bases for task sync',
  },
];

export function SettingsLayout() {
  const { t } = useTranslation('settings');
  const goToPreviousPath = usePreviousPath();
  const { user } = useAuth();

  // Filter navigation items based on admin status
  const visibleNavigation = settingsNavigation.filter(
    (item) => !item.adminOnly || user?.is_admin
  );

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
            <nav className="space-y-1">
              {visibleNavigation.map((item) => {
                const Icon = item.icon;
                return (
                  <NavLink
                    key={item.path}
                    to={item.path}
                    end
                    className={({ isActive }) =>
                      cn(
                        'flex items-start gap-3 px-3 py-2 text-sm transition-colors',
                        'hover:text-accent-foreground',
                        isActive
                          ? 'text-primary-foreground'
                          : 'text-secondary-foreground'
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
