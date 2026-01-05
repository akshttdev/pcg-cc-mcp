import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Project, ProjectBoard, TaskWithAttemptStatus } from 'shared/types';
import { showProjectForm } from '@/lib/modals';
import { projectsApi, tasksApi } from '@/lib/api';
import { AlertCircle, Loader2, Plus } from 'lucide-react';
import ProjectCard, {
  ProjectBoardSummary,
} from '@/components/projects/ProjectCard.tsx';
import { useKeyCreate, Scope } from '@/keyboard';
import { useAuth } from '@/contexts/AuthContext';

const createEmptySummary = (
  overrides: Partial<ProjectBoardSummary> = {}
): ProjectBoardSummary => ({
  totalBoards: 0,
  corePresence: {
    executive_assets: false,
    brand_assets: false,
    dev_assets: false,
    social_assets: false,
    agent_flows: false,
    artifact_gallery: false,
    approval_queue: false,
    research_hub: false,
    custom: false,
  },
  customCount: 0,
  totalTasks: 0,
  tasksByType: {
    executive_assets: 0,
    brand_assets: 0,
    dev_assets: 0,
    social_assets: 0,
    agent_flows: 0,
    artifact_gallery: 0,
    approval_queue: 0,
    research_hub: 0,
    custom: 0,
  },
  unassignedTasks: 0,
  latestActivity: undefined,
  ...overrides,
});

const summarizeBoards = (
  boards: ProjectBoard[],
  tasks: TaskWithAttemptStatus[]
): ProjectBoardSummary => {
  const summary = createEmptySummary();

  summary.totalBoards = boards.length;

  const boardMap = new Map<string, ProjectBoard>();

  boards.forEach((board) => {
    boardMap.set(board.id, board);
    summary.corePresence[board.board_type] = true;
    if (board.board_type === 'custom') {
      summary.customCount += 1;
    }
  });

  summary.corePresence.custom = summary.customCount > 0;

  let latest: string | undefined;

  tasks.forEach((task) => {
    const board = task.board_id ? boardMap.get(task.board_id) : undefined;
    if (board) {
      summary.tasksByType[board.board_type] += 1;
    } else {
      summary.unassignedTasks += 1;
    }

    if (
      task.updated_at &&
      (!latest ||
        new Date(task.updated_at).getTime() > new Date(latest).getTime())
    ) {
      latest = task.updated_at;
    }
  });

  summary.totalTasks = tasks.length;
  summary.latestActivity = latest;

  return summary;
};

export function ProjectList() {
  const { t } = useTranslation('projects');
  const { user } = useAuth();
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [focusedProjectId, setFocusedProjectId] = useState<string | null>(null);
  const [boardSummaries, setBoardSummaries] = useState<
    Record<string, ProjectBoardSummary>
  >({});
  const [boardSummariesLoading, setBoardSummariesLoading] = useState(false);

  // Only admins can create projects
  const isAdmin = user?.is_admin ?? false;

  const loadBoardSummaries = useCallback(
    async (projectList: Project[]) => {
      setBoardSummariesLoading(true);

      if (!projectList.length) {
        setBoardSummaries({});
        setBoardSummariesLoading(false);
        return;
      }

      const entries = await Promise.all(
        projectList.map(async (project) => {
          try {
            const [boards, tasks] = await Promise.all([
              projectsApi.listBoards(project.id),
              tasksApi.getAll(project.id),
            ]);
            return [project.id, summarizeBoards(boards, tasks)] as const;
          } catch (err) {
            console.error(
              `Failed to load board data for project ${project.id}:`,
              err
            );
            const message =
              err instanceof Error
                ? err.message
                : 'Failed to load board data';
            return [
              project.id,
              createEmptySummary({ error: message }),
            ] as const;
          }
        })
      );

      setBoardSummaries(Object.fromEntries(entries));
      setBoardSummariesLoading(false);
    },
    []
  );

  const fetchProjects = useCallback(async () => {
    setLoading(true);
    setError('');
    setBoardSummaries({});
    setBoardSummariesLoading(true);

    try {
      const result = await projectsApi.getAll();
      setProjects(result);
      loadBoardSummaries(result);
    } catch (error) {
      console.error('Failed to fetch projects:', error);
      if (error instanceof Error && error.message) {
        setError(error.message);
      } else {
        setError(t('errors.fetchFailed'));
      }
    } finally {
      setLoading(false);
    }
  }, [loadBoardSummaries, t]);

  const handleCreateProject = useCallback(async () => {
    // Only admins can create projects
    if (!isAdmin) {
      return;
    }
    try {
      const result = await showProjectForm();
      if (result === 'saved') {
        fetchProjects();
      }
    } catch (error) {
      // User cancelled - do nothing
    }
  }, [fetchProjects, isAdmin]);

  // Semantic keyboard shortcut for creating new project (admin only)
  useKeyCreate(isAdmin ? handleCreateProject : () => {}, { scope: Scope.PROJECTS });

  const handleEditProject = useCallback(
    async (project: Project) => {
      try {
        const result = await showProjectForm({ project });
        if (result === 'saved') {
          fetchProjects();
        }
      } catch (error) {
        // User cancelled - do nothing
      }
    },
    [fetchProjects]
  );

  // Set initial focus when projects are loaded
  useEffect(() => {
    if (projects.length > 0 && !focusedProjectId) {
      setFocusedProjectId(projects[0].id);
    }
  }, [projects, focusedProjectId]);

  useEffect(() => {
    fetchProjects();
  }, [fetchProjects]);

  return (
    <div className="space-y-6 p-8 pb-16 md:pb-8 h-full overflow-auto">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t('title')}</h1>
          <p className="text-muted-foreground">{t('subtitle')}</p>
        </div>
        {isAdmin && (
          <Button onClick={handleCreateProject}>
            <Plus className="mr-2 h-4 w-4" />
            {t('createProject')}
          </Button>
        )}
      </div>

      {error && (
        <Alert variant="destructive">
          <AlertCircle className="h-4 w-4" />
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {loading ? (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          {t('loading')}
        </div>
      ) : projects.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <div className="mx-auto flex h-12 w-12 items-center justify-center rounded-lg bg-muted">
              <Plus className="h-6 w-6" />
            </div>
            <h3 className="mt-4 text-lg font-semibold">{t('empty.title')}</h3>
            <p className="mt-2 text-sm text-muted-foreground">
              {t('empty.description')}
            </p>
            {isAdmin && (
              <Button className="mt-4" onClick={handleCreateProject}>
                <Plus className="mr-2 h-4 w-4" />
                {t('empty.createFirst')}
              </Button>
            )}
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
          {projects.map((project) => (
            <ProjectCard
              key={project.id}
              project={project}
              isFocused={focusedProjectId === project.id}
              setError={setError}
              onEdit={handleEditProject}
              fetchProjects={fetchProjects}
              boardSummary={
                boardSummariesLoading ? undefined : boardSummaries[project.id]
              }
            />
          ))}
        </div>
      )}
    </div>
  );
}
