/**
 * Hook that watches task state changes via WebSocket and shows toast notifications
 * for agent-driven transitions (status changes, watcher triggers, QA verdicts).
 *
 * This is a pure frontend approach — diffs old vs new task state on each render
 * and fires toasts when meaningful agent-driven changes are detected.
 */
import { useEffect, useRef } from 'react';
import type { TaskCollaborator } from 'shared/types';
import { toast } from 'sonner';

import type { TaskWithArchive } from '@/lib/api';

/** Status labels for display */
const STATUS_LABELS: Record<string, string> = {
  todo: 'To Do',
  inprogress: 'In Progress',
  inreview: 'In Review',
  done: 'Done',
  cancelled: 'Cancelled',
};

/** Watcher action labels */
const WATCHER_VERDICT_LABELS: Record<string, { label: string; icon: string }> =
  {
    qa_pass: { label: 'PASS', icon: '✅' },
    qa_needs_changes: { label: 'NEEDS CHANGES', icon: '⚠️' },
    qa_fail: { label: 'FAIL', icon: '❌' },
  };

/**
 * Detect and notify on agent-driven task changes.
 *
 * @param tasksById - Current tasks state from WebSocket stream
 */
export function useTaskChangeNotifications(
  tasksById: Record<string, TaskWithArchive>
) {
  const prevTasksRef = useRef<Record<string, TaskWithArchive>>({});

  useEffect(() => {
    const prevTasks = prevTasksRef.current;

    for (const [taskId, task] of Object.entries(tasksById)) {
      const prev = prevTasks[taskId];
      if (!prev) continue; // New task — skip (initial load or just created)

      // 1. Status change detection
      if (prev.status !== task.status) {
        const isAgentDriven = !!task.agent_id;
        const statusLabel = STATUS_LABELS[task.status] ?? task.status;
        const titleSnippet =
          task.title.length > 40 ? task.title.slice(0, 40) + '…' : task.title;

        if (isAgentDriven) {
          if (task.status === 'inprogress') {
            toast.info(`Agent started working on "${titleSnippet}"`, {
              description: `Status → ${statusLabel}`,
            });
          } else if (task.status === 'inreview') {
            toast.info(`"${titleSnippet}" moved to ${statusLabel}`, {
              description: task.has_merged_attempt
                ? 'PR created — ready for review'
                : `Status → ${statusLabel}`,
            });
          } else if (task.status === 'done') {
            toast.success(`"${titleSnippet}" completed`, {
              description: `Status → ${statusLabel}`,
            });
          }
        }
      }

      // 2. Watcher state change detection
      const prevCollabs = buildCollabMap(prev.parsed_collaborators);
      const currCollabs = buildCollabMap(task.parsed_collaborators);

      for (const [actorId, curr] of currCollabs.entries()) {
        if (curr.actor_type !== 'agent_watcher') continue;

        const prevAction = prevCollabs.get(actorId)?.last_action;
        if (prevAction === curr.last_action) continue; // No change

        const titleSnippet =
          task.title.length > 40 ? task.title.slice(0, 40) + '…' : task.title;

        // Watcher just triggered
        if (curr.last_action === 'triggered' && prevAction === 'watching') {
          toast.info(`QA review started on "${titleSnippet}"`, {
            description: `Watcher ${curr.actor_id} triggered`,
          });
        }

        // Watcher verdict received
        const verdict = WATCHER_VERDICT_LABELS[curr.last_action];
        if (verdict && prevAction !== curr.last_action) {
          const toastFn =
            curr.last_action === 'qa_pass'
              ? toast.success
              : curr.last_action === 'qa_fail'
                ? toast.error
                : toast.warning;

          toastFn(`${verdict.icon} QA verdict: ${verdict.label}`, {
            description: `"${titleSnippet}"`,
          });
        }
      }
    }

    // Update ref with current state for next comparison
    prevTasksRef.current = { ...tasksById };
  }, [tasksById]);
}

/** Build a Map of actor_id → TaskCollaborator for efficient lookup */
function buildCollabMap(
  collaborators: Array<TaskCollaborator> | null | undefined
): Map<string, TaskCollaborator> {
  const map = new Map<string, TaskCollaborator>();
  if (!collaborators) return map;
  for (const c of collaborators) {
    map.set(c.actor_id, c);
  }
  return map;
}
