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
  FolderClosed,
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
  Network,
  Globe,
  Activity,
  Building2,
  UserCircle,
  GripVertical,
  MoreHorizontal,
  Pencil,
  Trash2,
  FolderMinus,
  Target,
  TrendingUp,
  Package,
  Calendar,
  FileText,
  LayoutDashboard,
  Receipt,
  ExternalLink,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query';
import { projectsApi, organizationsApi, projectFoldersApi, tasksApi } from '@/lib/api';
import type { SidebarTree, SidebarOrg, SidebarClient as SidebarClientType, SidebarProject as SidebarProjectType, SidebarProjectFolder as SidebarProjectFolderType } from '@/lib/api';
import { showProjectForm } from '@/lib/modals';
import type { Project, ProjectBoard, TaskWithAttemptStatus } from 'shared/types';
import { useCommandStore } from '@/stores/useCommandStore';
import { useProjectOrderStore } from '@/stores/useProjectOrderStore';
import NiceModal from '@ebay/nice-modal-react';
import { useAuth } from '@/contexts/AuthContext';
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  useDroppable,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  useSortable,
  arrayMove,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

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
  { label: 'Topsi Platform', icon: Network, to: '/topsi', id: 'topsi' },
  { label: 'Projects', icon: FolderOpen, to: '/projects', id: 'projects' },
  { label: 'My Tasks', icon: ListTodo, to: '/my-tasks', id: 'my-tasks', memberOnly: true },
  { label: 'VIBELAND', icon: Box, to: '/virtual-environment', id: 'virtual-environment' },
  { label: 'Settings', icon: Settings, to: '/settings', id: 'settings' },
];

