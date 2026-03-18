import { useCallback } from 'react';
import type { TaskWithArchive } from '@/lib/api';

type Task = TaskWithArchive;

export const TASK_STATUSES = [
  'todo',
  'inprogress',
  'inreview',
  'done',
  'cancelled',
] as const;

interface UseTaskKeyboardNavOptions {
  selectedTask: Task | null;
  groupedFilteredTasks: Record<string, Task[]>;
  onViewTaskDetails: (task: Task, attemptId?: string, fullscreen?: boolean) => void;
}

export function useTaskKeyboardNav({
  selectedTask,
  groupedFilteredTasks,
  onViewTaskDetails,
}: UseTaskKeyboardNavOptions) {
  const selectNextTask = useCallback(() => {
    if (selectedTask) {
      const tasksInStatus = groupedFilteredTasks[selectedTask.status] || [];
      const currentIndex = tasksInStatus.findIndex(
        (task) => task.id === selectedTask.id
      );
      if (currentIndex >= 0 && currentIndex < tasksInStatus.length - 1) {
        onViewTaskDetails(tasksInStatus[currentIndex + 1]);
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          onViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, onViewTaskDetails]);

  const selectPreviousTask = useCallback(() => {
    if (selectedTask) {
      const tasksInStatus = groupedFilteredTasks[selectedTask.status] || [];
      const currentIndex = tasksInStatus.findIndex(
        (task) => task.id === selectedTask.id
      );
      if (currentIndex > 0) {
        onViewTaskDetails(tasksInStatus[currentIndex - 1]);
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          onViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, onViewTaskDetails]);

  const selectNextColumn = useCallback(() => {
    if (selectedTask) {
      const currentIndex = TASK_STATUSES.findIndex(
        (status) => status === selectedTask.status
      );
      for (let i = currentIndex + 1; i < TASK_STATUSES.length; i++) {
        const tasks = groupedFilteredTasks[TASK_STATUSES[i]];
        if (tasks && tasks.length > 0) {
          onViewTaskDetails(tasks[0]);
          return;
        }
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          onViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, onViewTaskDetails]);

  const selectPreviousColumn = useCallback(() => {
    if (selectedTask) {
      const currentIndex = TASK_STATUSES.findIndex(
        (status) => status === selectedTask.status
      );
      for (let i = currentIndex - 1; i >= 0; i--) {
        const tasks = groupedFilteredTasks[TASK_STATUSES[i]];
        if (tasks && tasks.length > 0) {
          onViewTaskDetails(tasks[0]);
          return;
        }
      }
    } else {
      for (const status of TASK_STATUSES) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          onViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, onViewTaskDetails]);

  return {
    selectNextTask,
    selectPreviousTask,
    selectNextColumn,
    selectPreviousColumn,
  };
}
