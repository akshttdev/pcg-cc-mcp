import { useMutation, useQueryClient } from '@tanstack/react-query';
import { tasksApi, activityApi } from '@/lib/api';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import { taskKeys, sidebarKeys } from '@/lib/query-keys';
import type {
  CreateTask,
  CreateAndStartTaskRequest,
  Task,
  TaskWithAttemptStatus,
  UpdateTask,
} from 'shared/types';

export function useTaskMutations(projectId?: string) {
  const queryClient = useQueryClient();
  const { navigateToTask } = useTaskViewManager();

  const invalidateQueries = (taskId?: string) => {
    if (projectId) {
      queryClient.invalidateQueries({ queryKey: taskKeys.list(projectId) });
    }
    queryClient.invalidateQueries({ queryKey: sidebarKeys.tree() });
    if (taskId) {
      queryClient.invalidateQueries({ queryKey: taskKeys.detail(taskId) });
    }
  };

  const createTask = useMutation({
    mutationFn: (data: CreateTask) => tasksApi.create(data),
    onSuccess: async (createdTask: Task, variables) => {
      invalidateQueries();
      if (projectId) {
        navigateToTask(projectId, createdTask.id);
      }
      try {
        await activityApi.create({
          task_id: createdTask.id,
          actor_id: variables?.created_by || 'current-user',
          actor_type: 'human',
          action: 'created',
          previous_state: null,
          new_state: {
            title: createdTask.title,
            priority: createdTask.priority,
            status: createdTask.status,
          },
          metadata: {
            created_via: 'ui',
          },
        });
      } catch (error) {
        console.error('Failed to log task creation activity', error);
      }
    },
    onError: (err) => {
      console.error('Failed to create task:', err);
    },
  });

  const createAndStart = useMutation({
    mutationFn: (data: CreateAndStartTaskRequest) =>
      tasksApi.createAndStart(data),
    onSuccess: async (createdTask: TaskWithAttemptStatus, variables) => {
      invalidateQueries();
      if (projectId) {
        navigateToTask(projectId, createdTask.id);
      }
      try {
        await activityApi.create({
          task_id: createdTask.id,
          actor_id: variables?.task.created_by || 'current-user',
          actor_type: 'human',
          action: 'created',
          previous_state: null,
          new_state: {
            title: createdTask.title,
            priority: createdTask.priority,
            status: createdTask.status,
          },
          metadata: {
            created_via: 'create_and_start',
          },
        });
      } catch (error) {
        console.error('Failed to log create-and-start activity', error);
      }
    },
    onError: (err) => {
      console.error('Failed to create and start task:', err);
    },
  });

  const updateTask = useMutation({
    mutationFn: ({
      taskId,
      data,
    }: {
      taskId: string;
      data: Partial<UpdateTask>;
    }) => tasksApi.update(taskId, data),
    onSuccess: async (updatedTask: Task, variables) => {
      invalidateQueries(updatedTask.id);
      const updatedFields = Object.keys(variables?.data || {}).filter(
        (key) => (variables?.data as Record<string, unknown>)[key] !== undefined
      );
      if (updatedFields.length === 0) {
        return;
      }
      try {
        await activityApi.create({
          task_id: updatedTask.id,
          actor_id: 'current-user',
          actor_type: 'human',
          action: 'updated',
          previous_state: null,
          new_state: null,
          metadata: {
            fields: updatedFields,
          },
        });
      } catch (error) {
        console.error('Failed to log task update activity', error);
      }
    },
    onError: (err) => {
      console.error('Failed to update task:', err);
    },
  });

  return {
    createTask,
    createAndStart,
    updateTask,
  };
}
