import { Link, useLocation } from 'react-router-dom';
import { Button } from '@/components/ui/button';
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
  Folder,
  Loader2,
  Star,
  Bot,
  Share2,
  BookOpen,
  Package,
  Activity,
  ListTodo,
  GripVertical,
  MoreHorizontal,
  Pencil,
  Trash2,
} from 'lucide-react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { useQuery, type QueryClient } from '@tanstack/react-query';
import { projectsApi } from '@/lib/api';
import type { SidebarProject as SidebarProjectType } from '@/lib/api';
import type { Project, ProjectBoard } from 'shared/types';
import NiceModal from '@ebay/nice-modal-react';
import type { CreateNameDialogResult } from '@/components/dialogs';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useDroppable } from '@dnd-kit/core';
import { HealthDot } from './HealthDot';
// CrmSidebarLinks removed — CRM is now org-scoped only
import { SortableProjectList } from './ProjectList';

// ============================================================================
// ProjectFolder — standalone project card (used in flat fallback list)
// ============================================================================

export function ProjectFolder({
  project,
  isActive,
  isExpanded,
  onToggle,
  isFavorite: isFav,
  onToggleFavorite,
}: {
  project: Project;
  isActive: boolean;
  isExpanded: boolean;
  onToggle: () => void;
  isFavorite: boolean;
  onToggleFavorite: () => void;
}) {
  const location = useLocation();

  const {
    data: boardsData = [],
    isLoading: isBoardsLoading,
  } = useQuery<ProjectBoard[], Error>({
    queryKey: ['projectBoardsSidebar', project.id],
    queryFn: () => projectsApi.listBoards(project.id),
    enabled: isExpanded,
    staleTime: 5 * 60 * 1000,
  });

  return (
    <Collapsible open={isExpanded} onOpenChange={onToggle}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          className={cn(
            "w-full justify-between px-2 py-1.5 h-auto font-normal group",
            isActive && "bg-primary/10 text-foreground"
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
                  isFav ? "text-[hsl(var(--warning))] fill-[hsl(var(--warning))]" : "text-muted-foreground"
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
          {isBoardsLoading ? (
            <div className="pl-2 pr-2 py-1 text-xs text-muted-foreground flex items-center gap-2">
              <Loader2 className="h-3 w-3 animate-spin" />
              Loading boards...
            </div>
          ) : (
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
                    'block pl-2 pr-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                    isBoardActive && 'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <span className="truncate" title={board.name}>
                    {board.name}
                  </span>
                </Link>
              );
            })
          )}

          {/* Controller */}
          <Link
            to={`/projects/${project.id}/control`}
            className={cn(
              'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground mt-1 border-t pt-2 transition-colors',
              location.pathname === `/projects/${project.id}/control` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Bot className="h-3 w-3 text-[hsl(var(--brand))]" />
            <span className="font-medium">Controller</span>
          </Link>

          {/* CRM removed from project sub-nav — CRM is org-scoped (see sidebar org section) */}

          {/* Social */}
          <Link
            to={`/projects/${project.id}/social`}
            className={cn(
              'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/projects/${project.id}/social` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Share2 className="h-3 w-3 text-muted-foreground" />
            <span>Social</span>
          </Link>

          {/* Knowledge */}
          <Link
            to={`/projects/${project.id}/knowledge`}
            className={cn(
              'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/projects/${project.id}/knowledge` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <BookOpen className="h-3 w-3 text-muted-foreground" />
            <span>Knowledge</span>
          </Link>

          {/* Deliverables */}
          <Link
            to={`/projects/${project.id}/deliverables`}
            className={cn(
              'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/projects/${project.id}/deliverables` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Package className="h-3 w-3 text-muted-foreground" />
            <span>Deliverables</span>
          </Link>

          {/* Pulse */}
          <Link
            to={`/projects/${project.id}/pulse`}
            className={cn(
              'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
              location.pathname === `/projects/${project.id}/pulse` && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <Activity className="h-3 w-3 text-muted-foreground" />
            <span>Pulse</span>
          </Link>
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// SortableSidebarProjectFolder — draggable project item in the org tree
// ============================================================================

export function SortableSidebarProjectFolder({
  project,
  projectId,
  isExpanded,
  onToggle,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  project: SidebarProjectType;
  projectId?: string;
  isExpanded: boolean;
  onToggle: () => void;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient?: QueryClient;
}) {
  const location = useLocation();
  const isActive = project.id === projectId || location.pathname.includes(`/projects/${project.id}`);

  // Container projects (empty git_repo_path) show children instead of boards
  const isContainer = project.is_container;
  const hasChildren = project.children && project.children.length > 0;

  const shouldFetchBoards =
    !isContainer && (isExpanded || location.pathname.includes(`/projects/${project.id}`));

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

  // Make container projects droppable targets for reparenting
  const { setNodeRef: setDropRef, isOver } = useDroppable({
    id: `container:${project.id}`,
    disabled: !isContainer,
  });

  const handleRename = async () => {
    try {
      const result = await NiceModal.show('create-name', {
        title: 'Rename Project Group',
        label: 'Group Name',
        placeholder: 'Enter new name...',
        submitText: 'Rename',
      }) as CreateNameDialogResult;
      if (result.name === project.name) return;
      await projectsApi.update(project.id, { name: result.name });
      queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
    } catch {
      // dialog dismissed
    }
  };

  const handleDelete = async () => {
    if (!window.confirm(`Delete "${project.name}"? ${hasChildren ? 'Child projects will be ungrouped.' : ''}`)) return;
    try {
      await projectsApi.delete(project.id);
      queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
    } catch (err) {
      console.error('Failed to delete project:', err);
    }
  };

  return (
    <div
      ref={(node) => {
        setNodeRef(node);
        if (isContainer) setDropRef(node);
      }}
      style={style}
      className={cn(
        'group/sortable rounded-sm',
        isDragging && 'opacity-50 z-50',
        isOver && isContainer && 'bg-accent/60 ring-1 ring-primary/50'
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
          <Link
            to={`/projects/${project.id}`}
            className={cn(
              'flex items-center gap-1.5 px-1.5 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground flex-1 min-w-0 text-left transition-colors',
              isActive && 'bg-primary/10 text-foreground font-medium'
            )}
          >
            <HealthDot status={project.health_status} />
            {isContainer ? (
              isExpanded ? (
                <FolderOpen className="h-3 w-3 text-[hsl(var(--warning))] shrink-0" />
              ) : (
                <FolderClosed className="h-3 w-3 text-[hsl(var(--warning))] shrink-0" />
              )
            ) : (
              <Folder className="h-3 w-3 text-muted-foreground shrink-0" />
            )}
            <span className="truncate flex-1">{project.name}</span>
            {project.active_issues_count != null && project.active_issues_count > 0 && (
              <span className="text-[9px] px-1 py-0.5 rounded bg-destructive/10 text-destructive shrink-0">
                {project.active_issues_count}
              </span>
            )}
            {isContainer && (
              <span className="text-[10px] text-muted-foreground">
                {project.children?.length || 0}
              </span>
            )}
          </Link>
          {isContainer && (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  className="p-0.5 opacity-0 group-hover/sortable:opacity-100 transition-opacity shrink-0 mr-1 rounded hover:bg-accent"
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
                <DropdownMenuItem onClick={handleDelete} className="text-destructive">
                  <Trash2 className="h-3 w-3 mr-2" />
                  Delete
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          )}
          <CollapsibleTrigger asChild>
            <button
              className="p-0.5 hover:bg-accent rounded-sm shrink-0 mr-0.5"
              onClick={(e) => e.stopPropagation()}
            >
              {isExpanded ? (
                <ChevronDown className="h-3 w-3" />
              ) : (
                <ChevronRight className="h-3 w-3" />
              )}
            </button>
          </CollapsibleTrigger>
        </div>
        <CollapsibleContent className="pl-6">
          <div className="space-y-0.5 py-1">
            {/* Container projects: show nested children */}
            {isContainer && hasChildren && (
              <SortableProjectList
                scopeKey={`container:${project.id}`}
                projects={project.children}
                projectId={projectId}
                expandedProjects={expandedProjects}
                onToggleProject={onToggleProject}
                queryClient={queryClient}
              />
            )}

            {/* Regular projects: show boards + sections */}
            {!isContainer && (
              <>
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
                          'block pl-2 pr-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                          isBoardActive && 'bg-primary/10 text-foreground font-medium'
                        )}
                      >
                        <span className="truncate" title={board.name}>
                          {board.name}
                        </span>
                      </Link>
                    );
                  })}

                {/* Tasks link */}
                <Link
                  to={`/projects/${project.id}/tasks`}
                  className={cn(
                    'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                    location.pathname === `/projects/${project.id}/tasks` &&
                      !location.search &&
                      'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <ListTodo className="h-3 w-3 text-muted-foreground" />
                  <span>Tasks</span>
                </Link>

                {/* Controller */}
                <Link
                  to={`/projects/${project.id}/control`}
                  className={cn(
                    'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground mt-1 border-t pt-2 transition-colors',
                    location.pathname === `/projects/${project.id}/control` &&
                      'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <Bot className="h-3 w-3 text-[hsl(var(--brand))]" />
                  <span className="font-medium">Controller</span>
                </Link>

                {/* CRM removed from project sub-nav — CRM is org-scoped (see sidebar org section) */}

                {/* Social Media Link */}
                <Link
                  to={`/projects/${project.id}/social`}
                  className={cn(
                    'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                    location.pathname === `/projects/${project.id}/social` &&
                      'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <Share2 className="h-3 w-3 text-muted-foreground" />
                  <span>Social</span>
                </Link>

                {/* Knowledge */}
                <Link
                  to={`/projects/${project.id}/knowledge`}
                  className={cn(
                    'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                    location.pathname === `/projects/${project.id}/knowledge` &&
                      'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <BookOpen className="h-3 w-3 text-muted-foreground" />
                  <span>Knowledge</span>
                </Link>

                {/* Deliverables */}
                <Link
                  to={`/projects/${project.id}/deliverables`}
                  className={cn(
                    'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                    location.pathname === `/projects/${project.id}/deliverables` &&
                      'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <Package className="h-3 w-3 text-muted-foreground" />
                  <span>Deliverables</span>
                </Link>

                {/* Pulse */}
                <Link
                  to={`/projects/${project.id}/pulse`}
                  className={cn(
                    'flex items-center gap-2 pl-2 pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                    location.pathname === `/projects/${project.id}/pulse` &&
                      'bg-primary/10 text-foreground font-medium'
                  )}
                >
                  <Activity className="h-3 w-3 text-muted-foreground" />
                  <span>Pulse</span>
                </Link>
              </>
            )}
          </div>
        </CollapsibleContent>
      </Collapsible>
    </div>
  );
}
