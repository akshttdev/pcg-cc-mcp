import { useCallback } from 'react';
import { useActivityStore } from '@/stores/useActivityStore';
import type { ActivityType } from '@/types/activity';

export function useActivityLogger(taskId?: string) {
  const { logActivity } = useActivityStore();

  const log = useCallback(
    (type: ActivityType, description: string, metadata?: Record<string, unknown>) => {
      if (taskId) {
        logActivity(taskId, type, description, metadata);
      }
    },
    [taskId, logActivity]
  );

  return {
    logTaskCreated: (title: string) =>
      log('task_created', `Task created: ${title}`),

    logTaskUpdated: (changes: string[]) =>
      log('task_updated', `Task updated`, { changes: changes.join(', ') }),

    logTaskDeleted: () =>
      log('task_deleted', 'Task deleted'),

    logStatusChanged: (oldStatus: string, newStatus: string) =>
      log('status_changed', `Status changed from ${oldStatus} to ${newStatus}`, {
        from: oldStatus,
        to: newStatus,
      }),

    logPriorityChanged: (oldPriority: string, newPriority: string) =>
      log('priority_changed', `Priority changed from ${oldPriority} to ${newPriority}`, {
        from: oldPriority,
        to: newPriority,
      }),

    logAssigneeChanged: (oldAssignee: string, newAssignee: string) =>
      log('assignee_changed', `Assignee changed from ${oldAssignee} to ${newAssignee}`, {
        from: oldAssignee,
        to: newAssignee,
      }),

    logCommentAdded: (commentPreview: string) =>
      log('comment_added', `Comment added: ${commentPreview.substring(0, 50)}...`),

    logDependencyAdded: (targetTask: string, type: string) =>
      log('dependency_added', `Dependency added: ${type} ${targetTask}`, {
        targetTask,
        dependencyType: type,
      }),

    logDependencyRemoved: (targetTask: string) =>
      log('dependency_removed', `Dependency removed: ${targetTask}`, {
        targetTask,
      }),

    logTimeLogged: (duration: string) =>
      log('time_logged', `Time logged: ${duration}`, { duration }),

    logFileAttached: (fileName: string) =>
      log('file_attached', `File attached: ${fileName}`, { fileName }),

    logCustomActivity: (type: ActivityType, description: string, metadata?: Record<string, unknown>) =>
      log(type, description, metadata),
  };
}
