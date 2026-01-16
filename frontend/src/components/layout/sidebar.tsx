import { useEffect, useMemo, useState } from 'react';
import { Link, useLocation, useParams } from 'react-router-dom';
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
  FolderOpen,
  Settings,
  BookOpen,
  MessageCircleQuestion,
  Plus,
  Folder,
  Loader2,
  Crown,
  Star,
  Box,
  Share2,
  Megaphone,
  Users,
  ListTodo,
  BarChart3,
  Bot,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { useQuery } from '@tanstack/react-query';
import { projectsApi, tasksApi } from '@/lib/api';
import { showProjectForm } from '@/lib/modals';
import type { Project, ProjectBoard, TaskWithAttemptStatus } from 'shared/types';
import { useCommandStore } from '@/stores/useCommandStore';
import NiceModal from '@ebay/nice-modal-react';
import { useAuth } from '@/contexts/AuthContext';

interface SidebarProps {
  className?: string;
}

// Navigation items with role-based visibility
interface NavItem {
  label: string;
  icon: typeof FolderOpen;
  to: string;
  id: string;
  adminOnly?: boolean;
  memberOnly?: boolean;
}

// Primary navigation - always visible (role-filtered)
const PRIMARY_NAV_ITEMS: NavItem[] = [
  { label: 'Nora Command', icon: Crown, to: '/nora', id: 'nora', adminOnly: true },
  { label: 'Projects', icon: FolderOpen, to: '/projects', id: 'projects' },
  { label: 'My Tasks', icon: ListTodo, to: '/my-tasks', id: 'my-tasks', memberOnly: true },
  { label: 'Virtual World', icon: Box, to: '/virtual-environment', id: 'virtual-environment' },
  { label: 'Settings', icon: Settings, to: '/settings', id: 'settings' },
];

// Global views - admin only, collapsible
const GLOBAL_VIEW_ITEMS: NavItem[] = [
  { label: 'All Tasks', icon: ListTodo, to: '/global-tasks', id: 'global-tasks', adminOnly: true },
  { label: 'All CRM', icon: Users, to: '/crm', id: 'crm', adminOnly: true },
  { label: 'All Social', icon: Megaphone, to: '/social-command', id: 'social-command', adminOnly: true },
];

const EXTERNAL_LINKS = [
  {
    label: 'Docs',
    icon: BookOpen,
    href: 'https://duckkanban.com/docs',
    external: true,
  },
  {
    label: 'Feedback & Support',
    icon: MessageCircleQuestion,
    action: 'feedback',
    external: false,
  },
];

interface ProjectFolderProps {
  project: Project;
  isActive: boolean;
  isExpanded: boolean;
  onToggle: () => void;
  isFavorite: boolean;
  onToggleFavorite: () => void;
}

