import { ChevronRight, Home, ChevronsUpDown, Minimize2, Maximize2 } from 'lucide-react';
import { Link, useParams, useNavigate, useLocation } from 'react-router-dom';
import { useProject } from '@/contexts/project-context';
import { useAuth } from '@/contexts/AuthContext';
import { useQuery } from '@tanstack/react-query';
import { tasksApi, organizationsApi, dataSourcesApi, personsApi, companiesApi } from '@/lib/api';
import { cn } from '@/lib/utils';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { useState, useMemo } from 'react';
import { Input } from '@/components/ui/input';
import { useViewStore } from '@/stores/useViewStore';

interface BreadcrumbItem {
  label: string;
  href: string;
}

interface BreadcrumbNavProps {
  /** Task-specific fullscreen toggle (URL-based, used on task detail pages) */
  onToggleFullscreen?: (fullscreen: boolean) => void;
  isFullscreen?: boolean;
}

export function BreadcrumbNav({ onToggleFullscreen, isFullscreen }: BreadcrumbNavProps = {}) {
  const { contentFullscreen, toggleContentFullscreen } = useViewStore();
  const { projectId, taskId, orgId, dataSourceId, clientId, personId, companyId } = useParams<{
    projectId?: string;
    taskId?: string;
    orgId?: string;
    dataSourceId?: string;
    clientId?: string;
    personId?: string;
    companyId?: string;
  }>();
  const { project } = useProject();
  const { user } = useAuth();
  const navigate = useNavigate();
  const location = useLocation();

  // Fetch sidebar tree for org/project switching
  const { data: sidebarTree, isLoading: isSidebarLoading } = useQuery({
    queryKey: ['sidebarTree'],
    queryFn: () => organizationsApi.getSidebarTree(),
    staleTime: 5 * 60 * 1000,
  });

  // Fetch task if taskId is present
  const { data: task } = useQuery({
    queryKey: ['task', taskId],
    queryFn: async () => {
      if (!taskId || !projectId) return null;
      const tasks = await tasksApi.getAll(projectId);
      return tasks.find((t) => t.id === taskId) || null;
    },
    enabled: !!taskId && !!projectId,
  });

  // Fetch data source if dataSourceId is present
  const { data: dataSource } = useQuery({
    queryKey: ['dataSource', dataSourceId],
    queryFn: () => dataSourcesApi.get(dataSourceId!),
    enabled: !!dataSourceId,
    staleTime: 5 * 60 * 1000,
  });

  // Fetch person if personId is present
  const { data: person } = useQuery({
    queryKey: ['person', personId],
    queryFn: () => personsApi.get(personId!),
    enabled: !!personId,
    staleTime: 5 * 60 * 1000,
  });

  // Fetch company if companyId is present
  const { data: company } = useQuery({
    queryKey: ['company', companyId],
    queryFn: () => companiesApi.get(companyId!),
    enabled: !!companyId,
    staleTime: 5 * 60 * 1000,
  });

  // Fetch client name from org members if clientId is present
  const { data: clientData } = useQuery({
    queryKey: ['org-clients', orgId],
    queryFn: () => organizationsApi.getClients(orgId!),
    enabled: !!clientId && !!orgId,
    staleTime: 5 * 60 * 1000,
  });
  const clientName = clientData?.find((c: any) => c.id === clientId)?.name;

  // Derive current org from route, project, or fall back to user's first org
  const allOrgs = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])]
    : [];

  const currentOrg = allOrgs.find((org) => {
    if (orgId) return org.id === orgId;
    if (project && projectId) {
      const findProject = (projects: any[]): boolean =>
        projects.some((p: any) => p.id === projectId || findProject(p.children || []));
      return (
        findProject(org.internal_projects) ||
        org.clients.some((c: any) => findProject(c.projects))
      );
    }
    return false;
  })
    // Fall back to user's home org, then first org from auth, then first from sidebar
    ?? allOrgs.find((org) => user?.home_organization_id && org.id === user.home_organization_id)
    ?? allOrgs.find((org) => user?.organizations?.[0] && org.id === user.organizations[0].id)
    ?? (allOrgs.length > 0 ? allOrgs[0] : undefined);

  // Build breadcrumb items — always start with org
  const items: BreadcrumbItem[] = [];

  if (currentOrg && currentOrg.id) {
    items.push({
      label: currentOrg.name,
      href: `/organizations/${currentOrg.id}`,
    });
  }

  if (project && projectId) {
    items.push({
      label: project.name,
      href: `/projects/${projectId}`,
    });
  }

  if (project && projectId && (taskId || location.pathname.includes('/tasks'))) {
    items.push({
      label: 'Tasks',
      href: `/projects/${projectId}/tasks`,
    });
  }

  if (task) {
    items.push({
      label: task.title.length > 50 ? `${task.title.substring(0, 50)}...` : task.title,
      href: `/projects/${projectId}/tasks/${taskId}`,
    });
  }

  // Project sub-pages
  if (project && projectId && !taskId) {
    const projectBase = `/projects/${projectId}`;
    if (location.pathname.includes('/knowledge')) {
      items.push({ label: 'Knowledge', href: `${projectBase}/knowledge` });
    } else if (location.pathname.includes('/deliverables')) {
      items.push({ label: 'Deliverables', href: `${projectBase}/deliverables` });
    } else if (location.pathname.includes('/control')) {
      items.push({ label: 'Controller', href: `${projectBase}/control` });
    } else if (location.pathname.includes('/pulse')) {
      items.push({ label: 'Pulse', href: `${projectBase}/pulse` });
    } else if (location.pathname.includes('/media')) {
      items.push({ label: 'Media Library', href: `${projectBase}/media` });
    } else if (location.pathname.includes('/social')) {
      items.push({ label: 'Social', href: `${projectBase}/social` });
    } else if (location.pathname.includes('/crm')) {
      items.push({ label: 'CRM', href: `${projectBase}/crm` });
      if (location.pathname.includes('/crm/overview')) {
        items.push({ label: 'Overview', href: `${projectBase}/crm/overview` });
      } else if (location.pathname.includes('/crm/sales')) {
        items.push({ label: 'Sales', href: `${projectBase}/crm/sales` });
      } else if (location.pathname.includes('/crm/delivery')) {
        items.push({ label: 'Delivery', href: `${projectBase}/crm/delivery` });
      } else if (location.pathname.includes('/crm/clients')) {
        items.push({ label: 'Clients', href: `${projectBase}/crm/clients` });
      } else if (location.pathname.includes('/crm/conferences')) {
        items.push({ label: 'Conferences', href: `${projectBase}/crm/conferences` });
      }
    }
  }

  // Organization sub-pages
  if (currentOrg && orgId && !projectId) {
    if (location.pathname.includes('/data-sources/')) {
      items.push({
        label: 'Intelligence',
        href: `/organizations/${orgId}/intelligence`,
      });
      items.push({
        label: 'Data Sources',
        href: `/organizations/${orgId}/intelligence/data-sources`,
      });
      if (dataSource) {
        const title = dataSource.title.length > 50
          ? `${dataSource.title.substring(0, 50)}...`
          : dataSource.title;
        items.push({
          label: title,
          href: `/organizations/${orgId}/data-sources/${dataSourceId}`,
        });
      }
    } else if (location.pathname.includes('/crm')) {
      items.push({ label: 'CRM', href: `/organizations/${orgId}/crm` });
      if (location.pathname.includes('/crm/contacts')) {
        items.push({ label: 'Contacts', href: `/organizations/${orgId}/crm/contacts` });
      } else if (location.pathname.includes('/crm/companies')) {
        items.push({ label: 'Companies', href: `/organizations/${orgId}/crm/companies` });
      } else if (location.pathname.includes('/crm/pipeline')) {
        items.push({ label: 'Pipeline', href: `/organizations/${orgId}/crm/pipeline` });
      } else if (location.pathname.includes('/crm/deliverables')) {
        items.push({ label: 'Deliverables', href: `/organizations/${orgId}/crm/deliverables` });
      }
    } else if (location.pathname.includes('/intelligence')) {
      items.push({ label: 'Intelligence', href: `/organizations/${orgId}/intelligence` });
      if (location.pathname.includes('/intelligence/data-sources')) {
        items.push({ label: 'Data Sources', href: `/organizations/${orgId}/intelligence/data-sources` });
      } else if (location.pathname.includes('/intelligence/artifacts')) {
        items.push({ label: 'Artifacts', href: `/organizations/${orgId}/intelligence/artifacts` });
      } else if (location.pathname.includes('/intelligence/workflows')) {
        items.push({ label: 'Workflows', href: `/organizations/${orgId}/intelligence/workflows` });
      } else if (location.pathname.includes('/intelligence/pulse')) {
        items.push({ label: 'Pulse', href: `/organizations/${orgId}/intelligence/pulse` });
      } else if (location.pathname.includes('/intelligence/topology')) {
        items.push({ label: 'Topology', href: `/organizations/${orgId}/intelligence/topology` });
      }
    } else if (location.pathname.includes('/social')) {
      items.push({ label: 'Social', href: `/organizations/${orgId}/social` });
    } else if (location.pathname.includes('/members')) {
      items.push({ label: 'Members', href: `/organizations/${orgId}/members` });
    } else if (location.pathname.includes('/projects')) {
      items.push({ label: 'Projects', href: `/organizations/${orgId}/projects` });
    } else if (location.pathname.includes('/integrations')) {
      items.push({ label: 'Integrations', href: `/organizations/${orgId}/integrations` });
    } else if (location.pathname.includes('/clients/')) {
      items.push({ label: 'Clients', href: `/organizations/${orgId}/crm/companies` });
      if (clientId && clientName) {
        items.push({ label: clientName, href: `/organizations/${orgId}/clients/${clientId}` });
      }
    } else if (location.pathname.includes('/brand-guide')) {
      items.push({ label: 'Brand Guide', href: `/organizations/${orgId}/brand-guide` });
    }
  }

  // Add page-level breadcrumb for routes without project context
  if (!projectId && !orgId) {
    const pageLabels: Record<string, string> = {
      '/projects': 'Projects',
      '/my-tasks': 'My Tasks',
      '/pulse': 'Pulse Engine',
      '/mesh': 'Mesh Network',
      '/virtual-environment': 'VIBELAND',
      '/vibe': 'VIBE',
      '/settings': 'Settings',
      '/nora': 'Nora Command',
      '/topsi': 'Topsi Platform',
      '/workflows': 'Workflows',
      '/global-tasks': 'Global Tasks',
      '/mission-control': 'Mission Control',
      '/social-command': 'Social Command',
      '/crm': 'CRM',
      '/people': 'People',
      '/companies': 'Companies',
      '/proposals': 'Proposals',
      '/invoices': 'Invoices',
      '/business-reports': 'Business Reports',
      '/command-center': 'Command Center',
      '/calendar': 'Calendar',
      '/discord': 'Discord Voice',
      '/oss-library-listener': 'OSS Library Listener',
      '/ai-usage': 'AI Usage',
      '/agent-executions': 'Agent Executions',
      '/site-directory': 'Site Directory',
    };
    // Sort by length descending so longer paths match first (e.g. /settings/profile before /settings)
    const matchedPath = Object.keys(pageLabels)
      .sort((a, b) => b.length - a.length)
      .find((p) => location.pathname.startsWith(p));
    if (matchedPath) {
      items.push({
        label: pageLabels[matchedPath],
        href: matchedPath,
      });
    }

    // Settings sub-pages
    if (location.pathname.startsWith('/settings/')) {
      const settingsLabels: Record<string, string> = {
        '/settings/general': 'General',
        '/settings/profile': 'Profile',
        '/settings/wallet': 'Wallet',
        '/settings/users': 'Users',
        '/settings/organizations': 'Organizations',
        '/settings/projects': 'Projects',
        '/settings/privacy': 'Privacy & Security',
        '/settings/activity': 'Activity Log',
        '/settings/agents': 'Agents',
        '/settings/keys': 'API Keys',
        '/settings/models': 'Models',
        '/settings/mcp': 'MCP Servers',
        '/settings/network': 'Network & Mesh',
      };
      const matchedSettings = Object.keys(settingsLabels).find((p) => location.pathname === p);
      if (matchedSettings) {
        items.push({ label: settingsLabels[matchedSettings], href: matchedSettings });
      }
    }

    // Person detail page
    if (personId && location.pathname.startsWith('/people/')) {
      const name = person
        ? person.full_name || person.email || 'Person'
        : undefined;
      if (name) {
        items.push({ label: name.length > 50 ? `${name.substring(0, 50)}...` : name, href: `/people/${personId}` });
      }
    }

    // Company detail page
    if (companyId && location.pathname.startsWith('/companies/')) {
      const name = company?.name;
      if (name) {
        items.push({ label: name.length > 50 ? `${name.substring(0, 50)}...` : name, href: `/companies/${companyId}` });
      }
    }

    // Business report detail
    if (location.pathname.match(/^\/business-reports\/[^/]+$/)) {
      items.push({ label: 'Report Detail', href: location.pathname });
    }
  }

  // Show skeleton while loading to avoid flashing incorrect org
  if (isSidebarLoading || !user) {
    return (
      <nav className="flex items-center space-x-1 text-sm text-muted-foreground px-4 py-2 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <Home className="h-4 w-4" />
        <ChevronRight className="h-4 w-4" />
        <div className="h-4 w-28 bg-muted animate-pulse rounded" />
      </nav>
    );
  }

  if (items.length === 0) {
    return null;
  }

  return (
    <nav className="flex items-center text-sm text-muted-foreground px-4 py-2 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="flex items-center space-x-1 flex-1 min-w-0">
        <Link
          to={currentOrg ? `/organizations/${currentOrg.id}` : '/projects'}
          className="flex items-center hover:text-foreground transition-colors shrink-0"
        >
          <Home className="h-4 w-4" />
        </Link>

        {items.map((item, index) => {
          const isLast = index === items.length - 1;
          const isOrgItem = currentOrg && item.href === `/organizations/${currentOrg.id}`;

          return (
            <div key={item.href} className="flex items-center space-x-1 min-w-0">
              <ChevronRight className="h-4 w-4 shrink-0" />
              {isOrgItem && allOrgs.length > 1 ? (
                <OrgSwitcher
                  currentOrg={currentOrg}
                  allOrgs={allOrgs}
                  onSelect={(id) => navigate(`/organizations/${id}`)}
                  isLast={isLast}
                />
              ) : (
                <Link
                  to={item.href}
                  className={cn(
                    'hover:text-foreground transition-colors truncate',
                    isLast && 'text-foreground font-medium'
                  )}
                >
                  {item.label}
                </Link>
              )}
            </div>
          );
        })}
      </div>

      {(() => {
        // Task pages use URL-based fullscreen; all other pages use store-based
        const isTaskPage = !!taskId && !!onToggleFullscreen;
        const isActive = isTaskPage ? !!isFullscreen : contentFullscreen;
        const handleToggle = isTaskPage
          ? () => onToggleFullscreen!(!isFullscreen)
          : toggleContentFullscreen;

        return (
          <button
            onClick={handleToggle}
            className="shrink-0 ml-3 flex items-center gap-1.5 rounded-md border px-2.5 py-1 text-xs font-medium hover:bg-muted transition-colors"
          >
            {isActive ? (
              <>
                <Minimize2 className="h-3.5 w-3.5" />
                Exit Fullscreen
              </>
            ) : (
              <>
                <Maximize2 className="h-3.5 w-3.5" />
                Fullscreen
              </>
            )}
          </button>
        );
      })()}
    </nav>
  );
}

