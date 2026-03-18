/**
 * ClientProjectPanel — embeds a project's boards + brand hero inline inside the Client Overview page.
 * Lightweight alternative to ProjectDetail; skips integrations/assets/dialogs.
 */
import { Link, useNavigate } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { projectsApi, tasksApi } from '@/lib/api';
import { projectKeys, projectBoardKeys } from '@/lib/query-keys';
import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import {
  ClipboardCheck, LayoutGrid, Layers, ExternalLink, Palette,
  CheckSquare, Package, Zap, FileText,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import type { ProjectBoard } from 'shared/types';
import type { TaskWithArchive } from '@/lib/api';

// ── Brand profile helpers (duplicated from project-detail to avoid circular dep) ─

const BRAND_PALETTES = [
  { primary: '#6366f1', secondary: '#a855f7' },
  { primary: '#0ea5e9', secondary: '#22d3ee' },
  { primary: '#10b981', secondary: '#34d399' },
  { primary: '#f59e0b', secondary: '#fbbf24' },
  { primary: '#ef4444', secondary: '#f97316' },
  { primary: '#8b5cf6', secondary: '#ec4899' },
];

function hashProjectId(id: string): number {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h * 31 + id.charCodeAt(i)) | 0;
  return Math.abs(h);
}

function getBrandInitials(name: string): string {
  return name.split(/\s+/).map((w) => w[0]?.toUpperCase() ?? '').slice(0, 2).join('');
}

function hexToRgba(hex: string, alpha: number): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

// ── Board card ────────────────────────────────────────────────────────────────

function BoardCard({ board, projectId, taskCount }: { board: ProjectBoard; projectId: string; taskCount: number }) {
  return (
    <Link
      to={`/projects/${projectId}/tasks?board=${board.id}`}
      className="flex items-center gap-3 p-3 rounded-lg border border-border/60 hover:bg-accent/60 hover:border-border transition-all group"
    >
      <LayoutGrid className="h-4 w-4 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <p className="text-sm font-medium truncate group-hover:text-primary transition-colors">{board.name}</p>
        {board.description && (
          <p className="text-xs text-muted-foreground truncate">{board.description}</p>
        )}
      </div>
      {taskCount > 0 && (
        <Badge variant="secondary" className="text-xs shrink-0">{taskCount}</Badge>
      )}
      <ExternalLink className="h-3 w-3 text-muted-foreground opacity-0 group-hover:opacity-100 shrink-0 transition-opacity" />
    </Link>
  );
}

// ── Main panel ────────────────────────────────────────────────────────────────

interface ClientProjectPanelProps {
  projectId: string;
  clientPageUrl?: string; // back-nav target
}

