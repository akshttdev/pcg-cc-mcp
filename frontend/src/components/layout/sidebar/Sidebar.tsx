import { useEffect, useState } from 'react';
import { Link, useLocation, useNavigate, useParams } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from '@/components/ui/collapsible';
import {
  ChevronDown,
  ChevronRight,
  Plus,
  Crown,
  Star,
  UserCircle,
  LayoutDashboard,
} from 'lucide-react';
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { MoreHorizontal } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { projectsApi, organizationsApi, stagingApi } from '@/lib/api';
import type { SidebarTree } from '@/lib/api';
import type { Project } from 'shared/types';
import { useCommandStore } from '@/stores/useCommandStore';
import { useViewStore } from '@/stores/useViewStore';
import { useExpandable } from '@/stores/useExpandableStore';
import { useKeyToggleSidebar } from '@/keyboard/hooks';
import { Scope } from '@/keyboard/registry';
import NiceModal from '@ebay/nice-modal-react';
import type { CreateNameDialogResult } from '@/components/dialogs';
import { useAuth } from '@/contexts/AuthContext';
import { useEffectiveRole } from '@/hooks/useEffectiveRole';
import { SidebarUserCard } from '@/components/layout/SidebarUserCard';
import {
  ADMIN_NAV_ITEMS,
  PRIMARY_NAV_ITEMS,
  MANAGEMENT_NAV_ITEMS,
  GLOBAL_VIEW_ITEMS,
  UTILITY_NAV_ITEMS,
  EXTERNAL_LINKS,
  type NavItem,
} from './constants';
import { ProjectFolder } from './ProjectFolder';
import { SidebarOrgGroups } from './OrgSection';

interface SidebarProps {
  className?: string;
}

