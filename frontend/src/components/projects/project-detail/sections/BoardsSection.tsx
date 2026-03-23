import { useNavigate } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { SectionHeader } from '@/components/ui/section-header';
import { CardGrid } from '@/components/ui/card-grid';
import { cn } from '@/lib/utils';
import {
  AlertCircle,
  Code2,
  LayoutGrid,
  Loader2,
  Palette,
  Plus,
  Shapes,
  Trash2,
} from 'lucide-react';
import type { ProjectBoard, TaskWithAttemptStatus } from 'shared/types';
import { formatStatusLabel } from '../helpers';
import type { BoardMeta } from '../types';

interface BoardsSectionProps {
  projectId: string;
  boards: ProjectBoard[];
  boardsLoading: boolean;
  boardsError: string;
  tasksLoading: boolean;
  tasksError: string;
  tasksByBoard: Map<string, TaskWithAttemptStatus[]>;
  unassignedTasks: TaskWithAttemptStatus[];
  isDefaultBoard: (boardType: ProjectBoard['board_type']) => boolean;
  onCreateBoard: () => void;
  onDeleteBoard: (board: ProjectBoard) => void;
}

const boardTemplateMeta: Record<ProjectBoard['board_type'], BoardMeta> = {
  default: {
    icon: LayoutGrid,
    accentBorder: 'border-blue-300/70',
    accentBackground: 'bg-gradient-to-br from-blue-200/30 via-blue-100/10 to-transparent',
    iconBg: 'bg-blue-100',
    iconColor: 'text-blue-600',
    tagline: 'Central hub for all project tasks, agent flows, and artifacts.',
    prompts: [
      'Track all tasks in one unified view',
      'Monitor agent execution and approvals',
      'Review artifacts and deliverables',
    ],
  },
  custom: {
    icon: Shapes,
    accentBorder: 'border-slate-300/60',
    accentBackground: 'bg-gradient-to-br from-slate-200/30 via-slate-100/10 to-transparent',
    iconBg: 'bg-slate-100',
    iconColor: 'text-slate-600',
    tagline: 'Create a focused workspace for specific teams or initiatives.',
    prompts: [
      'Name the focus and success criteria',
      'Link supporting assets and rituals',
      'Assign owners and recurring cadences',
    ],
  },
  brand_assets: {
    icon: Palette,
    accentBorder: 'border-pink-300/60',
    accentBackground: 'bg-gradient-to-br from-pink-200/30 via-pink-100/10 to-transparent',
    iconBg: 'bg-pink-100',
    iconColor: 'text-pink-600',
    tagline: 'Manage brand guidelines, logos, and creative assets.',
    prompts: [
      'Upload brand guidelines and logos',
      'Define colour palette and typography',
      'Organise creative asset library',
    ],
  },
  executive_assets: {
    icon: Code2,
    accentBorder: 'border-purple-300/60',
    accentBackground: 'bg-gradient-to-br from-purple-200/30 via-purple-100/10 to-transparent',
    iconBg: 'bg-purple-100',
    iconColor: 'text-purple-600',
    tagline: 'Executive-level deliverables, reports, and strategic assets.',
    prompts: [
      'Track executive reports and presentations',
      'Manage strategic planning documents',
      'Organise board-level communications',
    ],
  },
};

const boardTypeLabels: Record<ProjectBoard['board_type'], string> = {
  default: 'Main Board',
  custom: 'Custom',
  brand_assets: 'Brand Assets',
  executive_assets: 'Executive Assets',
};

function formatBoardLabel(value: ProjectBoard['board_type']) {
  return boardTypeLabels[value] ?? formatStatusLabel(value);
}

