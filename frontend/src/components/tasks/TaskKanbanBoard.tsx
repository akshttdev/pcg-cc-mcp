import { memo } from 'react';
import {
  type DragEndEvent,
  KanbanBoard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
} from '@/components/ui/shadcn-io/kanban';
import { TaskCard } from './TaskCard';
import type { TaskStatus, TaskWithAttemptStatus } from 'shared/types';
import type { AgentFlow } from '@/lib/api';

import { statusBoardColors, statusLabels } from '@/utils/status-labels';

type Task = TaskWithAttemptStatus;

interface TaskKanbanBoardProps {
  groupedTasks: Record<TaskStatus, Task[]>;
  onDragEnd: (event: DragEndEvent) => void;
  onEditTask: (task: Task) => void;
  onDeleteTask: (taskId: string) => void;
  onDuplicateTask?: (task: Task) => void;
  onViewTaskDetails: (task: Task) => void;
  selectedTask?: Task;
  selectionMode?: boolean;
  isSelected?: (taskId: string) => boolean;
  onToggleSelection?: (taskId: string) => void;
  agentFlowMap?: Map<string, AgentFlow>;
}

function TaskKanbanBoard({
  groupedTasks,
  onDragEnd,
  onEditTask,
  onDeleteTask,
  onDuplicateTask,
  onViewTaskDetails,
  selectedTask,
  selectionMode,
  isSelected,
  onToggleSelection,
  agentFlowMap,
}: TaskKanbanBoardProps) {
  return (
    <KanbanProvider onDragEnd={onDragEnd}>
      {Object.entries(groupedTasks).map(([status, statusTasks]) => (
        <KanbanBoard key={status} id={status as TaskStatus}>
          <KanbanHeader
            name={statusLabels[status as TaskStatus]}
            color={statusBoardColors[status as TaskStatus]}
          />
          <KanbanCards>
            {statusTasks.map((task, index) => (
              <TaskCard
                key={task.id}
                task={task}
                index={index}
                status={status}
                onEdit={onEditTask}
                onDelete={onDeleteTask}
                onDuplicate={onDuplicateTask}
                onViewDetails={onViewTaskDetails}
                isOpen={selectedTask?.id === task.id}
                selectionMode={selectionMode}
                isSelected={isSelected?.(task.id)}
                onToggleSelection={onToggleSelection}
                agentFlow={agentFlowMap?.get(task.id)}
              />
            ))}
          </KanbanCards>
        </KanbanBoard>
      ))}
    </KanbanProvider>
  );
}

export default memo(TaskKanbanBoard);
