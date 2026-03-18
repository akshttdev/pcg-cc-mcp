import { useCallback, useEffect, useState, useMemo } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AlertTriangle } from 'lucide-react';
import { Loader } from '@/components/ui/loader';
import { projectsApi, tasksApi, agentsApi, usersApi, resolveApiUrl } from '@/lib/api';
import type { UserListItem } from '@/lib/api';
import type { AgentChatRequest } from 'shared/types';
import { openTaskForm } from '@/lib/openTaskForm';
import { useViewStore } from '@/stores/useViewStore';
import { useBulkSelectionStore } from '@/stores/useBulkSelectionStore';
import { BulkSelectionToolbar } from '@/components/bulk-operations/BulkSelectionToolbar';
import { FilterPanel } from '@/components/filters/FilterPanel';
import { ExportDialog } from '@/components/export/ExportDialog';
import { ImportDialog } from '@/components/export/ImportDialog';
import { useFilterStore } from '@/stores/useFilterStore';
import { applyFilters } from '@/utils/filterUtils';

import { useSearch } from '@/contexts/search-context';
import { useQuery } from '@tanstack/react-query';
import { useTaskViewManager } from '@/hooks/useTaskViewManager';
import {
  useKeyCreate,
  useKeyExit,
  useKeyFocusSearch,
  useKeyNavUp,
  useKeyNavDown,
  useKeyNavLeft,
  useKeyNavRight,
  useKeyOpenDetails,
  Scope,
  useKeyToggleFullscreen,
  useKeyDeleteTask,
} from '@/keyboard';

import {
  getKanbanSectionClasses,
  getMainContainerClasses,
} from '@/lib/responsive-config';

import { EnhancedTaskDetailsPanel } from '@/components/tasks';
import { ResizableDrawer } from '@/components/ui/resizable-drawer';
import type { Project } from 'shared/types';
import type { TaskWithArchive } from '@/lib/api';
import type { DragEndEvent } from '@/components/ui/shadcn-io/kanban';
import { useProjectTasks } from '@/hooks/useProjectTasks';
import { useProjectAccess } from '@/hooks/useProjectAccess';
import { useTaskAgentFlowMap } from '@/hooks/useAgentFlows';
import { useTaskChangeNotifications } from '@/hooks/useTaskChangeNotifications';
import { useAuth } from '@/contexts/AuthContext';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import NiceModal from '@ebay/nice-modal-react';
import { useHotkeysContext } from 'react-hotkeys-hook';

import { ProjectTasksToolbar } from './ProjectTasksToolbar';
import { TaskViewRenderer } from './TaskViewRenderer';
import { useTaskKeyboardNav, TASK_STATUSES } from './useTaskKeyboardNav';

type Task = TaskWithArchive;