export function BoardsSection({
  projectId,
  boards,
  boardsLoading,
  boardsError,
  tasksLoading,
  tasksError,
  tasksByBoard,
  unassignedTasks,
  isDefaultBoard,
  onCreateBoard,
  onDeleteBoard,
}: BoardsSectionProps) {
  const navigate = useNavigate();

  return (
    <Card className="h-full">
      <div className="p-6 pb-0">
        <SectionHeader
          icon={LayoutGrid}
          title="Workstreams & Boards"
          subtitle="Activate and monitor each delivery lane across the brand."
          actions={
            <Button variant="outline" size="sm" onClick={onCreateBoard}>
              <Plus className="mr-1 h-4 w-4" /> Create Board
            </Button>
          }
        />
      </div>
      <CardContent className="space-y-4">
        {boardsLoading ? (
          <div className="flex items-center gap-2 text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            Loading boards...
          </div>
        ) : boardsError ? (
          <Alert variant="destructive">
            <AlertCircle className="h-4 w-4" />
            <AlertDescription>{boardsError}</AlertDescription>
          </Alert>
        ) : boards.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            Boards will appear here after the database migration completes.
          </p>
        ) : (
          <div className="space-y-4">
            {tasksError && (
              <Alert variant="destructive">
                <AlertCircle className="h-4 w-4" />
                <AlertDescription>{tasksError}</AlertDescription>
              </Alert>
            )}
            <CardGrid columns={{ sm: 1, lg: 2 }} gap={4}>
              {boards.map((board) => {
                const meta = boardTemplateMeta[board.board_type] || boardTemplateMeta.custom;
                const Icon = meta.icon;
                const boardTasks = tasksByBoard.get(board.id) ?? [];
                const openBoardWorkspace = () => {
                  const params = new URLSearchParams();
                  params.set('board', board.id);
                  navigate({
                    pathname: `/projects/${projectId}/tasks`,
                    search: params.toString(),
                  });
                };
                return (
                  <div
                    key={board.id}
                    role="button"
                    tabIndex={0}
                    onClick={openBoardWorkspace}
                    onKeyDown={(event) => {
                      if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        openBoardWorkspace();
                      }
                    }}
                    className={cn(
                      'relative overflow-hidden rounded-2xl border bg-background p-5 text-left shadow-sm transition focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 hover:shadow-md',
                      meta.accentBorder
                    )}
                  >
                    <div
                      className={cn(
                        'pointer-events-none absolute inset-0 rounded-2xl',
                        meta.accentBackground
                      )}
                    />
                    <div className="relative flex items-start justify-between gap-4">
                      <div className="flex items-start gap-3">
                        <span
                          className={cn(
                            'flex h-11 w-11 items-center justify-center rounded-full',
                            meta.iconBg
                          )}
                        >
                          <Icon className={cn('h-5 w-5', meta.iconColor)} />
                        </span>
                        <div>
                          <div className="flex flex-wrap items-center gap-2">
                            <p className="text-base font-semibold leading-tight">
                              {board.name}
                            </p>
                            <Badge variant="outline" className="uppercase">
                              {formatBoardLabel(board.board_type)}
                            </Badge>
                          </div>
                          <p className="mt-1 text-sm text-muted-foreground">
                            {board.description || meta.tagline}
                          </p>
                        </div>
                      </div>
                      {!isDefaultBoard(board.board_type) && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="relative text-muted-foreground hover:text-destructive"
                          onClick={(event) => {
                            event.stopPropagation();
                            event.preventDefault();
                            onDeleteBoard(board);
                          }}
                          aria-label={`Delete board ${board.name}`}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </div>
                    <div className="relative mt-4 space-y-2">
                      <div className="flex items-center justify-between text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                        <span>Tasks</span>
                        <span>{boardTasks.length}</span>
                      </div>
                      {tasksLoading ? (
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <Loader2 className="h-3.5 w-3.5 animate-spin" />
                          Loading tasks...
                        </div>
                      ) : boardTasks.length === 0 ? (
                        <p className="text-xs text-muted-foreground">
                          No tasks assigned to this board yet.
                        </p>
                      ) : (
                        <ul className="space-y-1">
                          {boardTasks.slice(0, 5).map((task) => (
                            <li
                              key={task.id}
                              className="flex items-center justify-between gap-2 rounded-lg border bg-muted/40 px-2 py-1"
                            >
                              <span className="truncate text-sm font-medium text-foreground">
                                {task.title}
                              </span>
                              <Badge
                                variant={task.status === 'done' ? 'secondary' : 'outline'}
                                className="uppercase text-[10px]"
                              >
                                {formatStatusLabel(task.status)}
                              </Badge>
                            </li>
                          ))}
                          {boardTasks.length > 5 && (
                            <li className="text-xs text-muted-foreground">
                              +{boardTasks.length - 5} more task
                              {boardTasks.length - 5 === 1 ? '' : 's'}
                            </li>
                          )}
                        </ul>
                      )}
                      <div className="flex flex-wrap items-center justify-between gap-3 pt-3 text-[11px] uppercase tracking-wide text-muted-foreground">
                        <span>
                          Updated {new Date(board.updated_at).toLocaleDateString()}
                        </span>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          className="h-7"
                          onClick={(event) => {
                            event.stopPropagation();
                            event.preventDefault();
                            openBoardWorkspace();
                          }}
                        >
                          Open board
                        </Button>
                      </div>
                    </div>
                  </div>
                );
              })}
            </CardGrid>
            {unassignedTasks.length > 0 && (
              <div
                role="button"
                tabIndex={0}
                onClick={() => {
                  const params = new URLSearchParams();
                  params.set('board', 'unassigned');
                  navigate({
                    pathname: `/projects/${projectId}/tasks`,
                    search: params.toString(),
                  });
                }}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    event.preventDefault();
                    const params = new URLSearchParams();
                    params.set('board', 'unassigned');
                    navigate({
                      pathname: `/projects/${projectId}/tasks`,
                      search: params.toString(),
                    });
                  }
                }}
                className={cn(
                  'rounded-2xl border border-dashed bg-muted/30 p-4 text-left transition hover:border-muted-foreground/50 hover:bg-muted/50 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2'
                )}
              >
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <p className="text-sm font-semibold">Unassigned Tasks</p>
                    <p className="text-xs text-muted-foreground">
                      Tasks without a board. Assign them to keep workstreams organized.
                    </p>
                  </div>
                  <Badge variant="outline" className="uppercase text-[10px]">
                    {unassignedTasks.length}
                  </Badge>
                </div>
                <ul className="mt-3 space-y-1">
                  {unassignedTasks.slice(0, 6).map((task) => (
                    <li
                      key={task.id}
                      className="flex items-center justify-between gap-2 rounded-lg border bg-background px-2 py-1"
                    >
                      <span className="truncate text-sm font-medium text-foreground">
                        {task.title}
                      </span>
                      <Badge
                        variant={task.status === 'done' ? 'secondary' : 'outline'}
                        className="uppercase text-[10px]"
                      >
                        {formatStatusLabel(task.status)}
                      </Badge>
                    </li>
                  ))}
                  {unassignedTasks.length > 6 && (
                    <li className="text-xs text-muted-foreground">
                      +{unassignedTasks.length - 6} more task
                      {unassignedTasks.length - 6 === 1 ? '' : 's'}
                    </li>
                  )}
                </ul>
              </div>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