// Global views - admin only, collapsible
const GLOBAL_VIEW_ITEMS: NavItem[] = [
  { label: 'All Tasks', icon: ListTodo, to: '/global-tasks', id: 'global-tasks', adminOnly: true },
  { label: 'People', icon: Users, to: '/people', id: 'people', adminOnly: true },
  { label: 'Proposals', icon: FileText, to: '/proposals', id: 'proposals', adminOnly: true },
  { label: 'Command Center', icon: LayoutDashboard, to: '/command-center', id: 'command-center', adminOnly: true },
  { label: 'Invoices', icon: Receipt, to: '/invoices', id: 'invoices', adminOnly: true },
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

// ============================================================================
// HealthDot — colored indicator for project/client/org health status
// ============================================================================

function HealthDot({ status }: { status?: string }) {
  if (!status) return null;
  const color =
    status === 'critical'
      ? 'bg-red-500'
      : status === 'warning'
        ? 'bg-yellow-500'
        : 'bg-green-500';
  return (
    <span
      className={cn('inline-block w-1.5 h-1.5 rounded-full shrink-0', color)}
      title={`Health: ${status}`}
    />
  );
}

// ============================================================================
// CrmSidebarLinks — expandable CRM sub-navigation for a project
// ============================================================================

function CrmSidebarLinks({
  projectId,
  location,
  indent = 'pl-5',
}: {
  projectId: string;
  location: ReturnType<typeof useLocation>;
  indent?: string;
}) {
  const isCrmActive = location.pathname.startsWith(`/projects/${projectId}/crm`);
  const [expanded, setExpanded] = useState(isCrmActive);

  const crmLinks = [
    { label: 'Overview', to: `/projects/${projectId}/crm/overview`, icon: BarChart3 },
    { label: 'Sales Pipeline', to: `/projects/${projectId}/crm/sales`, icon: TrendingUp },
    { label: 'Client Delivery', to: `/projects/${projectId}/crm/delivery`, icon: Package },
    { label: 'Contacts', to: `/projects/${projectId}/crm`, icon: Users },
    { label: 'Conferences', to: `/projects/${projectId}/crm/conferences`, icon: Calendar },
  ];

  return (
    <Collapsible open={expanded} onOpenChange={setExpanded}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            `flex items-center gap-2 ${indent} pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground w-full text-left`,
            isCrmActive && 'text-accent-foreground'
          )}
        >
          <Users className="h-3 w-3 text-muted-foreground" />
          <span className="flex-1">CRM</span>
          {expanded ? (
            <ChevronDown className="h-3 w-3" />
          ) : (
            <ChevronRight className="h-3 w-3" />
          )}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="space-y-0.5">
          {crmLinks.map((link) => {
            const Icon = link.icon;
            const isActive = link.to === `/projects/${projectId}/crm`
              ? location.pathname === link.to
              : location.pathname === link.to;
            return (
              <Link
                key={link.to}
                to={link.to}
                className={cn(
                  `flex items-center gap-2 ${indent} pl-7 pr-2 py-1 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground`,
                  isActive && 'bg-accent text-accent-foreground'
                )}
              >
                <Icon className="h-3 w-3 text-muted-foreground" />
                <span>{link.label}</span>
              </Link>
            );
          })}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// ProjectFolder — existing component for rendering leaf-level project items
// ============================================================================

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

          {/* CRM Section */}
          <CrmSidebarLinks projectId={project.id} location={location} indent="pl-5" />

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

// ============================================================================
// SortableSidebarProjectFolder — drag-sortable expandable project with boards
// ============================================================================

function SortableSidebarProjectFolder({
  project,
  projectId,
  isExpanded,
  onToggle,
  folderId,
  queryClient,
}: {
  project: SidebarProjectType;
  projectId?: string;
  isExpanded: boolean;
  onToggle: () => void;
  folderId?: string;
  queryClient?: QueryClient;
}) {
  const location = useLocation();
  const isActive = project.id === projectId || location.pathname.includes(`/projects/${project.id}`);

  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: project.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const shouldFetchBoards =
    isExpanded || location.pathname.includes(`/projects/${project.id}`);

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

  const handleRemoveFromFolder = async () => {
    if (!folderId) return;
    try {
      await projectFoldersApi.removeProject(folderId, project.id);
      queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
    } catch (err) {
      console.error('Failed to remove project from folder:', err);
    }
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        'group/sortable rounded-sm',
        isDragging && 'opacity-50 z-50'
      )}
    >
      <Collapsible open={isExpanded} onOpenChange={onToggle}>
        <div className="flex items-center">
          <button
            {...attributes}
            {...listeners}
            className="p-0.5 opacity-0 group-hover/sortable:opacity-100 transition-opacity cursor-grab active:cursor-grabbing shrink-0"
            tabIndex={-1}
          >
            <GripVertical className="h-3 w-3 text-muted-foreground" />
          </button>
          <CollapsibleTrigger asChild>
            <button
              className={cn(
                'flex items-center gap-1.5 px-1.5 py-1 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground flex-1 min-w-0 text-left',
                isActive && 'bg-accent text-accent-foreground'
              )}
            >
              <HealthDot status={project.health_status} />
              <Folder className="h-3 w-3 text-muted-foreground shrink-0" />
              <span className="truncate flex-1">{project.name}</span>
              {project.active_issues_count != null && project.active_issues_count > 0 && (
                <span className="text-[9px] px-1 py-0.5 rounded bg-red-100 text-red-700 dark:bg-red-950 dark:text-red-300 shrink-0">
                  {project.active_issues_count}
                </span>
              )}
              {isExpanded ? (
                <ChevronDown className="h-3 w-3 shrink-0" />
              ) : (
                <ChevronRight className="h-3 w-3 shrink-0" />
              )}
            </button>
          </CollapsibleTrigger>
          {folderId && (
            <button
              onClick={handleRemoveFromFolder}
              className="p-0.5 opacity-0 group-hover/sortable:opacity-100 transition-opacity shrink-0 mr-0.5 rounded hover:bg-accent"
              title="Remove from folder"
            >
              <FolderMinus className="h-3 w-3 text-muted-foreground" />
            </button>
          )}
        </div>
        <CollapsibleContent className="pl-6">
          <div className="space-y-0.5 py-1">
            {isBoardsLoading && shouldFetchBoards && (
              <div className="pl-2 pr-2 py-1 text-xs text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-3 w-3 animate-spin" />
                Loading boards...
              </div>
            )}

            {boardsError && shouldFetchBoards && !isBoardsLoading && (
              <div className="pl-2 pr-2 py-1 text-xs text-destructive">
                Failed to load boards
              </div>
            )}

            {!isBoardsLoading && !boardsError &&
              boardsData.map((board) => {
                const params = new URLSearchParams({ board: board.id });
                const isBoardActive =
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
                      'block pl-2 pr-2 py-1 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
                      isBoardActive && 'bg-accent text-accent-foreground'
                    )}
                  >
                    <span className="truncate" title={board.name}>
                      {board.name}
                    </span>
                  </Link>
                );
              })}

            {/* Controller */}
            <Link
              to={`/projects/${project.id}/control`}
              className={cn(
                'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground mt-1 border-t pt-2',
                location.pathname === `/projects/${project.id}/control` &&
                  'bg-accent text-accent-foreground'
              )}
            >
              <Bot className="h-3 w-3 text-purple-500" />
              <span className="font-medium">Controller</span>
            </Link>

            {/* CRM Section */}
            <CrmSidebarLinks projectId={project.id} location={location} indent="pl-2" />

            {/* Social */}
            <Link
              to={`/projects/${project.id}/social`}
              className={cn(
                'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
                location.pathname === `/projects/${project.id}/social` &&
                  'bg-accent text-accent-foreground'
              )}
            >
              <Share2 className="h-3 w-3 text-muted-foreground" />
              <span>Social</span>
            </Link>

            {/* Knowledge */}
            <Link
              to={`/projects/${project.id}/knowledge`}
              className={cn(
                'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
                location.pathname === `/projects/${project.id}/knowledge` &&
                  'bg-accent text-accent-foreground'
              )}
            >
              <BookOpen className="h-3 w-3 text-muted-foreground" />
              <span>Knowledge</span>
              {project.knowledge_completeness != null && (
                <span className="text-[9px] text-muted-foreground ml-auto">
                  {Math.round(project.knowledge_completeness * 100)}%
                </span>
              )}
            </Link>
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}