export function ClientProjectPanel({ projectId }: ClientProjectPanelProps) {
  const navigate = useNavigate();

  const { data: project, isLoading: projectLoading } = useQuery({
    queryKey: projectKeys.detail(projectId),
    queryFn: () => projectsApi.getById(projectId),
    staleTime: 2 * 60 * 1000,
  });

  const { data: boards = [], isLoading: boardsLoading } = useQuery({
    queryKey: projectBoardKeys.boards(projectId),
    queryFn: () => projectsApi.listBoards(projectId),
    staleTime: 2 * 60 * 1000,
    enabled: !!project,
  });

  const { data: allTasks = [] } = useQuery({
    queryKey: projectBoardKeys.tasks(projectId),
    queryFn: () => tasksApi.getAll(projectId),
    staleTime: 60 * 1000,
    enabled: !!project,
  });

  if (projectLoading) {
    return (
      <div className="flex items-center gap-2 py-8 text-muted-foreground text-sm">
        <Loader size={16} />Loading project…
      </div>
    );
  }

  if (!project) {
    return (
      <p className="text-sm text-muted-foreground py-4">Project not found.</p>
    );
  }

  // Brand palette from project id hash
  const palette = BRAND_PALETTES[hashProjectId(project.id) % BRAND_PALETTES.length];
  const primaryColor = palette.primary;
  const secondaryColor = palette.secondary;
  const brandInitials = getBrandInitials(project.name);
  const heroGradient: React.CSSProperties = {
    background: `linear-gradient(135deg, ${hexToRgba(primaryColor, 0.12)} 0%, ${hexToRgba(secondaryColor, 0.08)} 100%)`,
    borderColor: hexToRgba(primaryColor, 0.3),
  };

  // Task counts
  const tasksByBoard = new Map<string, number>();
  for (const task of allTasks as TaskWithArchive[]) {
    if (task.board_id) tasksByBoard.set(task.board_id, (tasksByBoard.get(task.board_id) ?? 0) + 1);
  }
  const totalTasks = (allTasks as TaskWithArchive[]).length;
  const activeTasks = (allTasks as TaskWithArchive[]).filter((t) => t.status !== 'done' && t.status !== 'cancelled' && !t.archived_at).length;
  const doneTasks  = (allTasks as TaskWithArchive[]).filter((t) => t.status === 'done').length;
  const pct = totalTasks > 0 ? Math.round((doneTasks / totalTasks) * 100) : 0;

  return (
    <div className="space-y-4">
      {/* Hero card */}
      <Card className="relative overflow-hidden border" style={heroGradient}>
        <CardContent className="p-6">
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            {/* Brand identity */}
            <div className="flex items-center gap-4">
              <div
                className="flex h-14 w-14 items-center justify-center rounded-2xl border-2 border-white/60 bg-white/80 text-xl font-bold text-foreground shadow-sm shrink-0"
                style={{ borderColor: hexToRgba(primaryColor, 0.4) }}
              >
                {brandInitials}
              </div>
              <div>
                <p className="text-xs uppercase tracking-widest text-muted-foreground mb-0.5">
                  {'Active'}
                </p>
                <h3 className="text-xl font-bold leading-tight">{project.name}</h3>
                {/* Project type has no description field */}
                {/* Brand palette chips */}
                <div className="flex items-center gap-2 mt-2">
                  <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Palette className="h-3 w-3" />
                    <span
                      className="w-4 h-4 rounded-full border border-white/30 shadow-sm inline-block"
                      style={{ background: primaryColor }}
                    />
                    <span
                      className="w-4 h-4 rounded-full border border-white/30 shadow-sm inline-block"
                      style={{ background: secondaryColor }}
                    />
                  </div>
                </div>
              </div>
            </div>

            {/* Stat chips */}
            <div className="flex gap-3 shrink-0 flex-wrap">
              <div className="text-center bg-white/70 dark:bg-black/20 rounded-xl px-4 py-2 shadow-sm">
                <p className="text-2xl font-bold">{boards.length}</p>
                <p className="text-[10px] uppercase text-muted-foreground">Boards</p>
              </div>
              <div className="text-center bg-white/70 dark:bg-black/20 rounded-xl px-4 py-2 shadow-sm">
                <p className="text-2xl font-bold">{activeTasks}</p>
                <p className="text-[10px] uppercase text-muted-foreground">Active</p>
              </div>
              <div className="text-center bg-white/70 dark:bg-black/20 rounded-xl px-4 py-2 shadow-sm">
                <p className="text-2xl font-bold">{totalTasks}</p>
                <p className="text-[10px] uppercase text-muted-foreground">Tasks</p>
              </div>
            </div>
          </div>

          {/* Progress bar */}
          {totalTasks > 0 && (
            <div className="mt-4">
              <div className="flex items-center justify-between text-xs text-muted-foreground mb-1">
                <span className="flex items-center gap-1"><CheckSquare className="h-3 w-3" />{doneTasks}/{totalTasks} tasks done</span>
                <span className={cn('font-medium', pct === 100 ? 'text-green-600' : '')}>{pct}%</span>
              </div>
              <div className="h-1.5 bg-black/10 rounded-full overflow-hidden">
                <div
                  className={cn('h-full rounded-full transition-all', pct === 100 ? 'bg-green-500' : 'bg-primary')}
                  style={{ width: `${pct}%` }}
                />
              </div>
            </div>
          )}

          {/* Action buttons */}
          <div className="mt-4 flex gap-2 flex-wrap">
            <Button
              size="sm"
              onClick={() => navigate(`/projects/${projectId}/tasks`)}
            >
              <ClipboardCheck className="h-3.5 w-3.5 mr-1.5" />
              Tasks
              {totalTasks > 0 && (
                <Badge variant="secondary" className="ml-1.5 h-4 text-[10px] bg-primary-foreground/20 text-primary-foreground">
                  {totalTasks}
                </Badge>
              )}
            </Button>
            <Button variant="outline" size="sm" onClick={() => navigate(`/projects/${projectId}/deliverables`)}>
              <FileText className="h-3.5 w-3.5 mr-1.5" />
              Deliverables
            </Button>
            <Button variant="outline" size="sm" onClick={() => navigate(`/projects/${projectId}/pulse`)}>
              <Zap className="h-3.5 w-3.5 mr-1.5" />
              Pulse
            </Button>
            <Button variant="ghost" size="sm" asChild>
              <Link to={`/projects/${projectId}`}>
                <Package className="h-3.5 w-3.5 mr-1.5" />
                Full View
                <ExternalLink className="h-3 w-3 ml-1" />
              </Link>
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Boards */}
      <div>
        <div className="flex items-center gap-2 mb-2">
          <Layers className="h-4 w-4 text-muted-foreground" />
          <h4 className="text-sm font-semibold">Boards</h4>
          {boardsLoading && <Loader size={12} />}
        </div>
        {boards.length === 0 && !boardsLoading ? (
          <p className="text-sm text-muted-foreground pl-6">No boards yet.</p>
        ) : (
          <div className="grid gap-2 sm:grid-cols-2">
            {(boards as ProjectBoard[]).map((board) => (
              <BoardCard
                key={board.id}
                board={board}
                projectId={projectId}
                taskCount={tasksByBoard.get(board.id) ?? 0}
              />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