export function ProjectTasks() {
  const { t } = useTranslation(['tasks', 'common']);
  const { projectId, taskId } = useParams<{
    projectId: string;
    taskId?: string;
    attemptId?: string;
  }>();
  const location = useLocation();
  const navigate = useNavigate();
  const { enableScope, disableScope } = useHotkeysContext();

  useEffect(() => {
    enableScope(Scope.KANBAN);
    return () => { disableScope(Scope.KANBAN); };
  }, [enableScope, disableScope]);

  const [project, setProject] = useState<Project | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filterPanelOpen, setFilterPanelOpen] = useState(false);
  const [showArchived, setShowArchived] = useState(false);
  const [hideTestTasks, setHideTestTasks] = useState(() => {
    try {
      return localStorage.getItem('orcha:hide-test-tasks') === 'true';
    } catch {
      return false;
    }
  });
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const { useEnhancedCards, sortOption } = useViewStore();
  const {
    selectionMode,
    selectedTaskIds,
    toggleSelectionMode,
    getSelectedCount,
    selectAll,
    clearSelection,
    selectTask,
    deselectTask,
  } = useBulkSelectionStore();
  const { getActiveFilters } = useFilterStore();

  // Extract board filter from URL
  const boardFilter = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get('board') ?? null;
  }, [location.search]);

  // Helper functions to open task forms
  const handleCreateTask = useCallback(() => {
    console.log('[ProjectTasks] Creating task with boardFilter:', boardFilter, 'URL search:', location.search);
    if (project?.id) {
      openTaskForm({ projectId: project.id, initialBoardId: boardFilter });
    }
  }, [project?.id, boardFilter, location.search]);

  const handleEditTask = useCallback((task: Task) => {
    if (project?.id) {
      openTaskForm({ projectId: project.id, task });
    }
  }, [project?.id]);

  const handleDuplicateTask = useCallback((task: Task) => {
    if (project?.id) {
      openTaskForm({ projectId: project.id, initialTask: task });
    }
  }, [project?.id]);

  const { query: searchQuery, focusInput } = useSearch();

  // Panel state
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  const [isPanelOpen, setIsPanelOpen] = useState(false);

  // Fullscreen state
  const { isFullscreen, navigateToTask, navigateToAttempt, toggleFullscreen } =
    useTaskViewManager();

  const { user } = useAuth();

  // Fetch users for assignee display
  const { data: usersData } = useQuery({
    queryKey: ['users'],
    queryFn: () => usersApi.list(),
    staleTime: 5 * 60 * 1000,
  });

  const usersMap = useMemo(() => {
    const map = new Map<string, UserListItem>();
    usersData?.forEach(u => map.set(u.id, u));
    return map;
  }, [usersData]);

  // Stream tasks for this project
  const {
    tasks: allTasks,
    tasksById,
    isLoading,
    error: streamError,
  } = useProjectTasks(projectId || '');

  useTaskChangeNotifications(tasksById);

  const { data: projectAccess } = useProjectAccess(projectId);

  // Apply access scope
  const tasks = useMemo(() => {
    if (!projectAccess || projectAccess.access_scope !== 'assigned_only' || !user) {
      return allTasks;
    }
    return allTasks.filter(t => t.assignee_id === user.id);
  }, [allTasks, projectAccess, user]);

  const taskIds = useMemo(() => tasks.map(t => t.id), [tasks]);
  const { flowMap: agentFlowMap } = useTaskAgentFlowMap(taskIds);

  // Sync selectedTask with URL params and live task updates
  useEffect(() => {
    if (taskId) {
      const t = taskId ? tasksById[taskId] : undefined;
      if (t) {
        setSelectedTask(t);
        setIsPanelOpen(true);
      }
    } else {
      setSelectedTask(null);
      setIsPanelOpen(false);
    }
  }, [taskId, tasksById]);

  const handleCreateNewTask = handleCreateTask;

  // Memoize filtered tasks
  const archivedCount = useMemo(() => tasks.filter((t) => t.archived_at).length, [tasks]);

  const filteredTasks = useMemo(() => {
    let result = tasks;

    if (!showArchived) {
      result = result.filter((t) => !t.archived_at);
    }

    if (hideTestTasks) {
      result = result.filter((t) => !t.title.startsWith('[E2E]') && !t.title.startsWith('[Test]'));
    }

    if (boardFilter) {
      if (boardFilter === 'unassigned') {
        result = result.filter((task) => !task.board_id);
      } else {
        result = result.filter((task) => task.board_id === boardFilter);
      }
    }

    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (task) =>
          task.title.toLowerCase().includes(query) ||
          (task.description && task.description.toLowerCase().includes(query))
      );
    }

    if (projectId) {
      const activeFilters = getActiveFilters(projectId);
      result = applyFilters(result, activeFilters);
    }

    return result;
  }, [tasks, boardFilter, searchQuery, projectId, getActiveFilters, showArchived, hideTestTasks]);

  // Memoize grouped filtered tasks, sorted by active sort option
  const groupedFilteredTasks = useMemo(() => {
    const priorityOrder: Record<string, number> = { critical: 0, high: 1, medium: 2, low: 3 };
    const groups: Record<string, Task[]> = {};
    TASK_STATUSES.forEach((status) => { groups[status] = []; });
    filteredTasks.forEach((task) => {
      const normalizedStatus = task.status.toLowerCase();
      if (groups[normalizedStatus]) {
        groups[normalizedStatus].push(task);
      } else {
        groups['todo'].push(task);
      }
    });

    const dir = sortOption.direction === 'asc' ? 1 : -1;

    for (const status of TASK_STATUSES) {
      groups[status].sort((a, b) => {
        let cmp = 0;
        switch (sortOption.field) {
          case 'priority': {
            const pa = priorityOrder[a.priority || 'medium'] ?? 2;
            const pb = priorityOrder[b.priority || 'medium'] ?? 2;
            cmp = pa - pb;
            if (cmp === 0) {
              const da = a.due_date ? new Date(a.due_date).getTime() : Infinity;
              const db = b.due_date ? new Date(b.due_date).getTime() : Infinity;
              cmp = da - db;
            }
            break;
          }
          case 'due_date': {
            const da = a.due_date ? new Date(a.due_date).getTime() : Infinity;
            const db = b.due_date ? new Date(b.due_date).getTime() : Infinity;
            cmp = da - db;
            break;
          }
          case 'updated_at': {
            cmp = new Date(a.updated_at).getTime() - new Date(b.updated_at).getTime();
            break;
          }
          case 'created_at': {
            cmp = new Date(a.created_at).getTime() - new Date(b.created_at).getTime();
            break;
          }
          case 'assignee_id': {
            cmp = (a.assignee_id || '').localeCompare(b.assignee_id || '');
            break;
          }
          case 'title': {
            cmp = a.title.localeCompare(b.title);
            break;
          }
        }
        return cmp * dir;
      });
    }
    return groups;
  }, [filteredTasks, sortOption]);

  const handleViewTaskDetails = useCallback(
    (task: Task, attemptIdToShow?: string, fullscreen?: boolean) => {
      if (attemptIdToShow) {
        navigateToAttempt(projectId!, task.id, attemptIdToShow, { fullscreen });
      } else {
        navigateToTask(projectId!, task.id, { fullscreen });
      }
    },
    [projectId, navigateToTask, navigateToAttempt]
  );

  // Keyboard navigation
  const { selectNextTask, selectPreviousTask, selectNextColumn, selectPreviousColumn } =
    useTaskKeyboardNav({
      selectedTask,
      groupedFilteredTasks,
      onViewTaskDetails: handleViewTaskDetails,
    });

  // Keyboard shortcuts
  useKeyCreate(handleCreateNewTask, { scope: Scope.KANBAN, preventDefault: true });
  useKeyFocusSearch(() => { focusInput(); }, { scope: Scope.KANBAN, preventDefault: true });

  const handleClosePanel = useCallback(() => {
    navigate(`/projects/${projectId}/tasks${location.search}`, { replace: true });
  }, [projectId, navigate, location.search]);

  useKeyExit(
    () => {
      if (isPanelOpen) {
        if (isFullscreen) {
          toggleFullscreen(false);
        } else {
          handleClosePanel();
        }
      } else {
        navigate('/projects');
      }
    },
    { scope: Scope.KANBAN }
  );

  useKeyToggleFullscreen(() => toggleFullscreen(!isFullscreen), { scope: Scope.KANBAN });
  useKeyNavUp(() => { selectPreviousTask(); }, { scope: Scope.KANBAN, preventDefault: true });
  useKeyNavDown(() => { selectNextTask(); }, { scope: Scope.KANBAN, preventDefault: true });
  useKeyNavLeft(() => { selectPreviousColumn(); }, { scope: Scope.KANBAN, preventDefault: true });
  useKeyNavRight(() => { selectNextColumn(); }, { scope: Scope.KANBAN, preventDefault: true });
  useKeyOpenDetails(() => {}, { scope: Scope.KANBAN });

  const handleDeleteTask = useCallback(
    (taskId: string) => {
      const task = tasksById[taskId];
      if (task) {
        NiceModal.show('delete-task-confirmation', { task, projectId: projectId! })
          .then(() => {
            if (selectedTask?.id === taskId) {
              handleClosePanel();
            }
          })
          .catch(() => {});
      }
    },
    [tasksById, projectId, selectedTask, handleClosePanel]
  );

  useKeyDeleteTask(
    () => { if (selectedTask) { handleDeleteTask(selectedTask.id); } },
    { scope: Scope.KANBAN, preventDefault: true }
  );

  const handleArchiveTask = useCallback(
    async (task: Task) => {
      try {
        if (task.archived_at) {
          await tasksApi.unarchive(task.id);
        } else {
          await tasksApi.archive(task.id);
          if (selectedTask?.id === task.id) {
            handleClosePanel();
          }
        }
      } catch (err) {
        setError('Failed to archive task');
      }
    },
    [selectedTask, handleClosePanel]
  );

  const handleSendMessageToAgent = useCallback(
    async (taskId: string, message: string, agentName?: string): Promise<string> => {
      const task = tasksById[taskId];
      if (!task) throw new Error('Task not found');

      const targetAgentName = agentName || task.assigned_agent;

      if (targetAgentName) {
        try {
          const agent = await agentsApi.getByName(targetAgentName);
          const request: AgentChatRequest = {
            message,
            sessionId: `task-${taskId}`,
            projectId: projectId || null,
            context: {
              taskId: task.id,
              taskTitle: task.title,
              isWorkflowFollowUp: true,
            },
            stream: false,
            model: null,
            provider: null,
          };
          const response = await agentsApi.chat(agent.id, request);
          return response.content;
        } catch (error) {
          console.warn(`Agent chat failed for ${targetAgentName}, falling back to Nora:`, error);
        }
      }

      const agentContext = agentName ? `[To ${agentName}] ` : '';
      const contextualMessage = `${agentContext}Regarding task "${task.title}": ${message}`;

      const response = await fetch(resolveApiUrl('/api/nora/chat'), {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          message: contextualMessage,
          sessionId: `task-${taskId}`,
          requestType: 'textInteraction',
          voiceEnabled: false,
          priority: 'normal',
          context: {
            taskId: task.id,
            taskTitle: task.title,
            projectId,
            executingAgent: agentName,
            isWorkflowFollowUp: true,
          },
        }),
      });

      if (!response.ok) {
        throw new Error(`Failed to send message${agentName ? ` to ${agentName}` : ''}`);
      }

      const data = await response.json();
      return data.content || data.message || data.response || 'Response received';
    },
    [tasksById, projectId]
  );

  const handleDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const { active, over } = event;
      if (!over || !active.data.current) return;

      const draggedTaskId = active.id as string;
      const newStatus = over.id as Task['status'];
      const task = tasksById[draggedTaskId];
      if (!task || task.status === newStatus) return;

      try {
        await tasksApi.update(draggedTaskId, {
          title: task.title,
          description: task.description,
          status: newStatus,
          parent_task_attempt: task.parent_task_attempt,
          image_ids: null,
        });
      } catch (err) {
        setError('Failed to update task status');
      }
    },
    [tasksById]
  );

  const fetchProject = useCallback(async () => {
    try {
      const result = await projectsApi.getById(projectId!);
      setProject(result);
    } catch (err) {
      setError('Failed to load project');
    }
  }, [projectId]);

  useEffect(() => {
    if (projectId) {
      fetchProject();
    }
  }, [projectId, fetchProject]);

  if (isLoading) {
    return <Loader message={t('loading')} size={32} className="py-8" />;
  }

  if (error) {
    return (
      <div className="p-4">
        <Alert>
          <AlertTitle className="flex items-center gap-2">
            <AlertTriangle size="16" />
            {t('common:states.error')}
          </AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      </div>
    );
  }

  return (
    <div
      className={`min-h-full ${getMainContainerClasses(isPanelOpen, isFullscreen)}`}
    >
      {streamError && (
        <Alert className="w-full z-30 xl:sticky xl:top-0">
          <AlertTitle className="flex items-center gap-2">
            <AlertTriangle size="16" />
            {t('common:states.reconnecting')}
          </AlertTitle>
          <AlertDescription>{streamError}</AlertDescription>
        </Alert>
      )}

      <div className="flex-1 min-h-0 xl:flex relative">
        <div className={getKanbanSectionClasses(isPanelOpen, isFullscreen)}>
          {/* Bulk Selection Toolbar */}
          {selectionMode && getSelectedCount() > 0 && tasks && projectId && (
            <BulkSelectionToolbar
              projectId={projectId}
              selectedCount={getSelectedCount()}
              totalCount={filteredTasks.length}
              onClearSelection={clearSelection}
              onSelectAll={() => selectAll(filteredTasks.map((t) => t.id))}
            />
          )}

          {/* View Switcher / Toolbar */}
          {tasks && tasks.length > 0 && projectId && (
            <ProjectTasksToolbar
              projectId={projectId}
              project={project}
              selectionMode={selectionMode}
              onToggleSelectionMode={toggleSelectionMode}
              showArchived={showArchived}
              archivedCount={archivedCount}
              onToggleArchived={() => setShowArchived(!showArchived)}
              hideTestTasks={hideTestTasks}
              onToggleHideTests={() => {
                const next = !hideTestTasks;
                setHideTestTasks(next);
                try { localStorage.setItem('orcha:hide-test-tasks', String(next)); } catch { /* non-fatal */ }
              }}
              onFilterClick={() => setFilterPanelOpen(true)}
              onImportClick={() => setImportDialogOpen(true)}
              onExportClick={() => setExportDialogOpen(true)}
            />
          )}

          {/* Filter Panel */}
          {projectId && (
            <FilterPanel
              open={filterPanelOpen}
              onOpenChange={setFilterPanelOpen}
              projectId={projectId}
            />
          )}

          {/* Export Dialog */}
          {projectId && project && (
            <ExportDialog
              open={exportDialogOpen}
              onOpenChange={setExportDialogOpen}
              tasks={filteredTasks}
              projectName={project.name}
            />
          )}

          {/* Import Dialog */}
          {projectId && (
            <ImportDialog
              open={importDialogOpen}
              onOpenChange={setImportDialogOpen}
              projectId={projectId}
              onImportComplete={() => {
                setImportDialogOpen(false);
              }}
            />
          )}

          <TaskViewRenderer
            projectId={projectId!}
            project={project}
            tasks={tasks}
            filteredTasks={filteredTasks}
            groupedFilteredTasks={groupedFilteredTasks}
            boardFilter={boardFilter}
            selectedTask={selectedTask || undefined}
            selectionMode={selectionMode}
            selectedTaskIds={selectedTaskIds}
            showArchived={showArchived}
            useEnhancedCards={useEnhancedCards}
            usersMap={usersMap}
            agentFlowMap={agentFlowMap}
            onCreateTask={handleCreateNewTask}
            onEditTask={handleEditTask}
            onDeleteTask={handleDeleteTask}
            onDuplicateTask={handleDuplicateTask}
            onArchiveTask={handleArchiveTask}
            onViewTaskDetails={handleViewTaskDetails}
            onDragEnd={handleDragEnd}
            onSelectTask={selectTask}
            onDeselectTask={deselectTask}
            onSendMessageToAgent={handleSendMessageToAgent}
          />
        </div>

        {/* Task Details Drawer / Fullscreen Panel */}
        {isPanelOpen && selectedTask && isFullscreen && (
          <EnhancedTaskDetailsPanel
            task={selectedTask}
            projectId={projectId!}
            onClose={handleClosePanel}
            onEdit={() => handleEditTask(selectedTask)}
            onDelete={() => handleDeleteTask(selectedTask.id)}
            onDuplicate={() => handleDuplicateTask(selectedTask)}
            onToggleFullscreen={() => toggleFullscreen(!isFullscreen)}
            isFullscreen={isFullscreen}
            className="fixed inset-0 z-50"
          />
        )}
        <ResizableDrawer
          open={isPanelOpen && !!selectedTask && !isFullscreen}
          onClose={handleClosePanel}
        >
          {({ isExpanded, toggleExpand }) =>
            selectedTask && (
              <EnhancedTaskDetailsPanel
                task={selectedTask}
                projectId={projectId!}
                onClose={handleClosePanel}
                onEdit={() => handleEditTask(selectedTask)}
                onDelete={() => handleDeleteTask(selectedTask.id)}
                onDuplicate={() => handleDuplicateTask(selectedTask)}
                onToggleExpand={toggleExpand}
                isExpanded={isExpanded}
                isFullscreen={false}
                className="h-full"
              />
            )
          }
        </ResizableDrawer>
      </div>
    </div>
  );
}