// ============================================================================
// SidebarFolderGroup — collapsible folder that contains nested projects
// ============================================================================

function SidebarFolderGroup({
  folder,
  projectId,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  folder: SidebarProjectFolderType;
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient?: QueryClient;
}) {
  const hasActiveProject = folder.projects.some((p) => p.id === projectId);
  const [expanded, setExpanded] = useState(hasActiveProject);

  // Make this folder a droppable target for projects
  const { setNodeRef, isOver } = useDroppable({
    id: `folder:${folder.id}`,
  });

  useEffect(() => {
    if (hasActiveProject && !expanded) {
      setExpanded(true);
    }
  }, [hasActiveProject]);

  const handleRename = async () => {
    const newName = window.prompt('Rename folder:', folder.name);
    if (!newName?.trim() || newName.trim() === folder.name) return;
    try {
      await projectFoldersApi.update(folder.id, { name: newName.trim() });
      queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
    } catch (err) {
      console.error('Failed to rename folder:', err);
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(`Delete folder "${folder.name}"? Projects inside will be ungrouped.`)) return;
    try {
      await projectFoldersApi.delete(folder.id);
      queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
    } catch (err) {
      console.error('Failed to delete folder:', err);
    }
  };

  return (
    <div ref={setNodeRef}>
      <Collapsible open={expanded} onOpenChange={setExpanded}>
        <div className={cn(
          "flex items-center group/folder rounded-sm transition-colors",
          isOver && "bg-amber-100 dark:bg-amber-950/40 ring-1 ring-amber-400"
        )}>
          <CollapsibleTrigger asChild>
            <Button
              variant="ghost"
              className="flex-1 justify-between px-2 py-1 h-auto font-normal text-xs min-w-0"
            >
              <div className="flex items-center gap-1.5 min-w-0">
                {expanded ? (
                  <FolderOpen className="h-3.5 w-3.5 text-amber-500 shrink-0" />
                ) : (
                  <FolderClosed className="h-3.5 w-3.5 text-amber-500 shrink-0" />
                )}
                <span className="truncate">{folder.name}</span>
              </div>
              <div className="flex items-center gap-1">
                <span className="text-[10px] text-muted-foreground">
                  {folder.projects.length}
                </span>
                {expanded ? (
                  <ChevronDown className="h-3 w-3" />
                ) : (
                  <ChevronRight className="h-3 w-3" />
                )}
              </div>
            </Button>
          </CollapsibleTrigger>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <button
                className="p-0.5 opacity-0 group-hover/folder:opacity-100 transition-opacity shrink-0 mr-1 rounded hover:bg-accent"
                onClick={(e) => e.stopPropagation()}
              >
                <MoreHorizontal className="h-3 w-3 text-muted-foreground" />
              </button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-36">
              <DropdownMenuItem onClick={handleRename}>
                <Pencil className="h-3 w-3 mr-2" />
                Rename
              </DropdownMenuItem>
              <DropdownMenuItem onClick={handleDelete} className="text-destructive focus:text-destructive">
                <Trash2 className="h-3 w-3 mr-2" />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
        <CollapsibleContent className="pl-4">
          <div className="space-y-0.5 py-0.5">
            <SortableProjectList
              scopeKey={`folder:${folder.id}`}
              projects={folder.projects}
              projectId={projectId}
              expandedProjects={expandedProjects}
              onToggleProject={onToggleProject}
              folderId={folder.id}
              queryClient={queryClient}
            />
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}

// ============================================================================
// SortableProjectList — DnD wrapper for a list of projects within a scope
// ============================================================================

function SortableProjectList({
  scopeKey,
  projects,
  folders,
  projectId,
  expandedProjects,
  onToggleProject,
  folderId,
  queryClient,
}: {
  scopeKey: string;
  projects: SidebarProjectType[];
  folders?: SidebarProjectFolderType[];
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  folderId?: string;
  queryClient?: QueryClient;
}) {
  const { getOrderedProjects, setOrder } = useProjectOrderStore();

  // Deduplicate by project ID (keep first occurrence)
  const seen = new Set<string>();
  const uniqueProjects = projects.filter(p => {
    if (seen.has(p.id)) return false;
    seen.add(p.id);
    return true;
  });

  const orderedProjects = getOrderedProjects(scopeKey, uniqueProjects);

  const sensors = useSensors(
    useSensor(PointerSensor, { activationConstraint: { distance: 5 } })
  );

  const handleDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const overId = String(over.id);

    // Check if dropped onto a folder
    if (overId.startsWith('folder:')) {
      const folderId = overId.replace('folder:', '');
      const projectDragId = String(active.id);
      try {
        await projectFoldersApi.addProject(folderId, projectDragId);
        queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
      } catch (err) {
        console.error('Failed to add project to folder:', err);
      }
      return;
    }

    // Otherwise, it's a reorder within the list
    const oldIndex = orderedProjects.findIndex((p) => p.id === active.id);
    const newIndex = orderedProjects.findIndex((p) => p.id === over.id);
    if (oldIndex === -1 || newIndex === -1) return;

    const newOrder = arrayMove(orderedProjects, oldIndex, newIndex);
    setOrder(scopeKey, newOrder.map((p) => p.id));
  };

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={handleDragEnd}
    >
      {/* Render folders as droppable targets inside the DndContext */}
      {folders && folders.map((f) => (
        <SidebarFolderGroup
          key={f.id}
          folder={f}
          projectId={projectId}
          expandedProjects={expandedProjects}
          onToggleProject={onToggleProject}
          queryClient={queryClient}
        />
      ))}
      {/* Render ungrouped projects as sortable items */}
      <SortableContext
        items={orderedProjects.map((p) => p.id)}
        strategy={verticalListSortingStrategy}
      >
        {orderedProjects.map((project) => (
          <SortableSidebarProjectFolder
            key={project.id}
            project={project}
            projectId={projectId}
            isExpanded={expandedProjects.has(project.id)}
            onToggle={() => onToggleProject(project.id)}
            folderId={folderId}
            queryClient={queryClient}
          />
        ))}
      </SortableContext>
    </DndContext>
  );
}

