import { ChevronRight, Home, ChevronsUpDown } from 'lucide-react';
import { Link, useParams, useNavigate, useLocation } from 'react-router-dom';
import { useProject } from '@/contexts/project-context';
import { useQuery } from '@tanstack/react-query';
import { tasksApi, organizationsApi } from '@/lib/api';
import { cn } from '@/lib/utils';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { useState, useMemo } from 'react';
import { Input } from '@/components/ui/input';

interface BreadcrumbItem {
  label: string;
  href: string;
}

export function BreadcrumbNav() {
  const { projectId, taskId, orgId } = useParams<{
    projectId?: string;
    taskId?: string;
    orgId?: string;
  }>();
  const { project } = useProject();
  const navigate = useNavigate();
  const location = useLocation();

  // Fetch sidebar tree for org/project switching
  const { data: sidebarTree } = useQuery({
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

  // Derive current org from project or route
  const currentOrg = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])].find((org) => {
        if (orgId) return org.id === orgId;
        if (project && projectId) {
          // Find org containing this project
          const findProject = (projects: any[]): boolean =>
            projects.some((p: any) => p.id === projectId || findProject(p.children || []));
          return (
            findProject(org.internal_projects) ||
            org.clients.some((c: any) => findProject(c.projects))
          );
        }
        return false;
      })
    : undefined;

  // Build breadcrumb items
  const items: BreadcrumbItem[] = [];

  items.push({
    label: 'Home',
    href: '/projects',
  });

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

  // Add page-level breadcrumb for routes without project context
  if (!projectId && !orgId) {
    const pageLabels: Record<string, string> = {
      '/projects': 'Projects',
      '/my-tasks': 'My Tasks',
      '/pulse': 'Pulse Engine',
      '/mesh': 'Mesh Network',
      '/virtual-environment': 'VIBELAND',
      '/settings': 'Settings',
      '/nora': 'Nora Command',
      '/topsi': 'Topsi Platform',
    };
    const matchedPath = Object.keys(pageLabels).find((p) => location.pathname.startsWith(p));
    if (matchedPath) {
      items.push({
        label: pageLabels[matchedPath],
        href: matchedPath,
      });
    }
  }

  if (items.length <= 1) {
    return null;
  }

  const allOrgs = sidebarTree
    ? [...(sidebarTree.owned_orgs || []), ...(sidebarTree.member_orgs || [])]
    : [];

  return (
    <nav className="flex items-center space-x-1 text-sm text-muted-foreground px-4 py-2 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <Link
        to="/projects"
        className="flex items-center hover:text-foreground transition-colors"
      >
        <Home className="h-4 w-4" />
      </Link>

      {items.slice(1).map((item, index) => {
        const isLast = index === items.length - 2;
        const isOrgItem = currentOrg && item.href === `/organizations/${currentOrg.id}`;

        return (
          <div key={item.href} className="flex items-center space-x-1">
            <ChevronRight className="h-4 w-4" />
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
                  'hover:text-foreground transition-colors',
                  isLast && 'text-foreground font-medium'
                )}
              >
                {item.label}
              </Link>
            )}
          </div>
        );
      })}
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