export function Sidebar({ className }: SidebarProps) {
  const location = useLocation();
  const navigate = useNavigate();
  const { projectId } = useParams<{ projectId: string }>();
  const { user } = useAuth();
  const { favorites, addFavorite, removeFavorite, isFavorite } = useCommandStore();
  const queryClient = useQueryClient();

  // Extract orgId from location since sidebar is outside org route tree
  const orgIdFromPath = location.pathname.match(/\/organizations\/([^/]+)/)?.[1];

  const { sidebarCollapsed, toggleSidebar } = useViewStore();

  // Fetch sidebar tree (hierarchical)
  const {
    data: sidebarTree,
    isLoading: isTreeLoading,
    error: treeError,
  } = useQuery<SidebarTree, Error>({
    queryKey: ['sidebarTree'],
    queryFn: organizationsApi.getSidebarTree,
  });

  // Fallback: flat project list (if tree fails or is empty)
  const {
    data: projects = [],
    isLoading: isProjectsLoading,
    error: projectsError,
  } = useQuery<Project[], Error>({
    queryKey: ['projects'],
    queryFn: projectsApi.getAll,
    enabled: !!treeError || (!!sidebarTree && sidebarTree.owned_orgs.length === 0 && sidebarTree.member_orgs.length === 0),
  });

  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(
    new Set(projectId ? [projectId] : [])
  );

  // Role-aware navigation gating
  const roleInfo = useEffectiveRole();
  const isAdmin = user?.is_admin ?? false;

  // Determine if we should show hierarchical or flat view
  const hasTree = sidebarTree && (sidebarTree.owned_orgs.length > 0 || sidebarTree.member_orgs.length > 0);
  const useFlatFallback = !hasTree && !isTreeLoading;

  // Part A fix: additive project expansion instead of single-select
  useEffect(() => {
    if (!projectId) return;
    setExpandedProjects(prev => {
      if (prev.has(projectId)) return prev;
      return new Set([...prev, projectId]);
    });
  }, [projectId]);

  const toggleProject = (id: string) => {
    setExpandedProjects(prev => {
      if (prev.has(id)) {
        const newSet = new Set(prev);
        newSet.delete(id);
        return newSet;
      } else {
        return new Set([...prev, id]);
      }
    });
  };

  // Part A fix: use Zustand-backed store for section toggles
  const [adminPlatformsExpanded, setAdminPlatformsExpanded] = useExpandable('sidebar:adminPlatforms', false);
  const [managementExpanded, setManagementExpanded] = useExpandable('sidebar:management', false);
  const [myWorkspaceExpanded, setMyWorkspaceExpanded] = useExpandable('sidebar:myWorkspace', true);

  // Staging pending count for sidebar badge
  const homeOrgId = user?.home_organization_id || user?.organizations?.[0]?.id;
  // Badge shows pending staging records needing review (user-level action items).
  // TODO: As we separate "My Workflows" (user tasks: review staged records, manage personal definitions)
  // from org-level Intelligence workflows (system automations, pipeline blueprints),
  // consider splitting this badge or moving org-level staging to Intelligence.
  const { data: stagingPendingCount = 0 } = useQuery({
    queryKey: ['stagingPendingCount', homeOrgId],
    queryFn: async () => {
      if (!homeOrgId) return 0;
      const records = await stagingApi.listPending(homeOrgId);
      return records.filter((r) => r.status === 'pending_review').length;
    },
    enabled: !!homeOrgId,
    staleTime: 30000,
    refetchInterval: 60000,
  });

  // Keyboard shortcut: Cmd+B / Ctrl+B to toggle sidebar
  useKeyToggleSidebar(() => toggleSidebar(), { scope: Scope.GLOBAL });

  // Filter navigation items based on effective role
  const filteredAdminNav = roleInfo.canSeeAdminPlatforms
    ? ADMIN_NAV_ITEMS
    : [];
  const filteredPrimaryNav = PRIMARY_NAV_ITEMS.filter((item) => {
    if (item.adminOnly && !isAdmin) return false;
    return true;
  });

  // Helper: determine if a nav item is active
  const isNavActive = (item: NavItem) => {
    if (item.to === '/projects') return location.pathname === '/projects';
    return location.pathname.startsWith(item.to);
  };

  // Render a single nav item (collapsed or expanded)
  const renderNavItem = (item: NavItem, active: boolean) => {
    const Icon = item.icon;
    const isAdminTool = item.id === 'nora' || item.id === 'topsi';
    const adminBg = item.id === 'nora'
      ? 'bg-primary/5 hover:bg-primary/10'
      : item.id === 'topsi'
        ? 'bg-[hsl(var(--info)/0.05)] hover:bg-[hsl(var(--info)/0.1)]'
        : '';
    const adminIconColor = item.id === 'nora' ? 'text-primary' : item.id === 'topsi' ? 'text-[hsl(var(--info))]' : '';
    const adminBadgeBg = item.id === 'nora' ? 'bg-primary' : item.id === 'topsi' ? 'bg-[hsl(var(--info))]' : '';

    if (sidebarCollapsed) {
      return (
        <Tooltip key={item.id}>
          <TooltipTrigger asChild>
            <Link to={item.to}>
              <Button
                variant="ghost"
                className={cn(
                  "w-full justify-center p-2 h-auto",
                  active && "sidebar-nav-item-active",
                  !active && adminBg
                )}
              >
                <Icon className={cn("h-4 w-4", adminIconColor)} />
              </Button>
            </Link>
          </TooltipTrigger>
          <TooltipContent side="right">{item.label}</TooltipContent>
        </Tooltip>
      );
    }

    return (
      <Link key={item.id} to={item.to}>
        <div
          className={cn(
            "sidebar-nav-item",
            active && "sidebar-nav-item-active",
            !active && adminBg
          )}
        >
          <Icon className={cn("h-4 w-4", adminIconColor)} />
          <span className="flex-1">{item.label}</span>
          {isAdminTool && (
            <span className={cn("text-[10px] text-white px-1.5 py-0.5 rounded", adminBadgeBg)}>
              ADMIN
            </span>
          )}
        </div>
      </Link>
    );
  };

  return (
    <TooltipProvider delayDuration={0}>
    <div className={cn(
      "flex flex-col h-full sidebar-container transition-all duration-200 overflow-hidden",
      sidebarCollapsed ? "w-14" : "w-72",
      className
    )}>
      {/* Collapsed sidebar: fixed sections */}
      {sidebarCollapsed && filteredAdminNav.length > 0 && (
        <div className="border-b border-border/40 p-1.5">
          <div className="space-y-1">
            {filteredAdminNav.map((item) => renderNavItem(item, isNavActive(item)))}
          </div>
        </div>
      )}
      {sidebarCollapsed && (
        <div className="border-b border-border/40 p-1.5">
          <div className="space-y-0.5">
            {filteredPrimaryNav.map((item) => renderNavItem(item, isNavActive(item)))}
          </div>
        </div>
      )}

      {/* Expanded sidebar: single scrollable area for all nav + org tree */}
      {!sidebarCollapsed && <div className="flex-1 flex flex-col overflow-hidden min-h-0">
        <ScrollArea className="flex-1 min-h-0">
          {/* Admin Platforms (collapsible, only shown for admins) */}
          <div className={cn(
            "transition-all duration-200 ease-in-out overflow-hidden",
            filteredAdminNav.length > 0 ? "max-h-[500px] opacity-100" : "max-h-0 opacity-0"
          )}>
            <div className="border-b border-border/40 p-2 px-3">
              <Collapsible open={adminPlatformsExpanded} onOpenChange={setAdminPlatformsExpanded}>
                <CollapsibleTrigger asChild>
                  <div className="sidebar-nav-item justify-between cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <Crown className="h-4 w-4 text-primary" />
                      <span>Admin Platforms</span>
                      {!adminPlatformsExpanded && filteredAdminNav.length > 0 && (
                        <span className="text-[10px] text-muted-foreground/70 font-medium">{filteredAdminNav.length}</span>
                      )}
                    </div>
                    {adminPlatformsExpanded ? (
                      <ChevronDown className="h-3.5 w-3.5" />
                    ) : (
                      <ChevronRight className="h-3.5 w-3.5" />
                    )}
                  </div>
                </CollapsibleTrigger>
                <CollapsibleContent>
                  <div className="space-y-0.5 pl-4 border-l border-border/40 ml-2 mt-1">
                    {filteredAdminNav.map((item) => renderNavItem(item, isNavActive(item)))}
                  </div>
                </CollapsibleContent>
              </Collapsible>
            </div>
          </div>

          {/* My Workspace — user-level pages */}
          <div className="border-b border-border/40">
            <Collapsible open={myWorkspaceExpanded} onOpenChange={setMyWorkspaceExpanded}>
              <CollapsibleTrigger asChild>
                <div className="sidebar-nav-item mx-3 my-1.5 justify-between cursor-pointer">
                  <div className="flex items-center gap-2.5">
                    <UserCircle className="h-4 w-4" />
                    <span>My Workspace</span>
                    {!myWorkspaceExpanded && filteredPrimaryNav.length > 0 && (
                      <span className="text-[10px] text-muted-foreground/70 font-medium">{filteredPrimaryNav.length}</span>
                    )}
                  </div>
                  {myWorkspaceExpanded ? (
                    <ChevronDown className="h-3.5 w-3.5" />
                  ) : (
                    <ChevronRight className="h-3.5 w-3.5" />
                  )}
                </div>
              </CollapsibleTrigger>
              <CollapsibleContent className="px-3 pb-1">
                <div className="space-y-0.5 pl-4 border-l border-border/40 ml-2">
                  {filteredPrimaryNav.map((item) => {
                    const Icon = item.icon;
                    const active = isNavActive(item);
                    const badgeCount = item.id === 'workflows' ? stagingPendingCount : 0;
                    return (
                      <Link key={item.id} to={item.to}>
                        <div className={cn(
                          "sidebar-nav-item text-xs py-1 justify-between",
                          active && "sidebar-nav-item-active"
                        )}>
                          <div className="flex items-center gap-2">
                            <Icon className="h-3.5 w-3.5" />
                            {item.label}
                          </div>
                          {badgeCount > 0 && (
                            <span className="text-[9px] bg-amber-100 dark:bg-amber-900/40 text-amber-700 dark:text-amber-300 rounded-full px-1.5 py-0.5 leading-none font-medium">
                              {badgeCount}
                            </span>
                          )}
                        </div>
                      </Link>
                    );
                  })}
                </div>
              </CollapsibleContent>
            </Collapsible>
          </div>

          {/* Views & Management — merged section, role-gated */}
          <div className={cn(
            "transition-all duration-200 ease-in-out overflow-hidden",
            roleInfo.canSeeManagement ? "max-h-[800px] opacity-100" : "max-h-0 opacity-0"
          )}>
            <div className="border-b border-border/40">
              <Collapsible open={managementExpanded} onOpenChange={setManagementExpanded}>
                <CollapsibleTrigger asChild>
                  <div className="sidebar-nav-item mx-3 my-1.5 justify-between cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <LayoutDashboard className="h-4 w-4" />
                      <span>Views & Management</span>
                      {!managementExpanded && (
                        <span className="text-[10px] text-muted-foreground/70 font-medium">{MANAGEMENT_NAV_ITEMS.length + GLOBAL_VIEW_ITEMS.length}</span>
                      )}
                    </div>
                    {managementExpanded ? (
                      <ChevronDown className="h-3.5 w-3.5" />
                    ) : (
                      <ChevronRight className="h-3.5 w-3.5" />
                    )}
                  </div>
                </CollapsibleTrigger>
                <CollapsibleContent className="px-3 pb-1">
                  <div className="space-y-0.5 pl-4 border-l border-border/40 ml-2">
                    {/* Global Views first (fewer, higher-level) */}
                    {GLOBAL_VIEW_ITEMS.map((item) => {
                      const Icon = item.icon;
                      const active = location.pathname === item.to;
                      return (
                        <Link key={item.id} to={item.to}>
                          <div className={cn(
                            "sidebar-nav-item text-xs py-1",
                            active && "sidebar-nav-item-active"
                          )}>
                            <Icon className="h-3.5 w-3.5" />
                            {item.label}
                          </div>
                        </Link>
                      );
                    })}
                    {/* Subtle divider between global views and management */}
                    <div className="border-t border-border/30 my-1" />
                    {MANAGEMENT_NAV_ITEMS.map((item) => {
                      const Icon = item.icon;
                      const active = location.pathname === item.to;
                      return (
                        <Link key={item.id} to={item.to}>
                          <div className={cn(
                            "sidebar-nav-item text-xs py-1",
                            active && "sidebar-nav-item-active"
                          )}>
                            <Icon className="h-3.5 w-3.5" />
                            {item.label}
                          </div>
                        </Link>
                      );
                    })}
                  </div>
                </CollapsibleContent>
              </Collapsible>
            </div>
          </div>

          {/* Favorites Section */}
          {favorites.length > 0 && (
            <div className="border-b border-border/40">
              <div className="px-3 py-2">
                <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
                  Favorites
                </span>
              </div>
              <div className="px-3 pb-2 space-y-1">
                {favorites.map((fav) => {
                  const proj = projects.find((p) => p.id === fav.projectId);
                  if (!proj) return null;

                  return (
                    <Link key={fav.id} to={`/projects/${proj.id}/tasks`}>
                      <Button
                        variant="ghost"
                        className={cn(
                          "w-full justify-start px-2 py-1.5 h-auto font-normal",
                          projectId === proj.id && "bg-primary/10 text-foreground font-medium"
                        )}
                      >
                        <Star className="h-4 w-4 mr-2 text-[hsl(var(--warning))] fill-[hsl(var(--warning))]" />
                        <span className="text-sm truncate">{proj.name}</span>
                      </Button>
                    </Link>
                  );
                })}
              </div>
            </div>
          )}

          {/* Organizations & Projects Section (Hierarchical) */}
          <div className="px-3 py-2 flex items-center justify-between">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            Organizations
          </span>
          {isAdmin && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 w-7 p-0 hover:bg-accent"
              title="New Organization"
              onClick={async () => {
                try {
                  const result = await NiceModal.show('create-name', {
                    title: 'New Organization',
                    label: 'Organization Name',
                    placeholder: 'Enter organization name...',
                    submitText: 'Create Organization',
                  }) as CreateNameDialogResult;
                  const slug = result.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
                  const newOrg = await organizationsApi.create({ name: result.name, slug });
                  queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
                  if (newOrg?.id) {
                    navigate(`/organizations/${newOrg.id}`);
                  }
                } catch {
                  // dialog dismissed
                }
              }}
            >
              <Plus className="h-4 w-4" />
            </Button>
          )}
        </div>

        <div className="px-3">
          <div className="space-y-1">
            {isTreeLoading ? (
              <div className="py-2 space-y-3">
                {/* Org skeleton */}
                <div className="space-y-2">
                  <div className="flex items-center gap-2">
                    <div className="h-2 w-2 rounded-full bg-muted animate-pulse" />
                    <div className="h-4 w-32 rounded bg-muted animate-pulse" />
                  </div>
                  <div className="pl-4 space-y-1.5">
                    <div className="h-3.5 w-20 rounded bg-muted animate-pulse" />
                    <div className="h-3.5 w-24 rounded bg-muted animate-pulse" />
                    <div className="h-3.5 w-28 rounded bg-muted animate-pulse" />
                  </div>
                  <div className="pl-4 space-y-1.5 pt-1">
                    <div className="h-3 w-16 rounded bg-muted/60 animate-pulse" />
                    <div className="flex items-center gap-1.5 pl-2">
                      <div className="h-2 w-2 rounded-full bg-muted animate-pulse" />
                      <div className="h-3.5 w-36 rounded bg-muted animate-pulse" />
                    </div>
                  </div>
                </div>
              </div>
            ) : hasTree ? (
              <SidebarOrgGroups
                sidebarTree={sidebarTree!}
                projectId={projectId}
                orgId={orgIdFromPath}
                isAdmin={isAdmin}
                homeOrgId={user?.home_organization_id || user?.organizations?.[0]?.id}
                expandedProjects={expandedProjects}
                onToggleProject={toggleProject}
                queryClient={queryClient}
              />
            ) : useFlatFallback ? (
              // Fallback: flat project list
              isProjectsLoading ? (
                <div className="py-2 space-y-2">
                  {[1, 2, 3].map((i) => (
                    <div key={i} className="flex items-center gap-2">
                      <div className="h-2 w-2 rounded-full bg-muted animate-pulse" />
                      <div className="h-4 rounded bg-muted animate-pulse" style={{ width: `${60 + i * 20}px` }} />
                    </div>
                  ))}
                </div>
              ) : projectsError ? (
                <div className="py-4 text-xs text-destructive">
                  Failed to load projects
                </div>
              ) : projects.length === 0 ? (
                <div className="py-4 text-xs text-muted-foreground">
                  No projects yet. Create one to get started.
                </div>
              ) : (
                projects.map((project) => (
                  <ProjectFolder
                    key={project.id}
                    project={project}
                    isActive={project.id === projectId}
                    isExpanded={expandedProjects.has(project.id)}
                    onToggle={() => toggleProject(project.id)}
                    isFavorite={isFavorite(project.id)}
                    onToggleFavorite={() => {
                      if (isFavorite(project.id)) {
                        removeFavorite(project.id);
                      } else {
                        addFavorite(project.id, project.name);
                      }
                    }}
                  />
                ))
              )
            ) : null}
          </div>
        </div>
        </ScrollArea>
      </div>}

      {/* Collapsed org indicator */}
      {sidebarCollapsed && (() => {
        const allOrgs = sidebarTree ? [...sidebarTree.owned_orgs, ...sidebarTree.member_orgs] : [];
        const activeOrgId = orgIdFromPath || homeOrgId;
        const activeOrg = activeOrgId ? allOrgs.find((o) => o.id === activeOrgId) : allOrgs[0];
        if (!activeOrg) return null;
        const initial = activeOrg.name?.charAt(0)?.toUpperCase() || '?';
        return (
          <div className="border-b border-border/40 p-1.5">
            <Tooltip>
              <TooltipTrigger asChild>
                <Link to={`/organizations/${activeOrg.id}`}>
                  <Button
                    variant="ghost"
                    className="w-full justify-center p-2 h-auto"
                  >
                    <div className="h-6 w-6 rounded bg-primary/15 text-primary flex items-center justify-center text-xs font-bold">
                      {initial}
                    </div>
                  </Button>
                </Link>
              </TooltipTrigger>
              <TooltipContent side="right">{activeOrg.name}</TooltipContent>
            </Tooltip>
          </div>
        );
      })()}

      {/* Spacer when collapsed */}
      {sidebarCollapsed && <div className="flex-1" />}

      {/* Bottom section: Settings + External Links + Collapse Toggle */}
      <div className={cn("border-t border-border/40 flex-shrink-0", sidebarCollapsed ? "p-1.5" : "p-2 px-3")}>
        <div className="space-y-1">
          {/* Settings (utility — pinned to bottom) */}
          {UTILITY_NAV_ITEMS.map((item) => renderNavItem(item, isNavActive(item)))}

          {/* Subtle separator */}
          <div className={cn("border-t border-border/40 my-1", sidebarCollapsed ? "mx-1" : "mx-0")} />

          {/* External Links — collapsed into "More" popover */}
          {sidebarCollapsed ? (
            <Popover>
              <Tooltip>
                <TooltipTrigger asChild>
                  <PopoverTrigger asChild>
                    <Button variant="ghost" className="w-full justify-center p-2 h-auto text-muted-foreground hover:text-foreground">
                      <MoreHorizontal className="h-4 w-4" />
                    </Button>
                  </PopoverTrigger>
                </TooltipTrigger>
                <TooltipContent side="right">More</TooltipContent>
              </Tooltip>
              <PopoverContent side="right" align="end" className="w-48 p-1">
                {EXTERNAL_LINKS.map((item) => {
                  const Icon = item.icon;
                  if (item.external) {
                    return (
                      <a key={item.href} href={item.href} target="_blank" rel="noopener noreferrer" className="block">
                        <Button variant="ghost" className="w-full justify-start px-3 py-2 h-auto text-sm">
                          <Icon className="h-4 w-4 mr-2" />
                          {item.label}
                        </Button>
                      </a>
                    );
                  }
                  return (
                    <Button
                      key={item.label}
                      variant="ghost"
                      className="w-full justify-start px-3 py-2 h-auto text-sm"
                      data-testid={item.action ? `${item.action}-button` : undefined}
                      onClick={() => { if (item.action) NiceModal.show(item.action); }}
                    >
                      <Icon className="h-4 w-4 mr-2" />
                      {item.label}
                    </Button>
                  );
                })}
              </PopoverContent>
            </Popover>
          ) : (
            <Popover>
              <PopoverTrigger asChild>
                <Button
                  variant="ghost"
                  className="w-full justify-start px-3 py-2 h-auto text-muted-foreground hover:text-foreground"
                >
                  <MoreHorizontal className="h-4 w-4 mr-3" />
                  <span className="text-sm">More</span>
                </Button>
              </PopoverTrigger>
              <PopoverContent side="right" align="end" className="w-48 p-1">
                {EXTERNAL_LINKS.map((item) => {
                  const Icon = item.icon;
                  if (item.external) {
                    return (
                      <a key={item.href} href={item.href} target="_blank" rel="noopener noreferrer" className="block">
                        <Button variant="ghost" className="w-full justify-start px-3 py-2 h-auto text-sm">
                          <Icon className="h-4 w-4 mr-2" />
                          {item.label}
                        </Button>
                      </a>
                    );
                  }
                  return (
                    <Button
                      key={item.label}
                      variant="ghost"
                      className="w-full justify-start px-3 py-2 h-auto text-sm"
                      data-testid={item.action ? `${item.action}-button` : undefined}
                      onClick={() => { if (item.action) NiceModal.show(item.action); }}
                    >
                      <Icon className="h-4 w-4 mr-2" />
                      {item.label}
                    </Button>
                  );
                })}
              </PopoverContent>
            </Popover>
          )}

          {/* User card with view-as switcher */}
          <div className={cn("border-t border-border/40 my-1", sidebarCollapsed ? "mx-1" : "mx-0")} />
          <SidebarUserCard isCollapsed={sidebarCollapsed} />

          {/* Subtle separator */}
          <div className={cn("border-t border-border/40 my-1", sidebarCollapsed ? "mx-1" : "mx-0")} />

          {/* Collapse Toggle — at the very bottom */}
          {sidebarCollapsed ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  className="w-full justify-center p-2 h-auto text-muted-foreground hover:text-foreground"
                  onClick={toggleSidebar}
                >
                  <PanelLeftOpen className="h-4 w-4" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">Expand sidebar <kbd className="ml-1 text-[10px] opacity-60">&#8984;B</kbd></TooltipContent>
            </Tooltip>
          ) : (
            <Button
              variant="ghost"
              className="w-full justify-start px-3 py-2 h-auto text-muted-foreground hover:text-foreground"
              onClick={toggleSidebar}
            >
              <PanelLeftClose className="h-4 w-4 mr-3" />
              <span className="text-sm">Collapse</span>
              <kbd className="ml-auto text-[10px] text-muted-foreground/60 font-sans">&#8984;B</kbd>
            </Button>
          )}
        </div>
      </div>
    </div>
    </TooltipProvider>
  );
}
