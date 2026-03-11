import { useEffect, useMemo, useState } from 'react';
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
  Megaphone,
  Users,
  ListTodo,
  BarChart3,
  Network,
  Building2,
  UserCircle,
  GripVertical,
  TrendingUp,
  Package,
  FileText,
  LayoutDashboard,
  Receipt,
  LayoutGrid,
  Brain,
  Bot,
  Share2,
  Calendar,
  Radio,
  Database,
  MoreHorizontal,
  Pencil,
  Trash2,
  Activity,
  Globe,
  GitBranch,
  Headphones,
  Workflow,
  Map,
  Coins,
  Rocket,
  Plug,
  Cpu,
  Inbox,
  BarChart2,
  PhoneIncoming,
  ClipboardList,
} from 'lucide-react';
import { PanelLeftClose, PanelLeftOpen } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';
import { useQuery, useQueryClient, type QueryClient } from '@tanstack/react-query';
import { projectsApi, organizationsApi, stagingApi } from '@/lib/api';
import type {
  SidebarTree,
  SidebarOrg,
  SidebarClient as SidebarClientType,
  SidebarProject as SidebarProjectType,
} from '@/lib/api';
import type { Project, ProjectBoard } from 'shared/types';
import { useCommandStore } from '@/stores/useCommandStore';
import { useProjectOrderStore } from '@/stores/useProjectOrderStore';
import { useViewStore } from '@/stores/useViewStore';
import { useKeyToggleSidebar } from '@/keyboard/hooks';
import { Scope } from '@/keyboard/registry';
import NiceModal from '@ebay/nice-modal-react';
import type { CreateNameDialogResult } from '@/components/dialogs';
import type { ProjectFormDialogResult } from '@/components/dialogs';
import { useAuth } from '@/contexts/AuthContext';
import { useEffectiveRole } from '@/hooks/useEffectiveRole';
import {
  DndContext,
  closestCenter,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core';
import {
  SortableContext,
  useSortable,
  arrayMove,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';
import { useDroppable } from '@dnd-kit/core';

// Helper: count all projects recursively (including children)
function countProjects(projects: SidebarProjectType[]): number {
  return projects.reduce((sum, p) => sum + 1 + countProjects(p.children || []), 0);
}

// Helper: check if a project exists in a tree (recursively checking children)
function isProjectInTree(projects: SidebarProjectType[], projectId: string): boolean {
  return projects.some(p => p.id === projectId || isProjectInTree(p.children || [], projectId));
}

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

// Admin tools — separated visually at top
const ADMIN_NAV_ITEMS: NavItem[] = [
  { label: 'Site Directory', icon: Map, to: '/site-directory', id: 'site-directory', adminOnly: true },
  { label: 'Nora Command', icon: Crown, to: '/nora', id: 'nora', adminOnly: true },
  { label: 'Topsi Platform', icon: Network, to: '/topsi', id: 'topsi', adminOnly: true },
  { label: 'Mission Control', icon: Rocket, to: '/mission-control', id: 'mission-control', adminOnly: true },
  { label: 'Pulse Engine', icon: Activity, to: '/pulse', id: 'pulse', adminOnly: true },
  { label: 'Mesh Network', icon: Globe, to: '/mesh', id: 'mesh', adminOnly: true },
];

// Primary navigation - workspace destinations (user-level pages)
const PRIMARY_NAV_ITEMS: NavItem[] = [
  { label: 'My Projects', icon: FolderOpen, to: '/projects', id: 'projects' },
  { label: 'My Tasks', icon: ListTodo, to: '/my-tasks', id: 'my-tasks', memberOnly: true },
  { label: 'My Workflows', icon: Workflow, to: '/workflows', id: 'workflows' },
  { label: 'Calendar', icon: Calendar, to: '/calendar', id: 'calendar' },
  { label: 'VIBELAND', icon: Box, to: '/virtual-environment', id: 'virtual-environment' },
  { label: 'VIBE', icon: Coins, to: '/vibe', id: 'vibe' },
];

// Management nav — admin-only, collapsible
const MANAGEMENT_NAV_ITEMS: NavItem[] = [
  { label: 'All People', icon: Users, to: '/people', id: 'people', adminOnly: true },
  { label: 'All Companies', icon: Building2, to: '/companies', id: 'companies', adminOnly: true },
  { label: 'Proposals', icon: FileText, to: '/proposals', id: 'proposals', adminOnly: true },
  { label: 'Invoices', icon: Receipt, to: '/invoices', id: 'invoices', adminOnly: true },
  { label: 'Call Intake', icon: PhoneIncoming, to: '/call-intake', id: 'call-intake', adminOnly: true },
  { label: 'Reports', icon: ClipboardList, to: '/business-reports', id: 'business-reports', adminOnly: true },
  { label: 'Command Center', icon: LayoutDashboard, to: '/command-center', id: 'command-center', adminOnly: true },
  { label: 'Discord Voice', icon: Headphones, to: '/discord', id: 'discord', adminOnly: true },
  { label: 'AI Usage', icon: Cpu, to: '/ai-usage', id: 'ai-usage', adminOnly: true },
];

// Global views - admin only, collapsible
const GLOBAL_VIEW_ITEMS: NavItem[] = [
  { label: 'All Tasks', icon: ListTodo, to: '/global-tasks', id: 'global-tasks', adminOnly: true },
  { label: 'CRM Admin', icon: Users, to: '/crm', id: 'crm', adminOnly: true },
  { label: 'All Social', icon: Megaphone, to: '/social-command', id: 'social-command', adminOnly: true },
];

// Utility nav — pinned to bottom above external links
const UTILITY_NAV_ITEMS: NavItem[] = [
  { label: 'Settings', icon: Settings, to: '/settings', id: 'settings' },
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
      ? 'bg-destructive'
      : status === 'warning'
        ? 'bg-[hsl(var(--warning))]'
        : 'bg-[hsl(var(--success))]';
  return (
    <span
      className={cn('inline-block w-1.5 h-1.5 rounded-full shrink-0', color)}
      title={`Health: ${status}`}
    />
  );
}

// ============================================================================
// OrgCrmSection — collapsible CRM with Overview/Contacts/Pipeline/Deliverables/Social
// ============================================================================

function OrgCrmSection({
  orgId,
  location,
}: {
  orgId: string;
  location: ReturnType<typeof useLocation>;
}) {
  const orgBase = `/organizations/${orgId}`;
  const isCrmActive =
    location.pathname === orgBase ||
    location.pathname.startsWith(`${orgBase}/crm`);
  const [open, setOpen] = useState(isCrmActive);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1.5 w-full px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            isCrmActive && 'text-accent-foreground font-medium'
          )}
        >
          <Users className="h-3 w-3 shrink-0 text-primary" />
          <span className="flex-1 text-left">CRM</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {[
            { label: 'Overview',     to: `${orgBase}/crm`,              icon: LayoutGrid, color: 'text-muted-foreground',        match: location.pathname === `${orgBase}/crm` },
            { label: 'Contacts',     to: `${orgBase}/crm/contacts`,     icon: Users,      color: 'text-primary',                 match: location.pathname === `${orgBase}/crm/contacts` },
            { label: 'Companies',    to: `${orgBase}/crm/companies`,    icon: Building2,  color: 'text-purple-500',              match: location.pathname === `${orgBase}/crm/companies` },
            { label: 'Pipeline',     to: `${orgBase}/crm/pipeline`,     icon: TrendingUp, color: 'text-[hsl(var(--warning))]',   match: location.pathname === `${orgBase}/crm/pipeline` },
            { label: 'Deliverables', to: `${orgBase}/crm/deliverables`, icon: Package,    color: 'text-[hsl(var(--success))]',   match: location.pathname === `${orgBase}/crm/deliverables` },
          ].map(({ label, to, icon: Icon, color, match }) => (
            <Link
              key={label}
              to={to}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                match && 'bg-primary/10 text-foreground font-medium'
              )}
            >
              <Icon className={cn('h-3 w-3 shrink-0', color)} />
              <span>{label}</span>
            </Link>
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// OrgSocialSection — collapsible Social sub-navigation for an org
// ============================================================================

function OrgSocialSection({
  orgId,
  location,
}: {
  orgId: string;
  location: ReturnType<typeof useLocation>;
}) {
  const orgBase = `/organizations/${orgId}`;
  const sp = new URLSearchParams(location.search);
  const isOnSocial = location.pathname === orgBase && sp.get('tab') === 'social';
  const [open, setOpen] = useState(isOnSocial);

  const socialViews = [
    { label: 'Overview',  sv: '',          icon: LayoutGrid,  color: 'text-muted-foreground' },
    { label: 'Accounts',  sv: 'accounts',  icon: Share2,      color: 'text-pink-500' },
    { label: 'Content',   sv: 'content',   icon: FileText,    color: 'text-purple-500' },
    { label: 'Inbox',     sv: 'inbox',     icon: Inbox,       color: 'text-amber-500' },
    { label: 'Analytics', sv: 'analytics', icon: BarChart2,   color: 'text-blue-500' },
  ];

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1.5 w-full px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            isOnSocial && 'text-accent-foreground font-medium'
          )}
        >
          <Share2 className="h-3 w-3 shrink-0 text-pink-500" />
          <span className="flex-1 text-left">Social</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {socialViews.map(({ label, sv, icon: Icon, color }) => {
            const to = sv
              ? `${orgBase}?tab=social&sv=${sv}`
              : `${orgBase}?tab=social`;
            const isActive = isOnSocial && (sp.get('sv') || '') === sv;
            return (
              <Link
                key={label}
                to={to}
                className={cn(
                  'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                  isActive && 'bg-primary/10 text-foreground font-medium'
                )}
              >
                <Icon className={cn('h-3 w-3 shrink-0', color)} />
                <span>{label}</span>
              </Link>
            );
          })}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// OrgIntelligenceSection — collapsible Intelligence sub-navigation for an org
// ============================================================================

function OrgIntelligenceSection({
  orgId,
  location,
}: {
  orgId: string;
  location: ReturnType<typeof useLocation>;
}) {
  const orgBase = `/organizations/${orgId}`;
  const isOnOrgIntel = location.pathname.startsWith(`${orgBase}/intelligence`);
  const [open, setOpen] = useState(isOnOrgIntel);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-1.5 w-full px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            isOnOrgIntel && 'text-accent-foreground font-medium'
          )}
        >
          <Brain className="h-3 w-3 shrink-0 text-[hsl(var(--success))]" />
          <span className="flex-1 text-left">Intelligence</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {[
            { label: 'Overview',      icon: Brain,         color: 'text-[hsl(var(--success))]', to: `${orgBase}/intelligence`,              match: location.pathname === `${orgBase}/intelligence` },
            { label: 'Data Sources',  icon: Database,       color: 'text-primary',                to: `${orgBase}/intelligence/data-sources`, match: location.pathname === `${orgBase}/intelligence/data-sources` },
            { label: 'Artifacts',     icon: FileText,      color: 'text-[hsl(var(--brand))]',    to: `${orgBase}/intelligence/artifacts`,    match: location.pathname === `${orgBase}/intelligence/artifacts` },
            { label: 'Workflows',     icon: GitBranch,     color: 'text-purple-500',             to: `${orgBase}/intelligence/workflows`,    match: location.pathname === `${orgBase}/intelligence/workflows` },
            { label: 'Pulse',         icon: Radio,         color: 'text-[hsl(var(--warning))]',  to: `${orgBase}/intelligence/pulse`,        match: location.pathname === `${orgBase}/intelligence/pulse` },
            { label: 'Topology',      icon: Network,       color: 'text-[hsl(var(--info))]',     to: `${orgBase}/intelligence/topology`,     match: location.pathname === `${orgBase}/intelligence/topology` },
          ].map(({ label, to, icon: Icon, color, match }) => (
            <Link
              key={label}
              to={to}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                match && 'bg-primary/10 text-foreground font-medium'
              )}
            >
              <Icon className={cn('h-3 w-3 shrink-0', color)} />
              <span>{label}</span>
            </Link>
          ))}
        </div>
      </CollapsibleContent>
    </Collapsible>
  );
}