function ProjectFolder({ project, isActive, isExpanded, onToggle, isFavorite, onToggleFavorite }: ProjectFolderProps) {
  const location = useLocation();
  const shouldFetchTasks =
    isExpanded || location.pathname.includes(`/projects/${project.id}`);
  const shouldFetchBoards = shouldFetchTasks;

  const {
    data: tasksData = [],
    isLoading: isTasksLoading,
    error: tasksError,
  } = useQuery<TaskWithAttemptStatus[], Error>({
    queryKey: ['projectTasksSidebar', project.id],
    queryFn: () => tasksApi.getAll(project.id),
    enabled: shouldFetchTasks,
    staleTime: 60 * 1000,
  });

  const {
    data: boardsData = [],
    isLoading: isBoardsLoading,
    error: boardsError,
  } = useQuery<ProjectBoard[], Error>({
    queryKey: ['projectBoardsSidebar', project.id],
    queryFn: () => projectsApi.listBoards(project.id),
    enabled: shouldFetchBoards,
    staleTime: 5 * 60 * 1000,
  });

  const tasksByBoard = useMemo(() => {
    const map = new Map<string, TaskWithAttemptStatus[]>();
    tasksData.forEach((task) => {
      const key = task.board_id ?? 'unassigned';
      if (!map.has(key)) {
        map.set(key, []);
      }
      map.get(key)!.push(task);
    });
    return map;
  }, [tasksData]);

  const unassignedTasks = tasksByBoard.get('unassigned') ?? [];

  return (
    <Collapsible open={isExpanded} onOpenChange={onToggle}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          className={cn(
            "w-full justify-between px-2 py-1.5 h-auto font-normal group",
            isActive && "bg-accent text-accent-foreground"
          )}
        >
          <div className="flex items-center gap-2 text-left flex-1 min-w-0">
            <Folder className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm truncate">{project.name}</span>
          </div>
          <div className="flex items-center gap-1">
            <span
              onClick={(e) => {
                e.stopPropagation();
                onToggleFavorite();
              }}
              className="p-0.5 hover:bg-accent rounded opacity-0 group-hover:opacity-100 transition-opacity cursor-pointer"
            >
              <Star
                className={cn(
                  "h-3 w-3",
                  isFavorite ? "text-yellow-500 fill-yellow-500" : "text-muted-foreground"
                )}
              />
            </span>
            {isExpanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
          </div>
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className="pl-6">
        <div className="space-y-0.5 py-1">
          {isBoardsLoading && shouldFetchBoards && (
            <div className="pl-7 pr-2 py-1 text-xs text-muted-foreground flex items-center gap-2">
              <Loader2 className="h-3 w-3 animate-spin" />
              Loading boards...
            </div>
          )}

          {boardsError && shouldFetchBoards && !isBoardsLoading && (
            <div className="pl-7 pr-2 py-1 text-xs text-destructive">
              Failed to load boards
            </div>
          )}

          {!isBoardsLoading && !boardsError &&
            (boardsData ?? []).map((board) => {
              const boardTasks = tasksByBoard.get(board.id) ?? [];
              const params = new URLSearchParams({ board: board.id });
              const isActive =
                location.pathname === `/projects/${project.id}/tasks` &&
                location.search.includes(`board=${board.id}`);
              return (
                <Link
                  key={board.id}
                  to={{
                    pathname: `/projects/${project.id}/tasks`,
                    search: params.toString(),
                  }}
                  className={cn(
                    'block pl-7 pr-2 py-1 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
                    isActive && 'bg-accent text-accent-foreground'
                  )}
                >
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate" title={board.name}>
                      {board.name}
                    </span>
                    <span className="text-[10px] uppercase text-muted-foreground">
                      {boardTasks.length}
                    </span>
                  </div>
                </Link>
              );
            })}

          {!isTasksLoading && !tasksError && unassignedTasks.length > 0 && (
            <Link
              to={{
                pathname: `/projects/${project.id}/tasks`,
                search: 'board=unassigned',
              }}
              className={cn(
                'block pl-7 pr-2 py-1 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
                location.pathname === `/projects/${project.id}/tasks` &&
                  location.search.includes('board=unassigned') &&
                  'bg-accent text-accent-foreground'
              )}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="truncate" title="Unassigned">
                  Unassigned
                </span>
                <span className="text-[10px] uppercase text-muted-foreground">
                  {unassignedTasks.length}
                </span>
              </div>
            </Link>
          )}

          {/* Project Controller / Master Control */}
          <Link
            to={`/projects/${project.id}/control`}
            className={cn(
              'flex items-center gap-2 pl-5 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground mt-2 border-t pt-2',
              location.pathname === `/projects/${project.id}/control` &&
                'bg-accent text-accent-foreground'
            )}
          >
            <Bot className="h-3 w-3 text-purple-500" />
            <span className="font-medium">Controller</span>
          </Link>

          {/* CRM Board Link */}
          <Link
            to={`/projects/${project.id}/crm`}
            className={cn(
              'flex items-center gap-2 pl-5 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
              location.pathname === `/projects/${project.id}/crm` &&
                'bg-accent text-accent-foreground'
            )}
          >
            <Users className="h-3 w-3 text-muted-foreground" />
            <span>CRM</span>
          </Link>

          {/* Social Media Link */}
          <Link
            to={`/projects/${project.id}/social`}
            className={cn(
              'flex items-center gap-2 pl-5 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
              location.pathname === `/projects/${project.id}/social` &&
                'bg-accent text-accent-foreground'
            )}
          >
            <Share2 className="h-3 w-3 text-muted-foreground" />
            <span>Social</span>
          </Link>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

export function Sidebar({ className }: SidebarProps) {
  const location = useLocation();
  const { projectId } = useParams<{ projectId: string }>();
  const { user } = useAuth();
  const { favorites, addFavorite, removeFavorite, isFavorite } = useCommandStore();
  const {
    data: projects = [],
    isLoading: isProjectsLoading,
    error: projectsError,
    refetch: refetchProjects,
  } = useQuery<Project[], Error>({
    queryKey: ['projects'],
    queryFn: projectsApi.getAll,
  });
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(
    new Set(projectId ? [projectId] : [])
  );

  // Only admins can create projects
  const isAdmin = user?.is_admin ?? false;

  useEffect(() => {
    if (!projectId) return;
    setExpandedProjects((prev) => {
      if (prev.has(projectId)) {
        return prev;
      }
      const next = new Set(prev);
      next.add(projectId);
      return next;
    });
  }, [projectId]);

  const toggleProject = (id: string) => {
    setExpandedProjects(prev => {
      const newSet = new Set(prev);
      if (newSet.has(id)) {
        newSet.delete(id);
      } else {
        newSet.add(id);
      }
      return newSet;
    });
  };

  const handleCreateProject = async () => {
    const existingIds = new Set(projects.map((project) => project.id));

    try {
      const result = await showProjectForm();
      if (result === 'saved') {
        const { data: updatedProjects } = await refetchProjects();

        if (updatedProjects && updatedProjects.length > 0) {
          const newProject = updatedProjects.find(
            (project) => !existingIds.has(project.id)
          );

          if (newProject) {
            setExpandedProjects((prev) => {
              const next = new Set(prev);
              next.add(newProject.id);
              return next;
            });
          }
        }
      }
    } catch (error) {
      console.error('Failed to create project from sidebar:', error);
    }
  };

  const [globalViewsExpanded, setGlobalViewsExpanded] = useState(false);

  // Filter navigation items based on user role
  const filteredPrimaryNav = PRIMARY_NAV_ITEMS.filter((item) => {
    if (item.adminOnly && !isAdmin) return false;
    if (item.memberOnly && isAdmin) return false;
    return true;
  });

  return (
    <div className={cn("flex flex-col h-full bg-muted/30 border-r", className)}>
      {/* Primary Navigation */}
      <div className="p-3 border-b">
        <div className="space-y-1">
          {filteredPrimaryNav.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname.startsWith(item.to);

            return (
              <Link key={item.id} to={item.to}>
                <Button
                  variant="ghost"
                  className={cn(
                    "w-full justify-start px-3 py-2 h-auto font-medium",
                    isActive && "bg-accent text-accent-foreground",
                    item.id === 'nora' && "bg-purple-50 hover:bg-purple-100 dark:bg-purple-950/30 dark:hover:bg-purple-950/50"
                  )}
                >
                  <Icon className={cn(
                    "h-4 w-4 mr-3",
                    item.id === 'nora' && "text-purple-600"
                  )} />
                  <span className="text-sm">{item.label}</span>
                  {item.id === 'nora' && (
                    <span className="ml-auto text-[10px] bg-purple-600 text-white px-1.5 py-0.5 rounded">
                      ADMIN
                    </span>
                  )}
                </Button>
              </Link>
            );
          })}
        </div>
      </div>

      {/* Global Views - Admin Only */}
      {isAdmin && (
        <div className="border-b">
          <Collapsible open={globalViewsExpanded} onOpenChange={setGlobalViewsExpanded}>
            <CollapsibleTrigger asChild>
              <Button
                variant="ghost"
                className="w-full justify-between px-3 py-2 h-auto font-medium text-muted-foreground hover:text-foreground"
              >
                <div className="flex items-center">
                  <BarChart3 className="h-4 w-4 mr-3" />
                  <span className="text-sm">Global Views</span>
                </div>
                {globalViewsExpanded ? (
                  <ChevronDown className="h-4 w-4" />
                ) : (
                  <ChevronRight className="h-4 w-4" />
                )}
              </Button>
            </CollapsibleTrigger>
            <CollapsibleContent className="px-3 pb-2">
              <div className="space-y-1 pl-4 border-l border-muted ml-2">
                {GLOBAL_VIEW_ITEMS.map((item) => {
                  const Icon = item.icon;
                  const isActive = location.pathname === item.to;

                  return (
                    <Link key={item.id} to={item.to}>
                      <Button
                        variant="ghost"
                        className={cn(
                          "w-full justify-start px-2 py-1.5 h-auto font-normal text-sm",
                          isActive && "bg-accent text-accent-foreground"
                        )}
                      >
                        <Icon className="h-3.5 w-3.5 mr-2 text-muted-foreground" />
                        {item.label}
                      </Button>
                    </Link>
                  );
                })}
              </div>
            </CollapsibleContent>
          </Collapsible>
        </div>
      )}

      {/* Favorites Section */}
      {favorites.length > 0 && (
        <div className="border-b">
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
                      projectId === proj.id && "bg-accent text-accent-foreground"
                    )}
                  >
                    <Star className="h-4 w-4 mr-2 text-yellow-500 fill-yellow-500" />
                    <span className="text-sm truncate">{proj.name}</span>
                  </Button>
                </Link>
              );
            })}
          </div>
        </div>
      )}

      {/* Projects Section */}
      <div className="flex-1 flex flex-col overflow-hidden min-h-0">
        <div className="px-3 py-2 flex items-center justify-between flex-shrink-0">
          <span className="text-xs font-semibold text-muted-foreground uppercase tracking-wider">
            Projects
          </span>
          {isAdmin && (
            <Button
              variant="ghost"
              size="sm"
              className="h-7 w-7 p-0 hover:bg-accent"
              onClick={handleCreateProject}
              >
              <Plus className="h-4 w-4" />
            </Button>
          )}
        </div>

        <ScrollArea className="flex-1 px-3 min-h-0">
          <div className="space-y-1">
            {isProjectsLoading ? (
              <div className="py-4 text-xs text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-3 w-3 animate-spin" />
                Loading projects...
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
            )}
          </div>
        </ScrollArea>
      </div>

      {/* External Links */}
      <div className="p-3 border-t">
        <div className="space-y-1">
          {EXTERNAL_LINKS.map((item) => {
            const Icon = item.icon;

            if (item.external) {
              return (
                <a
                  key={item.href}
                  href={item.href}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block"
                >
                  <Button
                    variant="ghost"
                    className="w-full justify-start px-3 py-2 h-auto text-muted-foreground hover:text-foreground"
                  >
                    <Icon className="h-4 w-4 mr-3" />
                    <span className="text-sm">{item.label}</span>
                  </Button>
                </a>
              );
            }

            // Internal action (opens modal)
            return (
              <Button
                key={item.label}
                variant="ghost"
                className="w-full justify-start px-3 py-2 h-auto text-muted-foreground hover:text-foreground"
                onClick={() => {
                  if (item.action) {
                    NiceModal.show(item.action);
                  }
                }}
              >
                <Icon className="h-4 w-4 mr-3" />
                <span className="text-sm">{item.label}</span>
              </Button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
