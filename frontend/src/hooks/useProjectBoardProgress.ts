import { useQuery } from '@tanstack/react-query';
import { projectsApi, tasksApi } from '@/lib/api';
import type { ProjectBoard, TaskWithAttemptStatus } from 'shared/types';

interface BoardProgress {
  boardId: string;
  boardName: string;
  totalAssets: number;
  completedAssets: number;
  percentage: number;
}

// Map delivery stages to board name patterns
const STAGE_BOARD_MAP: Record<string, string[]> = {
  'Brand Guide': ['brand', 'branding'],
  'Online Presence': ['web', 'dev', 'online', 'presence'],
  'Social Stack': ['social'],
};

export function useProjectBoardProgress(projectId: string | undefined) {
  const { data: boards = [] } = useQuery<ProjectBoard[]>({
    queryKey: ['projectBoards', projectId],
    queryFn: () => projectsApi.listBoards(projectId!),
    enabled: !!projectId,
    staleTime: 5 * 60 * 1000,
  });

  const { data: tasks = [] } = useQuery<TaskWithAttemptStatus[]>({
    queryKey: ['projectTasks', projectId],
    queryFn: () => tasksApi.getAll(projectId!),
    enabled: !!projectId,
    staleTime: 60 * 1000,
  });

  // Calculate progress for each board
  const boardProgress: BoardProgress[] = boards.map((board) => {
    const boardTasks = tasks.filter((t) => t.board_id === board.id);
    const completedTasks = boardTasks.filter(
      (t) => t.status === 'done'
    );
    const total = boardTasks.length;
    const completed = completedTasks.length;
    return {
      boardId: board.id,
      boardName: board.name,
      totalAssets: total,
      completedAssets: completed,
      percentage: total > 0 ? Math.round((completed / total) * 100) : 0,
    };
  });

  // Find progress for a delivery stage
  const getProgressForStage = (stageName: string): BoardProgress | undefined => {
    const keywords = STAGE_BOARD_MAP[stageName];
    if (!keywords) return undefined;
    return boardProgress.find((bp) =>
      keywords.some((kw) => bp.boardName.toLowerCase().includes(kw))
    );
  };

  return {
    boardProgress,
    getProgressForStage,
  };
}