// ============================================================================
// CrmSidebarLinks — project-level CRM links (collapsible)
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
  const crmLinks = useMemo(
    () => [
      { label: 'Overview',        to: `/projects/${projectId}/crm/overview`,     icon: BarChart3  },
      { label: 'Sales Pipeline',  to: `/projects/${projectId}/crm/sales`,        icon: TrendingUp },
      { label: 'Client Delivery', to: `/projects/${projectId}/crm/delivery`,     icon: Package    },
      { label: 'Contacts',        to: `/projects/${projectId}/crm`,              icon: Users      },
      { label: 'Conferences',     to: `/projects/${projectId}/crm/conferences`,  icon: Calendar   },
    ],
    [projectId]
  );

  const hasCrmActive = crmLinks.some((link) => location.pathname === link.to);
  const [open, setOpen] = useState(hasCrmActive);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <button
          className={cn(
            'flex items-center gap-2 w-full pr-2 py-1.5 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
            indent,
            hasCrmActive && 'text-foreground font-medium'
          )}
        >
          <Users className="h-3 w-3 text-primary shrink-0" />
          <span className="flex-1 text-left">CRM</span>
          {open ? <ChevronDown className="h-3 w-3" /> : <ChevronRight className="h-3 w-3" />}
        </button>
      </CollapsibleTrigger>
      <CollapsibleContent>
        <div className="pl-4 space-y-0.5 py-0.5">
          {crmLinks.map((link) => {
            const isActive = location.pathname === link.to;
            const Icon = link.icon;
            return (
              <Link
                key={link.to}
                to={link.to}
                className={cn(
                  'flex items-center gap-2 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors',
                  isActive && 'bg-primary/10 text-foreground font-medium'
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
// ProjectFolder — standalone project card (used in flat fallback list)
// ============================================================================

function ProjectFolder({
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

          {/* CRM */}
          <CrmSidebarLinks projectId={project.id} location={location} indent="pl-2" />

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

function SortableSidebarProjectFolder({
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

                {/* CRM Section */}
                <CrmSidebarLinks projectId={project.id} location={location} indent="pl-2" />

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

// ============================================================================
// SortableProjectList — DnD wrapper for a list of projects within a scope
// ============================================================================

function SortableProjectList({
  scopeKey,
  projects,
  projectId,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  scopeKey: string;
  projects: SidebarProjectType[];
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
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

    // Check if dropped onto a container project
    if (overId.startsWith('container:')) {
      const parentId = overId.replace('container:', '');
      const projectDragId = String(active.id);
      try {
        await projectsApi.setParent(projectDragId, parentId);
        queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
      } catch (err) {
        console.error('Failed to reparent project:', err);
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
            expandedProjects={expandedProjects}
            onToggleProject={onToggleProject}
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
  organizationId,
  projectId,
  expandedProjects,
  onToggleProject,
  queryClient,
}: {
  client: SidebarClientType;
  organizationId: string;
  projectId?: string;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient?: QueryClient;
}) {
  const hasActiveProject = isProjectInTree(client.projects, projectId || '');
  const storageKey = `sidebar:client:${client.id}:expanded`;
  const [expanded, setExpanded] = useState<boolean>(() => {
    if (hasActiveProject) return true;
    const stored = localStorage.getItem(storageKey);
    return stored !== null ? stored === 'true' : false;
  });
  const handleSetExpanded = (next: boolean) => {
    setExpanded(next);
    localStorage.setItem(storageKey, String(next));
  };

  useEffect(() => {
    if (hasActiveProject && !expanded) {
      handleSetExpanded(true);
    }
  }, [hasActiveProject]);

  return (
    <Collapsible open={expanded} onOpenChange={handleSetExpanded}>
      {/* Entire header row is the collapse trigger — chevron always visible on the left */}
      <CollapsibleTrigger asChild>
        <div
          className={cn(
            'flex items-center gap-1.5 px-2 py-1.5 rounded-sm cursor-pointer hover:bg-accent/60 hover:text-accent-foreground transition-colors group/client text-xs',
            hasActiveProject && 'bg-primary/10 text-foreground font-medium'
          )}
        >
          {expanded ? (
            <ChevronDown className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          ) : (
            <ChevronRight className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
          )}
          <HealthDot status={client.health_status} />
          <UserCircle className="h-3.5 w-3.5 text-primary shrink-0" />
          <span className="truncate flex-1 font-normal">{client.name}</span>
          <div className="flex items-center gap-1 shrink-0">
            {client.active_issues_count != null && client.active_issues_count > 0 && (
              <span className="text-[9px] px-1 py-0.5 rounded bg-destructive/10 text-destructive">
                {client.active_issues_count}
              </span>
            )}
            <span className="text-[10px] text-muted-foreground">
              {countProjects(client.projects)}
            </span>
          </div>
        </div>
      </CollapsibleTrigger>
      <CollapsibleContent className="pl-4">
        <div className="space-y-0.5 py-0.5">
          <SortableProjectList
            scopeKey={`client:${client.id}`}
            projects={client.projects}
            projectId={projectId}
            expandedProjects={expandedProjects}
            onToggleProject={onToggleProject}
            queryClient={queryClient}
          />

          {/* Client context quick links */}
          <div className="pt-1 mt-1 border-t border-border/40 space-y-0.5">
            <Link
              to={`/organizations/${organizationId}/clients/${client.id}`}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground',
                location.pathname === `/organizations/${organizationId}/clients/${client.id}` && 'bg-primary/10 text-foreground font-medium'
              )}
            >
              <UserCircle className="h-3 w-3 shrink-0" />
              <span>Client Overview</span>
            </Link>
            {client.crm_person_id && (
              <Link
                to={`/people/${client.crm_person_id}`}
                className={cn(
                  'flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground',
                  location.pathname === `/people/${client.crm_person_id}` && 'bg-primary/10 text-foreground font-medium'
                )}
              >
                <Users className="h-3 w-3 shrink-0" />
                <span>CRM Profile</span>
              </Link>
            )}
            <Link
              to={`/organizations/${organizationId}/crm/pipeline`}
              className={cn(
                'flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground',
              )}
            >
              <Package className="h-3 w-3 shrink-0" />
              <span>Deliverables</span>
            </Link>
            <button
              className="flex items-center gap-1.5 px-2 py-1 text-[10px] rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors text-muted-foreground w-full text-left opacity-0 group-hover/client:opacity-100"
              onClick={async (e) => {
                e.stopPropagation();
                try {
                  const result = await NiceModal.show('project-form', {
                    organization_id: organizationId,
                    client_id: client.id,
                  }) as ProjectFormDialogResult;
                  if (result === 'saved') {
                    queryClient?.invalidateQueries({ queryKey: ['sidebarTree'] });
                  }
                } catch {
                  // dialog dismissed
                }
              }}
            >
              <Plus className="h-3 w-3 shrink-0" />
              <span>Add Project</span>
            </button>
          </div>
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
  activeOrgId,
  isAdmin,
  expandedProjects,
  onToggleProject,
  queryClient,
  isWorkspacePage,
}: {
  org: SidebarOrg;
  projectId?: string;
  activeOrgId?: string;
  isAdmin: boolean;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient: QueryClient;
  isWorkspacePage?: boolean;
}) {
  const navigate = useNavigate();
  const location = useLocation();
  const [internalExpanded, setInternalExpanded] = useState(true);
  const [clientsExpanded, setClientsExpanded] = useState(true);
  const [orgContentExpanded, setOrgContentExpanded] = useState(!isWorkspacePage);

  // Auto-collapse/expand when workspace page state changes
  useEffect(() => {
    setOrgContentExpanded(!isWorkspacePage);
  }, [isWorkspacePage]);

  const findInTree = (projects: SidebarProjectType[], id: string): boolean =>
    projects.some((p) => p.id === id || findInTree(p.children || [], id));
  const hasActiveProject = projectId ? (
    findInTree(org.internal_projects, projectId) ||
    org.clients.some((c) => findInTree(c.projects, projectId))
  ) : false;
  const isActiveOrg = activeOrgId === org.id || hasActiveProject;

  return (
    <div className="space-y-0.5">
      {/* Org header — clickable to toggle content */}
      <Collapsible open={orgContentExpanded} onOpenChange={setOrgContentExpanded}>
        <div className={cn(
          "flex items-center rounded-sm transition-colors",
          isActiveOrg && !isWorkspacePage && "bg-primary/10 dark:bg-primary/15 shadow-[inset_3px_0_0_hsl(var(--brand))]"
        )}>
          <Button
            variant="ghost"
            className={cn(
              "flex-1 justify-start px-2 py-1.5 h-auto font-medium text-xs uppercase tracking-wider text-muted-foreground hover:text-foreground min-w-0",
              isActiveOrg && !isWorkspacePage && "text-foreground font-semibold"
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
              <button className="flex items-center gap-1 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider hover:text-foreground">
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
                      queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
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
              <button className="flex items-center gap-1 text-[10px] font-semibold text-muted-foreground uppercase tracking-wider hover:text-foreground">
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
                    queryClient.invalidateQueries({ queryKey: ['sidebarTree'] });
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
// SidebarOrgGroups — separates orgs into active (has content) vs empty
// ============================================================================

function SidebarOrgGroups({
  sidebarTree,
  projectId,
  orgId,
  isAdmin,
  homeOrgId,
  expandedProjects,
  onToggleProject,
  queryClient,
  isWorkspacePage,
}: {
  sidebarTree: SidebarTree;
  projectId?: string;
  orgId?: string;
  isAdmin: boolean;
  homeOrgId?: string | null;
  expandedProjects: Set<string>;
  onToggleProject: (id: string) => void;
  queryClient: QueryClient;
  isWorkspacePage?: boolean;
}) {
  const [showOtherOrgs, setShowOtherOrgs] = useState(false);

  // Derive activeOrgId: from URL orgId, from which org contains the active project, or from home org
  const allOrgs = [...sidebarTree.owned_orgs, ...sidebarTree.member_orgs];
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
    // Fall back to home org when no URL context
    if (homeOrgId) return homeOrgId;
    return undefined;
  }, [orgId, projectId, homeOrgId, allOrgs]);

  // Only show the active org expanded; all others go into "Other Organizations"
  const activeOrg = activeOrgId ? allOrgs.find((o) => o.id === activeOrgId) : undefined;
  const otherOrgs = allOrgs.filter((o) => o.id !== activeOrgId);

  return (
    <>
      {/* Active organization — fully expanded */}
      {activeOrg && (
        <OrgSection
          key={activeOrg.id || activeOrg.slug}
          org={activeOrg}
          projectId={projectId}
          activeOrgId={activeOrgId}
          isAdmin={isAdmin}
          expandedProjects={expandedProjects}
          onToggleProject={onToggleProject}
          queryClient={queryClient}
          isWorkspacePage={isWorkspacePage}
        />
      )}

      {/* Other organizations — collapsed name-only list */}
      {otherOrgs.length > 0 && (
        <Collapsible open={showOtherOrgs} onOpenChange={setShowOtherOrgs}>
          <CollapsibleTrigger asChild>
            <Button
              variant="ghost"
              className="w-full justify-between px-2 py-1.5 h-auto text-[10px] font-semibold text-muted-foreground uppercase tracking-wider hover:text-foreground"
            >
              <span>Other Organizations ({otherOrgs.length})</span>
              {showOtherOrgs ? (
                <ChevronDown className="h-3 w-3" />
              ) : (
                <ChevronRight className="h-3 w-3" />
              )}
            </Button>
          </CollapsibleTrigger>
          <CollapsibleContent className="pl-2">
            <div className="space-y-0.5 py-0.5">
              {otherOrgs.map((org) => (
                <Link
                  key={org.id || org.slug}
                  to={org.id ? `/organizations/${org.id}` : '#'}
                  className="flex items-center gap-1.5 px-2 py-1 text-xs rounded-sm hover:bg-accent/60 hover:text-accent-foreground transition-colors"
                >
                  <Building2 className="h-3 w-3 text-muted-foreground shrink-0" />
                  <span className="truncate">{org.name}</span>
                </Link>
              ))}
            </div>
          </CollapsibleContent>
        </Collapsible>
      )}
    </>
  );
}

// ============================================================================
// Main Sidebar
// ============================================================================

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

  useEffect(() => {
    if (!projectId) return;
    // Only keep the active project expanded
    setExpandedProjects(new Set([projectId]));
  }, [projectId]);

  const toggleProject = (id: string) => {
    setExpandedProjects(prev => {
      if (prev.has(id)) {
        // Collapsing the currently expanded project
        const newSet = new Set(prev);
        newSet.delete(id);
        return newSet;
      } else {
        // Expand only this project, collapse all others
        return new Set([id]);
      }
    });
  };

  const [globalViewsExpanded, setGlobalViewsExpanded] = useState(false);
  const [adminPlatformsExpanded, setAdminPlatformsExpanded] = useState(false);
  const [managementExpanded, setManagementExpanded] = useState(false);
  const [myWorkspaceExpanded, setMyWorkspaceExpanded] = useState(true);

  // Staging pending count for sidebar badge
  const homeOrgId = user?.home_organization_id || user?.organizations?.[0]?.id;
  const { data: stagingPendingCount = 0 } = useQuery({
    queryKey: ['stagingPendingCount', homeOrgId],
    queryFn: async () => {
      if (!homeOrgId) return 0;
      const records = await stagingApi.listPending(homeOrgId);
      return records.filter((r: any) => r.status === 'pending_review').length;
    },
    enabled: !!homeOrgId,
    staleTime: 30000,
    refetchInterval: 60000,
  });

  // Detect if user is on a "My Workspace" page (user-level, not org-level)
  const WORKSPACE_PATHS = PRIMARY_NAV_ITEMS.map(item => item.to);
  const isWorkspacePage = WORKSPACE_PATHS.some(path => location.pathname === path || location.pathname.startsWith(path + '/'));

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
      sidebarCollapsed ? "w-14" : "w-64",
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
          {filteredAdminNav.length > 0 && (
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
          )}

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

          {/* Management, Global Views - role-gated (collapsed into sections) */}
          {roleInfo.canSeeManagement && (
            <div className="border-b border-border/40">
              {/* Management section */}
              <Collapsible open={managementExpanded} onOpenChange={setManagementExpanded}>
                <CollapsibleTrigger asChild>
                  <div className="sidebar-nav-item mx-3 my-1.5 justify-between cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <LayoutDashboard className="h-4 w-4" />
                      <span>Management</span>
                      {!managementExpanded && (
                        <span className="text-[10px] text-muted-foreground/70 font-medium">{MANAGEMENT_NAV_ITEMS.length}</span>
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

              {/* Global Views section */}
              <Collapsible open={globalViewsExpanded} onOpenChange={setGlobalViewsExpanded}>
                <CollapsibleTrigger asChild>
                  <div className="sidebar-nav-item mx-3 my-1.5 justify-between cursor-pointer">
                    <div className="flex items-center gap-2.5">
                      <BarChart3 className="h-4 w-4" />
                      <span>Global Views</span>
                      {!globalViewsExpanded && (
                        <span className="text-[10px] text-muted-foreground/70 font-medium">{GLOBAL_VIEW_ITEMS.length}</span>
                      )}
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
                isWorkspacePage={isWorkspacePage}
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

          {/* External Links */}
          {EXTERNAL_LINKS.map((item) => {
            const Icon = item.icon;

            if (sidebarCollapsed) {
              if (item.external) {
                return (
                  <Tooltip key={item.href}>
                    <TooltipTrigger asChild>
                      <a href={item.href} target="_blank" rel="noopener noreferrer" className="block">
                        <Button variant="ghost" className="w-full justify-center p-2 h-auto text-muted-foreground hover:text-foreground">
                          <Icon className="h-4 w-4" />
                        </Button>
                      </a>
                    </TooltipTrigger>
                    <TooltipContent side="right">{item.label}</TooltipContent>
                  </Tooltip>
                );
              }
              return (
                <Tooltip key={item.label}>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      className="w-full justify-center p-2 h-auto text-muted-foreground hover:text-foreground"
                      onClick={() => { if (item.action) NiceModal.show(item.action); }}
                    >
                      <Icon className="h-4 w-4" />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="right">{item.label}</TooltipContent>
                </Tooltip>
              );
            }

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
