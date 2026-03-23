import { useMemo } from 'react';
import { Link, useLocation, useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
  ChevronDown,
  ChevronRight,
  Plus,
  Folder,
  Building2,
  Plug,
  Users,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { QueryClient } from '@tanstack/react-query';
import { sidebarKeys } from '@/lib/query-keys';
import { organizationsApi } from '@/lib/api';
import type {
  SidebarTree,
  SidebarOrg,
} from '@/lib/api';
import NiceModal from '@ebay/nice-modal-react';
import type { CreateNameDialogResult } from '@/components/dialogs';
import type { ProjectFormDialogResult } from '@/components/dialogs';
import { useExpandable } from '@/stores/useExpandableStore';
import { HealthDot } from './HealthDot';
import { isProjectInTree } from './helpers';
import { OrgCrmSection, OrgSocialSection, OrgIntelligenceSection } from './OrgWorkspaceLinks';
import { SortableProjectList } from './ProjectList';
import { ClientGroup } from './ClientGroup';

// ============================================================================
// OrgSection — renders org name as section header with internal projects + clients
// ============================================================================

export function OrgSection({
  org,
  projectId,
  activeOrgId,
  isAdmin,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  org: SidebarOrg;
  projectId?: string;
  activeOrgId?: string;
  isAdmin: boolean;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient: QueryClient;
}) {
  const navigate = useNavigate();
  const location = useLocation();

  // Part A fix: use Zustand-backed store instead of useState for collapse persistence
  // Default collapsed when section is empty to reduce visual noise
  const [internalExpanded, setInternalExpanded] = useExpandable(`sidebar:org:${org.id}:internal`, org.internal_projects.length > 0);
  const [clientsExpanded, setClientsExpanded] = useExpandable(`sidebar:org:${org.id}:clients`, org.clients.length > 0);
  const hasActiveProject = projectId ? (
    isProjectInTree(org.internal_projects, projectId) ||
    org.clients.some((c) => isProjectInTree(c.projects, projectId))
  ) : false;
  const isActiveOrg = activeOrgId === org.id || hasActiveProject;

  // Default: only expand the active org to reduce cognitive load
  const [orgContentExpanded, setOrgContentExpanded] = useExpandable(`sidebar:org:${org.id}`, isActiveOrg);

  return (
    <div className="space-y-0.5">
      {/* Org header — clickable to toggle content */}
      <Collapsible open={orgContentExpanded} onOpenChange={setOrgContentExpanded}>
        <div className={cn(
          "flex items-center rounded-sm transition-colors",
          isActiveOrg && "bg-primary/10 dark:bg-primary/15 shadow-[inset_3px_0_0_hsl(var(--brand))]"
        )}>
          <Button
            variant="ghost"
            className={cn(
              "flex-1 justify-start px-2 py-1.5 h-auto font-medium text-xs uppercase tracking-wider text-muted-foreground hover:text-foreground min-w-0",
              isActiveOrg && "text-foreground font-semibold"
            )}
            onClick={() => {
              if (org.id) navigate(`/organizations/${org.id}`);
            }}
          >
            <div className="flex items-center gap-1.5 min-w-0">
              <HealthDot status={org.health_status} />
              <Building2 className="h-3.5 w-3.5 shrink-0" />
              <span className="truncate">{org.name}</span>
            </div>
          </Button>
          <CollapsibleTrigger asChild>
            <Button variant="ghost" size="sm" className="h-6 w-6 p-0 mr-1">
              {orgContentExpanded ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
            </Button>
          </CollapsibleTrigger>
        </div>

        {/* Org content */}
        <CollapsibleContent>
      <div className="pl-2 space-y-0.5">
        {/* Org-level workspace links: CRM, Social, Intelligence */}
        <div className="px-1 py-1 space-y-0.5">
          <OrgCrmSection orgId={org.id} location={location} />

          <OrgSocialSection orgId={org.id} location={location} />

          <OrgIntelligenceSection orgId={org.id} location={location} />

          <Link
            to={`/organizations/${org.id}/members`}
            className={cn(
              'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/organizations/${org.id}/members` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Users className="h-3 w-3 shrink-0 text-muted-foreground" />
            <span>Members</span>
          </Link>

          <Link
            to={`/organizations/${org.id}/projects`}
            className={cn(
              'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/organizations/${org.id}/projects` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Folder className="h-3 w-3 shrink-0 text-muted-foreground" />
            <span>Projects</span>
          </Link>

          <Link
            to={`/organizations/${org.id}/integrations`}
            className={cn(
              'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/organizations/${org.id}/integrations` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Plug className="h-3 w-3 shrink-0 text-muted-foreground" />
            <span>Integrations</span>
          </Link>
        </div>

        {/* Internal projects — collapsible */}
        <Collapsible open={internalExpanded} onOpenChange={setInternalExpanded}>
          <div className="px-2 py-0.5 flex items-center justify-between">
            <CollapsibleTrigger asChild>
              <button className="flex items-center gap-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider hover:text-foreground">
                {internalExpanded ? <ChevronDown className="h-2.5 w-2.5" /> : <ChevronRight className="h-2.5 w-2.5" />}
                Internal Projects
              </button>
            </CollapsibleTrigger>
            {isAdmin && org.role === 'admin' && (
              <Button
                variant="ghost"
                size="sm"
                className="h-5 w-5 p-0 hover:bg-accent"
                title="New Project"
                onClick={async () => {
                  try {
                    const result = await NiceModal.show('project-form', {
                      organization_id: org.id,
                    }) as ProjectFormDialogResult;
                    if (result === 'saved') {
                      queryClient.invalidateQueries({ queryKey: sidebarKeys.tree() });
                    }
                  } catch {
                    // dialog dismissed
                  }
                }}
              >
                <Plus className="h-3 w-3" />
              </Button>
            )}
          </div>
          <CollapsibleContent>
            {org.internal_projects.length > 0 && (
              <SortableProjectList
                scopeKey={`org:${org.id}:internal`}
                projects={org.internal_projects}
                projectId={projectId}
                expandedProjects={expandedProjects}
                onToggleProject={onToggleProject}
                queryClient={queryClient}
              />
            )}
          </CollapsibleContent>
        </Collapsible>

        {/* Client groups — collapsible */}
        <Collapsible open={clientsExpanded} onOpenChange={setClientsExpanded}>
          <div className="px-2 py-0.5 flex items-center justify-between">
            <CollapsibleTrigger asChild>
              <button className="flex items-center gap-1 text-xs font-semibold text-muted-foreground uppercase tracking-wider hover:text-foreground">
                {clientsExpanded ? <ChevronDown className="h-2.5 w-2.5" /> : <ChevronRight className="h-2.5 w-2.5" />}
                Clients
              </button>
            </CollapsibleTrigger>
            {isAdmin && org.role === 'admin' && (
              <Button
                variant="ghost"
                size="sm"
                className="h-5 w-5 p-0 hover:bg-accent"
                title="New Client"
                onClick={async () => {
                  try {
                    const result = await NiceModal.show('create-name', {
                      title: 'New Client',
                      label: 'Client Name',
                      placeholder: 'Enter client name...',
                      submitText: 'Create Client',
                    }) as CreateNameDialogResult;
                    const slug = result.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
                    await organizationsApi.createClient(org.id, { name: result.name, slug });
                    queryClient.invalidateQueries({ queryKey: sidebarKeys.tree() });
                  } catch {
                    // dialog dismissed
                  }
                }}
              >
                <Plus className="h-3 w-3" />
              </Button>
            )}
          </div>
          <CollapsibleContent>
            {org.clients.map((client) => (
              <ClientGroup
                key={client.id}
                client={client}
                organizationId={org.id}
                projectId={projectId}
                expandedProjects={expandedProjects}
                onToggleProject={onToggleProject}
                queryClient={queryClient}
              />
            ))}
          </CollapsibleContent>
        </Collapsible>

      </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}

// ============================================================================
// SidebarOrgGroups — renders all orgs as collapsible sections
// ============================================================================

export function SidebarOrgGroups({
  sidebarTree,
  projectId,
  orgId,
  isAdmin,
  homeOrgId,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  sidebarTree: SidebarTree;
  projectId?: string;
  orgId?: string;
  isAdmin: boolean;
  homeOrgId?: string | null;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient: QueryClient;
}) {
  // Derive activeOrgId for highlight purposes only
  const allOrgs = useMemo(
    () => [...sidebarTree.owned_orgs, ...sidebarTree.member_orgs],
    [sidebarTree.owned_orgs, sidebarTree.member_orgs]
  );
  const activeOrgId = useMemo(() => {
    if (orgId) return orgId;
    if (projectId) {
      for (const org of allOrgs) {
        if (isProjectInTree(org.internal_projects, projectId)) return org.id;
        for (const client of org.clients) {
          if (isProjectInTree(client.projects, projectId)) return org.id;
        }
      }
    }
    if (homeOrgId) return homeOrgId;
    return undefined;
  }, [orgId, projectId, homeOrgId, allOrgs]);

  return (
    <>
      {allOrgs.map((org) => (
        <OrgSection
          key={org.id || org.slug}
          org={org}
          projectId={projectId}
          activeOrgId={activeOrgId}
          isAdmin={isAdmin}
          expandedProjects={expandedProjects}
          onToggleProject={onToggleProject}
          queryClient={queryClient}
        />
      ))}
    </>
  );
}
