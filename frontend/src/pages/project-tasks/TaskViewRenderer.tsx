import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Plus } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useViewStore } from '@/stores/useViewStore';
import { openTaskForm } from '@/lib/openTaskForm';
import { TableView } from '@/components/views/TableView';
import { GalleryView } from '@/components/views/GalleryView';
import { TimelineView } from '@/components/views/TimelineView';
import { CalendarView } from '@/components/views/CalendarView';
import { ProjectOverview } from '@/components/projects/ProjectOverview';
import TaskKanbanBoard from '@/components/tasks/TaskKanbanBoard';
import type { Project, TaskStatus } from 'shared/types';
import type { TaskWithArchive, UserListItem, AgentFlow } from '@/lib/api';
import type { AgentWithParsedFields } from 'shared/types';
import type { DragEndEvent } from '@/components/ui/shadcn-io/kanban';

type Task = TaskWithArchive;

interface TaskViewRendererProps {
  projectId: string;
  project: Project | null;
  tasks: Task[];
  filteredTasks: Task[];
  groupedFilteredTasks: Record<string, Task[]>;
  boardFilter: string | null;
  selectedTask: Task | undefined;
  selectionMode: boolean;
  selectedTaskIds: Set<string>;
  showArchived: boolean;
  useEnhancedCards: boolean;
  usersMap: Map<string, UserListItem>;
  agentsMap: Map<string, AgentWithParsedFields>;
  agentFlowMap: Map<string, AgentFlow>;
  onCreateTask: () => void;
  onEditTask: (task: Task) => void;
  onDeleteTask: (taskId: string) => void;
  onDuplicateTask: (task: Task) => void;
  onArchiveTask: (task: Task) => void;
  onViewTaskDetails: (task: Task, attemptId?: string, fullscreen?: boolean) => void;
  onDragEnd: (event: DragEndEvent) => void;
  onSelectTask: (taskId: string) => void;
  onDeselectTask: (taskId: string) => void;
  onSendMessageToAgent: (taskId: string, message: string, agentName?: string) => Promise<string>;
}

export function TaskViewRenderer({
  projectId,
  project,
  tasks,
  filteredTasks,
  groupedFilteredTasks,
  boardFilter,
  selectedTask,
  selectionMode,
  selectedTaskIds,
  showArchived,
  useEnhancedCards,
  usersMap,
  agentsMap,
  agentFlowMap,
  onCreateTask,
  onEditTask,
  onDeleteTask,
  onDuplicateTask,
  onArchiveTask,
  onViewTaskDetails,
  onDragEnd,
  onSelectTask,
  onDeselectTask,
  onSendMessageToAgent,
}: TaskViewRendererProps) {
  const { t } = useTranslation(['tasks', 'common']);
  const { currentViewType } = useViewStore();

  if (!tasks || tasks.length === 0) {
    return (
      <div className="max-w-7xl mx-auto mt-8">
        <Card>
          <CardContent className="text-center py-8">
            <p className="text-muted-foreground">{t('empty.noTasks')}</p>
            <Button className="mt-4" onClick={onCreateTask}>
              <Plus className="h-4 w-4 mr-2" />
              {t('empty.createFirst')}
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (!filteredTasks || filteredTasks.length === 0) {
    return (
      <div className="max-w-7xl mx-auto mt-8">
        <Card>
          <CardContent className="text-center py-8">
            <p className="text-muted-foreground">
              {t('empty.noSearchResults')}
            </p>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (currentViewType === 'overview' && project) {
    return <ProjectOverview project={project} tasks={filteredTasks} />;
  }

  if (currentViewType === 'table') {
    return (
      <div className="w-full h-full p-6">
        <TableView
          tasks={filteredTasks}
          projectId={projectId}
          onEditTask={onEditTask}
          onDeleteTask={onDeleteTask}
          onDuplicateTask={onDuplicateTask}
        />
      </div>
    );
  }

  if (currentViewType === 'gallery') {
    return (
      <div className="w-full h-full p-6">
        <GalleryView
          tasks={filteredTasks}
          projectId={projectId}
          onEditTask={onEditTask}
          onDeleteTask={onDeleteTask}
          onDuplicateTask={onDuplicateTask}
        />
      </div>
    );
  }

  if (currentViewType === 'timeline') {
    return (
      <div className="w-full h-full">
        <TimelineView
          tasks={filteredTasks}
          onTaskClick={(task) => onViewTaskDetails(task, undefined, true)}
        />
      </div>
    );
  }

  if (currentViewType === 'calendar') {
    return (
      <div className="w-full h-full">
        <CalendarView
          tasks={filteredTasks}
          onTaskClick={(task) => onViewTaskDetails(task, undefined, true)}
          onCreateTask={() => {
            openTaskForm({ projectId, initialBoardId: boardFilter });
          }}
        />
      </div>
    );
  }

  // Default: Kanban view
  return (
    <div className="w-full h-full p-6">
      <TaskKanbanBoard
        groupedTasks={groupedFilteredTasks}
        onDragEnd={onDragEnd}
        onEditTask={onEditTask}
        onDeleteTask={onDeleteTask}
        onDuplicateTask={onDuplicateTask}
        onArchiveTask={onArchiveTask}
        onViewTaskDetails={onViewTaskDetails}
        selectedTask={selectedTask}
        selectionMode={selectionMode}
        isSelected={(taskId) => selectedTaskIds.has(taskId)}
        onToggleSelection={(taskId) => {
          if (selectedTaskIds.has(taskId)) {
            onDeselectTask(taskId);
          } else {
            onSelectTask(taskId);
          }
        }}
        agentFlowMap={agentFlowMap}
        useEnhancedCards={useEnhancedCards}
        onSendMessageToAgent={onSendMessageToAgent}
        showArchived={showArchived}
        usersMap={usersMap}
        agentsMap={agentsMap}
        onCreateTask={(status) => {
          openTaskForm({ projectId, initialBoardId: boardFilter, initialStatus: status as TaskStatus });
        }}
      />
    </div>
  );
}
