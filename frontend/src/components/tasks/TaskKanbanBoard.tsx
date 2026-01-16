import { memo, useMemo } from 'react';
import {
  type DragEndEvent,
  KanbanBoard,
  KanbanCards,
  KanbanHeader,
  KanbanProvider,
} from '@/components/ui/shadcn-io/kanban';
import { TaskCard } from './TaskCard';
import { EnhancedTaskCard, type TaskCardMode } from './EnhancedTaskCard';
import type { TaskStatus, TaskWithAttemptStatus } from 'shared/types';
import type { AgentFlow } from '@/lib/api';
import { useTasksCardData } from '@/hooks/useTaskCardData';

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
  // New props for enhanced cards
  useEnhancedCards?: boolean;
  defaultCardMode?: TaskCardMode;
  onSendMessageToAgent?: (taskId: string, message: string, agentName?: string) => Promise<string>;
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
  useEnhancedCards = false,
  defaultCardMode,
  onSendMessageToAgent,
}: TaskKanbanBoardProps) {
  // Collect all task IDs for batch fetching enriched data
  const allTaskIds = useMemo(() => {
    return Object.values(groupedTasks).flat().map((task) => task.id);
  }, [groupedTasks]);

  // Fetch enriched card data (artifacts, workflow events) when using enhanced cards
  const { cardDataMap } = useTasksCardData(useEnhancedCards ? allTaskIds : []);

  // Create a message handler that includes the taskId
  const createMessageHandler = (taskId: string) => {
    if (!onSendMessageToAgent) return undefined;
    return (message: string, agentName?: string) =>
      onSendMessageToAgent(taskId, message, agentName);
  };

  return (
    <KanbanProvider onDragEnd={onDragEnd}>
      {Object.entries(groupedTasks).map(([status, statusTasks]) => (
        <KanbanBoard key={status} id={status as TaskStatus}>
          <KanbanHeader
            name={statusLabels[status as TaskStatus]}
            color={statusBoardColors[status as TaskStatus]}
          />
          <KanbanCards>
            {statusTasks.map((task, index) => {
              const cardData = cardDataMap.get(task.id);

              if (useEnhancedCards) {
                return (
                  <EnhancedTaskCard
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
                    primaryArtifact={cardData?.primaryArtifact}
                    artifacts={cardData?.artifacts}
                    workflowEvents={cardData?.workflowEvents}
                    onSendMessage={createMessageHandler(task.id)}
                    defaultMode={defaultCardMode}
                  />
                );
              }

              return (
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
              );
            })}
          </KanbanCards>
        </KanbanBoard>
      ))}
    </KanbanProvider>
  );
}

export default memo(TaskKanbanBoard);