// ============================================================================
// ClientGroup — renders a client with expand/collapse and project list
// ============================================================================

function ClientGroup({
  client,
  orgId,
  projectId,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  client: SidebarClientType;
  orgId: string;
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient?: QueryClient;
}) {
  const hasActiveProject = client.projects.some(
    (p) => p.id === projectId
  ) || (client.folders || []).some((f) => f.projects.some((p) => p.id === projectId));
  const [expanded, setExpanded] = useState(hasActiveProject);

  useEffect(() => {
    if (hasActiveProject && !expanded) {
      setExpanded(true);
    }
  }, [hasActiveProject]);

  return (
    <Collapsible open={expanded} onOpenChange={setExpanded}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          className="w-full justify-between px-2 py-1 h-auto font-normal text-xs"
        >
          <div className="flex items-center gap-1.5 min-w-0 group/client">
            <HealthDot status={client.health_status} />
            <UserCircle className="h-3.5 w-3.5 text-blue-500 shrink-0" />
            <span className="truncate">{client.name}</span>
            <Link
              to={`/organizations/${orgId}?tab=projects&client=${client.id}`}
              onClick={(e) => e.stopPropagation()}
              className="opacity-0 group-hover/client:opacity-100 ml-0.5 shrink-0 text-muted-foreground hover:text-foreground"
              title={`${client.name} in org profile`}
            >
              <ExternalLink className="h-3 w-3" />
            </Link>
          </div>
          <div className="flex items-center gap-1">
            {client.active_issues_count != null && client.active_issues_count > 0 && (
              <span className="text-[9px] px-1 py-0.5 rounded bg-red-100 text-red-700 dark:bg-red-950 dark:text-red-300">
                {client.active_issues_count}
              </span>
            )}
            <span className="text-[10px] text-muted-foreground">
              {client.projects.length + (client.folders || []).reduce((sum, f) => sum + f.projects.length, 0)}
            </span>
            {expanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
          </div>
        </Button>
      </CollapsibleTrigger>
      <CollapsibleContent className="pl-4">
        <div className="space-y-0.5 py-0.5">
          <SortableProjectList
            scopeKey={`client:${client.id}`}
            projects={client.projects}
            folders={client.folders}
            projectId={projectId}
            expandedProjects={expandedProjects}
            onToggleProject={onToggleProject}
            queryClient={queryClient}
          />
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// OrgSection — renders org name as section header with internal projects + clients
// ============================================================================

function OrgSection({
  org,
  projectId,
  isAdmin,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  org: SidebarOrg;
  projectId?: string;
  isAdmin: boolean;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient: QueryClient;
}) {
  const location = useLocation();
  const hasActiveProject =
    org.internal_projects.some((p) => p.id === projectId) ||
    (org.internal_folders || []).some((f) => f.projects.some((p) => p.id === projectId)) ||
    org.clients.some((c) =>
      c.projects.some((p) => p.id === projectId) ||
      (c.folders || []).some((f) => f.projects.some((p) => p.id === projectId))
    );
  const [expanded, setExpanded] = useState(hasActiveProject || org.role === 'admin');

  useEffect(() => {
    if (hasActiveProject && !expanded) {
      setExpanded(true);
    }
  }, [hasActiveProject]);

  return (
    <div>
      <div className="flex items-center px-2 py-1.5">
        <button
          onClick={() => setExpanded(!expanded)}
          className="shrink-0 mr-1 text-muted-foreground hover:text-foreground"
        >
          {expanded ? <ChevronDown className="h-3.5 w-3.5" /> : <ChevronRight className="h-3.5 w-3.5" />}
        </button>
        <Link
          to={`/organizations/${org.id}`}
          className={cn(
            'flex items-center gap-1.5 min-w-0 font-medium text-xs uppercase tracking-wider text-muted-foreground hover:text-foreground transition-colors',
            location.pathname === `/organizations/${org.id}` && 'text-foreground'
          )}
          title={`${org.name} overview`}
        >
          <HealthDot status={org.health_status} />
          <Building2 className="h-3.5 w-3.5 shrink-0" />
          <span className="truncate">{org.name}</span>
        </Link>
      </div>
      {expanded && (
      <div className="pl-2">
        <div className="space-y-0.5">
          {/* Org-level CRM pipelines link */}
          <div className="px-1 py-1">
            <Link
              to={`/organizations/${org.id}?tab=pipelines`}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent hover:text-accent-foreground',
                location.search.includes('tab=pipelines') && location.pathname === `/organizations/${org.id}` &&
                  'bg-accent text-accent-foreground'
              )}
            >
              <Target className="h-3 w-3 text-amber-500 shrink-0" />
              <span>Pipelines</span>
            </Link>
          </div>

          {/* Internal projects and folders (no client) */}
          {(org.internal_projects.length > 0 || (org.internal_folders || []).length > 0) && (
            <div className="space-y-0.5 py-0.5">
              {org.clients.length > 0 && (
                <div className="px-2 py-0.5 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                  Internal
                </div>
              )}
              <SortableProjectList
                scopeKey={`org:${org.id}:internal`}
                projects={org.internal_projects}
                folders={org.internal_folders}
                projectId={projectId}
                expandedProjects={expandedProjects}
                onToggleProject={onToggleProject}
                queryClient={queryClient}
              />
            </div>
          )}

          {/* Client groups */}
          {org.clients.map((client) => (
            <ClientGroup
              key={client.id}
              client={client}
              orgId={org.id}
              projectId={projectId}
              expandedProjects={expandedProjects}
              onToggleProject={onToggleProject}
              queryClient={queryClient}
            />
          ))}

          {/* New Client / New Folder buttons (org admin only) */}
          {isAdmin && org.role === 'admin' && (
            <>
              <Button
                variant="ghost"
                className="w-full justify-start px-2 py-1 h-auto text-xs text-muted-foreground hover:text-foreground"
                onClick={() => {
                  // TODO: Open client creation dialog
                }}
              >
                <Plus className="h-3 w-3 mr-1.5" />
                New Client
              </Button>
              <Button
                variant="ghost"
                className="w-full justify-start px-2 py-1 h-auto text-xs text-muted-foreground hover:text-foreground"
                onClick={async () => {
                  const name = window.prompt('Folder name:');
                  if (!name?.trim()) return;
                  try {
                    await projectFoldersApi.create(org.id, { name: name.trim() });
                    queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
                  } catch (err) {
                    console.error('Failed to create folder:', err);
                  }
                }}
              >
                <Plus className="h-3 w-3 mr-1.5" />
                New Folder
              </Button>
            </>
          )}
        </div>
      </div>
      )}
    </div>
  );
}

// ============================================================================
// Main Sidebar
// ============================================================================

export function Sidebar({ className }: SidebarProps) {
  const location = useLocation();
  const { projectId } = useParams<{ projectId: string }>();
  const { user } = useAuth();
  const { favorites, addFavorite, removeFavorite, isFavorite } = useCommandStore();
  const queryClient = useQueryClient();

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
    refetch: refetchProjects,
  } = useQuery<Project[], Error>({
    queryKey: ['projects'],
    queryFn: projectsApi.getAll,
    enabled: !!treeError || (!!sidebarTree && sidebarTree.owned_orgs.length === 0 && sidebarTree.member_orgs.length === 0),
  });

  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(
    new Set(projectId ? [projectId] : [])
  );

  // Only admins can create projects
  const isAdmin = user?.is_admin ?? false;

  // Determine if we should show hierarchical or flat view
  const hasTree = sidebarTree && (sidebarTree.owned_orgs.length > 0 || sidebarTree.member_orgs.length > 0);
  const useFlatFallback = !hasTree && !isTreeLoading;

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
    <div className={cn("flex flex-col h-full sidebar-container", className)}>
      {/* Primary Navigation */}
      <div className="p-3 border-b border-border/40">
        <div className="space-y-0.5">
          {filteredPrimaryNav.map((item) => {
            const Icon = item.icon;
            const isActive = location.pathname.startsWith(item.to);

            return (
              <Link key={item.id} to={item.to}>
                <div
                  className={cn(
                    "sidebar-nav-item",
                    isActive && "sidebar-nav-item-active",
                    item.id === 'nora' && !isActive && "bg-purple-50/50 hover:bg-purple-100/50 dark:bg-purple-950/20 dark:hover:bg-purple-950/40",
                    item.id === 'topsi' && !isActive && "bg-cyan-50/50 hover:bg-cyan-100/50 dark:bg-cyan-950/20 dark:hover:bg-cyan-950/40"
                  )}
                >
                  <Icon className={cn(
                    "h-4 w-4",
                    item.id === 'nora' && "text-purple-600",
                    item.id === 'topsi' && "text-cyan-600"
                  )} />
                  <span className="flex-1">{item.label}</span>
                  {item.id === 'nora' && (
                    <span className="text-[10px] bg-purple-600 text-white px-1.5 py-0.5 rounded">
                      ADMIN
                    </span>
                  )}
                  {item.id === 'topsi' && (
                    <span className="text-[10px] bg-cyan-600 text-white px-1.5 py-0.5 rounded">
                      ADMIN
                    </span>
                  )}
                </div>
              </Link>
            );
          })}
        </div>
      </div>

      {/* Global Views - Admin Only */}
      {isAdmin && (
        <div className="border-b border-border/40">
          <Collapsible open={globalViewsExpanded} onOpenChange={setGlobalViewsExpanded}>
            <CollapsibleTrigger asChild>
              <div className="sidebar-nav-item mx-3 my-1.5 justify-between">
                <div className="flex items-center gap-2.5">
                  <BarChart3 className="h-4 w-4" />
                  <span>Global Views</span>
                </div>
                {globalViewsExpanded ? (
                  <ChevronDown className="h-3.5 w-3.5" />
                ) : (
                  <ChevronRight className="h-3.5 w-3.5" />
                )}
              </div>
            </CollapsibleTrigger>
            <CollapsibleContent className="px-3 pb-2">
              <div className="space-y-0.5 pl-4 border-l border-border/40 ml-2">
                {GLOBAL_VIEW_ITEMS.map((item) => {
                  const Icon = item.icon;
                  const isActive = location.pathname === item.to;

                  return (
                    <Link key={item.id} to={item.to}>
                      <div
                        className={cn(
                          "sidebar-nav-item text-xs py-1",
                          isActive && "sidebar-nav-item-active"
                        )}
                      >
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
      )}

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

      {/* Organizations & Projects Section (Hierarchical) */}
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
            {isTreeLoading ? (
              <div className="py-4 text-xs text-muted-foreground flex items-center gap-2">
                <Loader2 className="h-3 w-3 animate-spin" />
                Loading projects...
              </div>
            ) : hasTree ? (
              <>
                {/* Owned organizations */}
                {sidebarTree!.owned_orgs.map((org) => (
                  <OrgSection
                    key={org.id || org.slug}
                    org={org}
                    projectId={projectId}
                    isAdmin={isAdmin}
                    expandedProjects={expandedProjects}
                    onToggleProject={toggleProject}
                    queryClient={queryClient}
                  />
                ))}

                {/* Guest access organizations */}
                {sidebarTree!.member_orgs.length > 0 && (
                  <>
                    <div className="px-2 pt-3 pb-1">
                      <span className="text-[10px] font-semibold text-muted-foreground uppercase tracking-wider">
                        Guest Access
                      </span>
                    </div>
                    {sidebarTree!.member_orgs.map((org) => (
                      <OrgSection
                        key={org.id || org.slug}
                        org={org}
                        projectId={projectId}
                        isAdmin={isAdmin}
                        expandedProjects={expandedProjects}
                        onToggleProject={toggleProject}
                        queryClient={queryClient}
                      />
                    ))}
                  </>
                )}
              </>
            ) : useFlatFallback ? (
              // Fallback: flat project list
              isProjectsLoading ? (
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
              )
            ) : null}
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