function OrgSwitcher({
  currentOrg,
  allOrgs,
  onSelect,
  isLast,
}: {
  currentOrg: { id: string; name: string };
  allOrgs: { id: string; name: string }[];
  onSelect: (id: string) => void;
  isLast: boolean;
}) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState('');

  const filteredOrgs = useMemo(() => {
    const q = search.toLowerCase().trim();
    return allOrgs.filter((org) => org.id && (!q || org.name.toLowerCase().includes(q)));
  }, [allOrgs, search]);

  return (
    <Popover open={open} onOpenChange={(v) => { setOpen(v); if (!v) setSearch(''); }}>
      <PopoverTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1 rounded-md px-1.5 py-0.5 -mx-1.5 -my-0.5 border border-transparent transition-all',
            'hover:border-border hover:text-foreground hover:bg-muted/50',
            isLast && 'text-foreground font-medium'
          )}
        >
          {currentOrg.name}
          <ChevronsUpDown className="h-3 w-3 opacity-50" />
        </button>
      </PopoverTrigger>
      <PopoverContent className="w-[220px] p-0" align="start">
        <div className="p-2 border-b">
          <Input
            placeholder="Search orgs..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="h-8 text-sm"
            autoFocus
          />
        </div>
        <div className="max-h-[200px] overflow-y-auto p-1">
          {filteredOrgs.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-3">No organizations found.</p>
          ) : (
            filteredOrgs.map((org) => (
              <button
                key={org.id}
                onClick={() => {
                  onSelect(org.id);
                  setOpen(false);
                  setSearch('');
                }}
                className={cn(
                  'w-full text-left rounded-sm px-2 py-1.5 text-sm cursor-pointer transition-colors',
                  'hover:bg-accent hover:text-accent-foreground',
                  org.id === currentOrg.id && 'bg-accent font-medium'
                )}
              >
                {org.name}
              </button>
            ))
          )}
        </div>
      </PopoverContent>
    </Popover>
  );
}
