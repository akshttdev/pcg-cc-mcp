import { useCallback, useEffect, useState, useMemo } from 'react';
import { useLocation, useNavigate, useParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { AlertTriangle, Plus } from 'lucide-react';
import { Loader } from '@/components/ui/loader';
import { projectsApi, tasksApi, attemptsApi } from '@/lib/api';
import { openTaskForm } from '@/lib/openTaskForm';
import { ViewSwitcher } from '@/components/views/ViewSwitcher';
import { TableView } from '@/components/views/TableView';
import { GalleryView } from '@/components/views/GalleryView';
import { TimelineView } from '@/components/views/TimelineView';
import { CalendarView } from '@/components/views/CalendarView';
import { useViewStore } from '@/stores/useViewStore';
import { TagManager } from '@/components/tags/TagManager';
import { useBulkSelectionStore } from '@/stores/useBulkSelectionStore';
import { BulkSelectionToolbar } from '@/components/bulk-operations/BulkSelectionToolbar';
import { CheckSquare, Download, Upload } from 'lucide-react';
import { FilterButton } from '@/components/filters/FilterButton';
import { FilterPanel } from '@/components/filters/FilterPanel';
import { SavedFiltersMenu } from '@/components/filters/SavedFiltersMenu';
import { useFilterStore } from '@/stores/useFilterStore';
import { applyFilters } from '@/utils/filterUtils';
import { ExportDialog } from '@/components/export/ExportDialog';
import { ImportDialog } from '@/components/export/ImportDialog';

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

import TaskKanbanBoard from '@/components/tasks/TaskKanbanBoard';
import { TaskDetailsPanel } from '@/components/tasks/TaskDetailsPanel';
import type { TaskWithAttemptStatus, Project, TaskAttempt } from 'shared/types';
import type { DragEndEvent } from '@/components/ui/shadcn-io/kanban';
import { useProjectTasks } from '@/hooks/useProjectTasks';
import { useTaskAgentFlowMap } from '@/hooks/useAgentFlows';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import NiceModal from '@ebay/nice-modal-react';
import { useHotkeysContext } from 'react-hotkeys-hook';

type Task = TaskWithAttemptStatus;

export function ProjectTasks() {
  const { t } = useTranslation(['tasks', 'common']);
  const { projectId, taskId, attemptId } = useParams<{
    projectId: string;
    taskId?: string;
    attemptId?: string;
  }>();
  const location = useLocation();
  const navigate = useNavigate();
  const { enableScope, disableScope } = useHotkeysContext();

  useEffect(() => {
    enableScope(Scope.KANBAN);

    return () => {
      disableScope(Scope.KANBAN);
    };
  }, [enableScope, disableScope]);

  const [project, setProject] = useState<Project | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filterPanelOpen, setFilterPanelOpen] = useState(false);
  const [exportDialogOpen, setExportDialogOpen] = useState(false);
  const [importDialogOpen, setImportDialogOpen] = useState(false);
  const { currentViewType } = useViewStore();
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

  // Helper functions to open task forms
  const handleCreateTask = () => {
    if (project?.id) {
      openTaskForm({ projectId: project.id });
    }
  };

  const handleEditTask = (task: Task) => {
    if (project?.id) {
      openTaskForm({ projectId: project.id, task });
    }
  };

  const handleDuplicateTask = (task: Task) => {
    if (project?.id) {
      openTaskForm({ projectId: project.id, initialTask: task });
    }
  };
  const { query: searchQuery, focusInput } = useSearch();

  // Panel state
  const [selectedTask, setSelectedTask] = useState<Task | null>(null);
  const [isPanelOpen, setIsPanelOpen] = useState(false);

  // Fullscreen state using custom hook
  const { isFullscreen, navigateToTask, navigateToAttempt, toggleFullscreen } =
    useTaskViewManager();

  // Attempts fetching (only when task is selected)
  const { data: attempts = [] } = useQuery({
    queryKey: ['taskAttempts', selectedTask?.id],
    queryFn: () => attemptsApi.getAll(selectedTask!.id),
    enabled: !!selectedTask?.id,
    refetchInterval: 5000,
  });

  // Selected attempt logic
  const selectedAttempt = useMemo(() => {
    if (!attempts.length) return null;
    if (attemptId) {
      const found = attempts.find((a) => a.id === attemptId);
      if (found) return found;
    }
    return attempts[0] || null; // Most recent fallback
  }, [attempts, attemptId]);

  // Navigation callback for attempt selection
  const setSelectedAttempt = useCallback(
    (attempt: TaskAttempt | null) => {
      if (!selectedTask) return;

      if (attempt) {
        navigateToAttempt(projectId!, selectedTask.id, attempt.id);
      } else {
        navigateToTask(projectId!, selectedTask.id);
      }
    },
    [navigateToTask, navigateToAttempt, projectId, selectedTask]
  );

  // Stream tasks for this project
  const {
    tasks,
    tasksById,
    isLoading,
    error: streamError,
  } = useProjectTasks(projectId || '');

  // Fetch agent flows for all tasks to display on cards
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

  // Define task creation handler
  const handleCreateNewTask = useCallback(() => {
    handleCreateTask();
  }, [handleCreateTask]);

  // Semantic keyboard shortcuts for kanban page
  // Prevent default is needed to stop the input having the value 'c'
  useKeyCreate(handleCreateNewTask, {
    scope: Scope.KANBAN,
    preventDefault: true,
  });

  useKeyFocusSearch(
    () => {
      focusInput();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true, // Prevent Firefox quick find
    }
  );

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

  // Toggle fullscreen with Cmd+Enter
  useKeyToggleFullscreen(() => toggleFullscreen(!isFullscreen), {
    scope: Scope.KANBAN,
  });

  // Navigation shortcuts using semantic hooks
  const taskStatuses = [
    'todo',
    'inprogress',
    'inreview',
    'done',
    'cancelled',
  ] as const;

  // Memoize filtered tasks based on search query and filters
  const boardFilter = useMemo(() => {
    const params = new URLSearchParams(location.search);
    return params.get('board') ?? null;
  }, [location.search]);

  const filteredTasks = useMemo(() => {
    let result = tasks;

    if (boardFilter) {
      if (boardFilter === 'unassigned') {
        result = result.filter((task) => !task.board_id);
      } else {
        result = result.filter((task) => task.board_id === boardFilter);
      }
    }

    // Apply search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      result = result.filter(
        (task) =>
          task.title.toLowerCase().includes(query) ||
          (task.description && task.description.toLowerCase().includes(query))
      );
    }

    // Apply advanced filters
    if (projectId) {
      const activeFilters = getActiveFilters(projectId);
      result = applyFilters(result, activeFilters);
    }

    return result;
  }, [tasks, boardFilter, searchQuery, projectId, getActiveFilters]);

  // Memoize grouped filtered tasks
  const groupedFilteredTasks = useMemo(() => {
    const groups: Record<string, Task[]> = {};
    taskStatuses.forEach((status) => {
      groups[status] = [];
    });
    filteredTasks.forEach((task) => {
      const normalizedStatus = task.status.toLowerCase();
      if (groups[normalizedStatus]) {
        groups[normalizedStatus].push(task);
      } else {
        groups['todo'].push(task);
      }
    });
    return groups;
  }, [filteredTasks]);

  useKeyNavUp(
    () => {
      selectPreviousTask();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  useKeyNavDown(
    () => {
      selectNextTask();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  useKeyNavLeft(
    () => {
      selectPreviousColumn();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true, // Prevent page scroll
    }
  );

  useKeyNavRight(
    () => {
      selectNextColumn();
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true, // Prevent page scroll
    }
  );

  useKeyOpenDetails(() => {}, { scope: Scope.KANBAN });

  // Delete task shortcut
  useKeyDeleteTask(
    () => {
      if (selectedTask) {
        handleDeleteTask(selectedTask.id);
      }
    },
    {
      scope: Scope.KANBAN,
      preventDefault: true,
    }
  );

  // Full screen

  const fetchProject = useCallback(async () => {
    try {
      const result = await projectsApi.getById(projectId!);
      setProject(result);
    } catch (err) {
      setError('Failed to load project');
    }
  }, [projectId]);

  const handleClosePanel = useCallback(() => {
    // setIsPanelOpen(false);
    // setSelectedTask(null);
    // Remove task ID from URL when closing panel
    navigate(`/projects/${projectId}/tasks`, { replace: true });
  }, [projectId, navigate]);

  const handleDeleteTask = useCallback(
    (taskId: string) => {
      const task = tasksById[taskId];
      if (task) {
        NiceModal.show('delete-task-confirmation', {
          task,
          projectId: projectId!,
        })
          .then(() => {
            // Task was deleted, close panel if this task was selected
            if (selectedTask?.id === taskId) {
              handleClosePanel();
            }
          })
          .catch(() => {
            // Modal was cancelled - do nothing
          });
      }
    },
    [tasksById, projectId, selectedTask, handleClosePanel]
  );

  const handleEditTaskCallback = useCallback(
    (task: Task) => {
      handleEditTask(task);
    },
    [handleEditTask]
  );

  const handleDuplicateTaskCallback = useCallback(
    (task: Task) => {
      handleDuplicateTask(task);
    },
    [handleDuplicateTask]
  );

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

  // Navigation functions that use filtered/grouped tasks
  const selectNextTask = useCallback(() => {
    if (selectedTask) {
      const tasksInStatus = groupedFilteredTasks[selectedTask.status] || [];
      const currentIndex = tasksInStatus.findIndex(
        (task) => task.id === selectedTask.id
      );
      if (currentIndex >= 0 && currentIndex < tasksInStatus.length - 1) {
        handleViewTaskDetails(tasksInStatus[currentIndex + 1]);
      }
    } else {
      // Find first non-empty column
      for (const status of taskStatuses) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, handleViewTaskDetails]);

  const selectPreviousTask = useCallback(() => {
    if (selectedTask) {
      const tasksInStatus = groupedFilteredTasks[selectedTask.status] || [];
      const currentIndex = tasksInStatus.findIndex(
        (task) => task.id === selectedTask.id
      );
      if (currentIndex > 0) {
        handleViewTaskDetails(tasksInStatus[currentIndex - 1]);
      }
    } else {
      // Find first non-empty column
      for (const status of taskStatuses) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, handleViewTaskDetails]);

  const selectNextColumn = useCallback(() => {
    if (selectedTask) {
      const currentIndex = taskStatuses.findIndex(
        (status) => status === selectedTask.status
      );
      // Find next non-empty column
      for (let i = currentIndex + 1; i < taskStatuses.length; i++) {
        const tasks = groupedFilteredTasks[taskStatuses[i]];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          return;
        }
      }
    } else {
      // Find first non-empty column
      for (const status of taskStatuses) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, handleViewTaskDetails]);

  const selectPreviousColumn = useCallback(() => {
    if (selectedTask) {
      const currentIndex = taskStatuses.findIndex(
        (status) => status === selectedTask.status
      );
      // Find previous non-empty column
      for (let i = currentIndex - 1; i >= 0; i--) {
        const tasks = groupedFilteredTasks[taskStatuses[i]];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          return;
        }
      }
    } else {
      // Find first non-empty column
      for (const status of taskStatuses) {
        const tasks = groupedFilteredTasks[status];
        if (tasks && tasks.length > 0) {
          handleViewTaskDetails(tasks[0]);
          break;
        }
      }
    }
  }, [selectedTask, groupedFilteredTasks, handleViewTaskDetails]);

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
        // UI will update via WebSocket stream
      } catch (err) {
        setError('Failed to update task status');
      }
    },
    [tasksById]
  );

  // Initialize project when projectId changes
  useEffect(() => {
    if (projectId) {
      fetchProject();
    }
  }, [projectId, fetchProject]);

  // Remove legacy direct-navigation handler; live sync above covers this

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

      {/* Kanban + Panel Container - uses side-by-side layout on xl+ */}
      <div className="flex-1 min-h-0 xl:flex">
        {/* Left Column - Kanban Section */}
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

          {/* View Switcher */}
          {tasks && tasks.length > 0 && projectId && (
            <div className="px-6 py-4 border-b bg-background/95 backdrop-blur sticky top-0 z-10 flex items-center justify-between">
              <h2 className="text-lg font-semibold">{project?.name || 'Tasks'}</h2>
              <div className="flex items-center gap-2">
                <FilterButton
                  projectId={projectId}
                  onClick={() => setFilterPanelOpen(true)}
                />
                <SavedFiltersMenu projectId={projectId} />
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setImportDialogOpen(true)}
                  className="gap-2"
                >
                  <Upload className="h-4 w-4" />
                  Import
                </Button>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setExportDialogOpen(true)}
                  className="gap-2"
                >
                  <Download className="h-4 w-4" />
                  Export
                </Button>
                <Button
                  variant={selectionMode ? 'default' : 'outline'}
                  size="sm"
                  onClick={toggleSelectionMode}
                  className="gap-2"
                >
                  <CheckSquare className="h-4 w-4" />
                  {selectionMode ? 'Done' : 'Select'}
                </Button>
                <TagManager projectId={projectId} />
                <ViewSwitcher />
              </div>
            </div>
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
                // Tasks will auto-update via WebSocket stream
                setImportDialogOpen(false);
              }}
            />
          )}

          {!tasks || tasks.length === 0 ? (
            <div className="max-w-7xl mx-auto mt-8">
              <Card>
                <CardContent className="text-center py-8">
                  <p className="text-muted-foreground">{t('empty.noTasks')}</p>
                  <Button className="mt-4" onClick={handleCreateNewTask}>
                    <Plus className="h-4 w-4 mr-2" />
                    {t('empty.createFirst')}
                  </Button>
                </CardContent>
              </Card>
            </div>
          ) : !filteredTasks || filteredTasks.length === 0 ? (
            <div className="max-w-7xl mx-auto mt-8">
              <Card>
                <CardContent className="text-center py-8">
                  <p className="text-muted-foreground">
                    {t('empty.noSearchResults')}
                  </p>
                </CardContent>
              </Card>
            </div>
          ) : currentViewType === 'table' && projectId ? (
            <div className="w-full h-full p-6">
              <TableView
                tasks={filteredTasks}
                projectId={projectId}
                onEditTask={handleEditTaskCallback}
                onDeleteTask={handleDeleteTask}
                onDuplicateTask={handleDuplicateTaskCallback}
              />
            </div>
          ) : currentViewType === 'gallery' && projectId ? (
            <div className="w-full h-full p-6">
              <GalleryView
                tasks={filteredTasks}
                projectId={projectId}
                onEditTask={handleEditTaskCallback}
                onDeleteTask={handleDeleteTask}
                onDuplicateTask={handleDuplicateTaskCallback}
              />
            </div>
          ) : currentViewType === 'timeline' && projectId ? (
            <div className="w-full h-full">
              <TimelineView
                tasks={filteredTasks}
                onTaskClick={(task) => handleViewTaskDetails(task, undefined, true)}
              />
            </div>
          ) : currentViewType === 'calendar' && projectId ? (
            <div className="w-full h-full">
              <CalendarView
                tasks={filteredTasks}
                onTaskClick={(task) => handleViewTaskDetails(task, undefined, true)}
                onCreateTask={() => {
                  // Open task creation form with pre-filled date
                  openTaskForm({ projectId });
                }}
              />
            </div>
          ) : (
            <div className="w-full h-full p-6">
              <TaskKanbanBoard
                groupedTasks={groupedFilteredTasks}
                onDragEnd={handleDragEnd}
                onEditTask={handleEditTaskCallback}
                onDeleteTask={handleDeleteTask}
                onDuplicateTask={handleDuplicateTaskCallback}
                onViewTaskDetails={handleViewTaskDetails}
                selectedTask={selectedTask || undefined}
                selectionMode={selectionMode}
                isSelected={(taskId) => selectedTaskIds.has(taskId)}
                onToggleSelection={(taskId) => {
                  if (selectedTaskIds.has(taskId)) {
                    deselectTask(taskId);
                  } else {
                    selectTask(taskId);
                  }
                }}
                agentFlowMap={agentFlowMap}
              />
            </div>
          )}
        </div>

        {/* Right Column - Task Details Panel */}
        {isPanelOpen && (
          <TaskDetailsPanel
            task={selectedTask}
            projectHasDevScript={!!project?.dev_script}
            projectId={projectId!}
            onClose={handleClosePanel}
            onEditTask={handleEditTaskCallback}
            onDeleteTask={handleDeleteTask}
            onDuplicateTask={handleDuplicateTaskCallback}
            onNavigateToTask={(taskId) => {
              const task = tasksById[taskId];
              if (task) {
                handleViewTaskDetails(task, undefined, true);
              }
            }}
            isFullScreen={isFullscreen}
            selectedAttempt={selectedAttempt}
            attempts={attempts}
            setSelectedAttempt={setSelectedAttempt}
            tasksById={tasksById}
          />
        )}
      </div>
    </div>
  );
}
