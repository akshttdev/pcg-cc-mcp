import { useState, useEffect, useCallback, useMemo } from 'react';
import { Globe2, Settings2, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { ImageUploadSection } from '@/components/ui/ImageUploadSection';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { RichTextEditor } from '@/components/editor/RichTextEditor';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { templatesApi, imagesApi, projectsApi, attemptsApi } from '@/lib/api';
import { useTaskMutations } from '@/hooks/useTaskMutations';
import { useUserSystem } from '@/components/config-provider';
import { ExecutorProfileSelector } from '@/components/settings';
import BranchSelector from '@/components/tasks/BranchSelector';
import type {
  TaskStatus,
  TaskTemplate,
  ImageResponse,
  GitBranch,
  ExecutorProfileId,
  Priority,
  Task,
  ProjectBoard,
} from 'shared/types';
import NiceModal, { useModal } from '@ebay/nice-modal-react';
import { Switch } from '@/components/ui/switch';
import { Textarea } from '@/components/ui/textarea';

export interface TaskFormDialogProps {
  task?: Task | null; // Optional for create mode
  projectId?: string; // For file search functionality
  initialTemplate?: TaskTemplate | null; // For pre-filling from template
  initialTask?: Task | null; // For duplicating an existing task
  initialBaseBranch?: string; // For pre-selecting base branch in spinoff
  parentTaskAttemptId?: string; // For linking to parent task attempt
}

export const TaskFormDialog = NiceModal.create<TaskFormDialogProps>(
  ({
    task,
    projectId,
    initialTemplate,
    initialTask,
    initialBaseBranch,
    parentTaskAttemptId,
  }) => {
    const modal = useModal();
    const { createTask, createAndStart, updateTask } =
      useTaskMutations(projectId);
    const { system, profiles } = useUserSystem();
    const [title, setTitle] = useState('');
    const [description, setDescription] = useState('');
    const [status, setStatus] = useState<TaskStatus>('todo');
    const [priority, setPriority] = useState<Priority>('medium');
    const [assigneeId, setAssigneeId] = useState('');
    const [assignedAgent, setAssignedAgent] = useState('');
    const [assignedMcpsInput, setAssignedMcpsInput] = useState('');
    const [tagsInput, setTagsInput] = useState('');
    const [requiresApproval, setRequiresApproval] = useState(false);
    const [dueDate, setDueDate] = useState('');
    const [isSubmitting, setIsSubmitting] = useState(false);
    const [isSubmittingAndStart, setIsSubmittingAndStart] = useState(false);
    const [templates, setTemplates] = useState<TaskTemplate[]>([]);
    const [selectedTemplate, setSelectedTemplate] = useState<string>('');
    const [showDiscardWarning, setShowDiscardWarning] = useState(false);
    const [images, setImages] = useState<ImageResponse[]>([]);
    const [newlyUploadedImageIds, setNewlyUploadedImageIds] = useState<
      string[]
    >([]);
    const [branches, setBranches] = useState<GitBranch[]>([]);
    const [selectedBranch, setSelectedBranch] = useState<string>('');
    const [selectedExecutorProfile, setSelectedExecutorProfile] =
      useState<ExecutorProfileId | null>(null);
    const [boards, setBoards] = useState<ProjectBoard[]>([]);
    const [boardsLoading, setBoardsLoading] = useState(false);
    const [boardsError, setBoardsError] = useState<string | null>(null);
    const [selectedBoardId, setSelectedBoardId] = useState<string | null>(null);
    const [quickstartExpanded, setQuickstartExpanded] =
      useState<boolean>(false);

    const isEditMode = Boolean(task);

    const createdByFallback = useMemo(
      () => system.config?.github?.username || 'current-user',
      [system.config?.github?.username]
    );

    // Check if there's any content that would be lost
    const hasUnsavedChanges = useCallback(() => {
      if (!isEditMode) {
        // Create mode - warn when there's content
        return title.trim() !== '' || description.trim() !== '';
      } else if (task) {
        // Edit mode - warn when current values differ from original task
        const normalizeList = (value: string) =>
          value
            .split(',')
            .map((item) => item.trim())
            .filter(Boolean)
            .join(',');
        const normalizeJsonList = (value: string | null) => {
          if (!value) return '';
          try {
            const parsed = JSON.parse(value) as string[];
            return parsed.map((item) => item.trim()).filter(Boolean).join(',');
          } catch (error) {
            console.error('Failed to parse list for comparison', error);
            return '';
          }
        };

        const titleChanged = title.trim() !== task.title.trim();
        const descriptionChanged =
          (description || '').trim() !== (task.description || '').trim();
        const statusChanged = status !== task.status;
        const priorityChanged = priority !== task.priority;
        const assigneeChanged = (assigneeId || '').trim() !== (task.assignee_id || '');
        const agentChanged = (assignedAgent || '').trim() !== (task.assigned_agent || '');
        const mcpsChanged =
          normalizeList(assignedMcpsInput) !== normalizeJsonList(task.assigned_mcps);
        const tagsChanged =
          normalizeList(tagsInput) !== normalizeJsonList(task.tags);
        const approvalChanged = requiresApproval !== task.requires_approval;
        const dueDateChanged =
          (dueDate ? dueDate : '') !== (task.due_date ? task.due_date.slice(0, 10) : '');
        const boardChanged = (task.board_id || null) !== (selectedBoardId || null);

        return (
          titleChanged ||
          descriptionChanged ||
          statusChanged ||
          priorityChanged ||
          assigneeChanged ||
          agentChanged ||
          mcpsChanged ||
          tagsChanged ||
          approvalChanged ||
          dueDateChanged ||
          boardChanged
        );
      }
      return false;
    }, [
      title,
      description,
      status,
      priority,
      assigneeId,
      assignedAgent,
      assignedMcpsInput,
      tagsInput,
      requiresApproval,
      dueDate,
      selectedBoardId,
      isEditMode,
      task,
    ]);

    // Warn on browser/tab close if there are unsaved changes
    useEffect(() => {
      if (!modal.visible) return; // dialog closed → nothing to do

      // always re-evaluate latest fields via hasUnsavedChanges()
      const handleBeforeUnload = (e: BeforeUnloadEvent) => {
        if (hasUnsavedChanges()) {
          e.preventDefault();
          // Chrome / Edge still require returnValue to be set
          e.returnValue = '';
          return '';
        }
        // nothing returned → no prompt
      };

      window.addEventListener('beforeunload', handleBeforeUnload);
      return () =>
        window.removeEventListener('beforeunload', handleBeforeUnload);
    }, [modal.visible, hasUnsavedChanges]); // hasUnsavedChanges is memoised with title/descr deps

    useEffect(() => {
      if (!projectId || !modal.visible) {
        setBoards([]);
        setBoardsError(null);
        setBoardsLoading(false);
        return;
      }

      let cancelled = false;
      const loadBoards = async () => {
        setBoardsLoading(true);
        setBoardsError(null);
        try {
          const results = await projectsApi.listBoards(projectId);
          if (!cancelled) {
            setBoards(results);
          }
        } catch (error) {
          console.error('Failed to load boards', error);
          if (!cancelled) {
            setBoardsError(
              error instanceof Error ? error.message : 'Failed to load boards'
            );
          }
        } finally {
          if (!cancelled) {
            setBoardsLoading(false);
          }
        }
      };

      loadBoards();

      return () => {
        cancelled = true;
      };
    }, [projectId, modal.visible]);

    useEffect(() => {
      if (!selectedBoardId) return;
      if (!boards.length) return;
      const exists = boards.some((board) => board.id === selectedBoardId);
      if (!exists) {
        setSelectedBoardId(null);
      }
    }, [boards, selectedBoardId]);

    useEffect(() => {
      if (isEditMode) return;
      if (!modal.visible) return;
      if (boardsLoading) return;
      if (!boards.length) return;

      if (selectedBoardId && boards.some((board) => board.id === selectedBoardId)) {
        return;
      }

      // Prefer the default board, fallback to first available
      const preferred =
        boards.find((board) => board.board_type === 'default') || boards[0];
      if (preferred) {
        setSelectedBoardId(preferred.id);
      }
    }, [boards, boardsLoading, isEditMode, selectedBoardId, modal.visible]);

    useEffect(() => {
      if (task) {
        // Edit mode - populate with existing task data
        setTitle(task.title);
        setDescription(task.description || '');
        setStatus(task.status);
        setPriority(task.priority);
        setAssigneeId(task.assignee_id || '');
        setAssignedAgent(task.assigned_agent || '');
        setAssignedMcpsInput(() => {
          if (!task.assigned_mcps) return '';
          try {
            const parsed = JSON.parse(task.assigned_mcps) as string[];
            return parsed.join(', ');
          } catch (error) {
            console.error('Failed to parse assigned MCPs', error);
            return '';
          }
        });
        setTagsInput(() => {
          if (!task.tags) return '';
          try {
            const parsed = JSON.parse(task.tags) as string[];
            return parsed.join(', ');
          } catch (error) {
            console.error('Failed to parse tags', error);
            return '';
          }
        });
        setRequiresApproval(task.requires_approval);
        setDueDate(task.due_date ? task.due_date.slice(0, 10) : '');
        setSelectedBoardId(task.board_id || null);

        // Load existing images for the task
        if (modal.visible) {
          imagesApi
            .getTaskImages(task.id)
            .then((taskImages) => setImages(taskImages))
            .catch((err) => {
              console.error('Failed to load task images:', err);
              setImages([]);
            });
        }
      } else if (initialTask) {
        // Duplicate mode - pre-fill from existing task but reset status to 'todo' and no images
        setTitle(initialTask.title);
        setDescription(initialTask.description || '');
        setStatus('todo'); // Always start duplicated tasks as 'todo'
        setPriority(initialTask.priority);
        setAssigneeId(initialTask.assignee_id || '');
        setAssignedAgent(initialTask.assigned_agent || '');
        setAssignedMcpsInput(() => {
          if (!initialTask.assigned_mcps) return '';
          try {
            const parsed = JSON.parse(initialTask.assigned_mcps) as string[];
            return parsed.join(', ');
          } catch (error) {
            console.error('Failed to parse assigned MCPs', error);
            return '';
          }
        });
        setTagsInput(() => {
          if (!initialTask.tags) return '';
          try {
            const parsed = JSON.parse(initialTask.tags) as string[];
            return parsed.join(', ');
          } catch (error) {
            console.error('Failed to parse tags', error);
            return '';
          }
        });
        setRequiresApproval(initialTask.requires_approval);
        setDueDate(initialTask.due_date ? initialTask.due_date.slice(0, 10) : '');
        setSelectedTemplate('');
        setImages([]);
        setNewlyUploadedImageIds([]);
        setSelectedBoardId(initialTask.board_id || null);
      } else if (initialTemplate) {
        // Create mode with template - pre-fill from template
        setTitle(initialTemplate.title);
        setDescription(initialTemplate.description || '');
        setStatus('todo');
        setPriority('medium');
        setAssigneeId('');
        setAssignedAgent('');
        setAssignedMcpsInput('');
        setTagsInput('');
        setRequiresApproval(false);
        setDueDate('');
        setSelectedTemplate('');
        setSelectedBoardId(null);
      } else {
        // Create mode - reset to defaults
        setTitle('');
        setDescription('');
        setStatus('todo');
        setPriority('medium');
        setAssigneeId('');
        setAssignedAgent('');
        setAssignedMcpsInput('');
        setTagsInput('');
        setRequiresApproval(false);
        setDueDate('');
        setSelectedTemplate('');
        setImages([]);
        setNewlyUploadedImageIds([]);
        setSelectedBranch('');
        setSelectedExecutorProfile(system.config?.executor_profile || null);
        setQuickstartExpanded(false);
        setSelectedBoardId(null);
      }
    }, [
      task,
      initialTask,
      initialTemplate,
      modal.visible,
      system.config?.executor_profile,
    ]);

    // Fetch templates and branches when dialog opens in create mode
    useEffect(() => {
      if (modal.visible && !isEditMode && projectId) {
        // Fetch templates and branches
        Promise.all([
          templatesApi.listByProject(projectId),
          templatesApi.listGlobal(),
          projectsApi.getBranches(projectId),
        ])
          .then(([projectTemplates, globalTemplates, projectBranches]) => {
            // Combine templates with project templates first
            setTemplates([...projectTemplates, ...globalTemplates]);

            // Set branches and default to initialBaseBranch if provided, otherwise current branch
            setBranches(projectBranches);

            if (
              initialBaseBranch &&
              projectBranches.some((b) => b.name === initialBaseBranch)
            ) {
              // Use initialBaseBranch if it exists in the project branches (for spinoff)
              setSelectedBranch(initialBaseBranch);
            } else {
              // Default behavior: use current branch or first available
              const currentBranch = projectBranches.find((b) => b.is_current);
              const defaultBranch = currentBranch || projectBranches[0];
              if (defaultBranch) {
                setSelectedBranch(defaultBranch.name);
              }
            }
          })
          .catch(console.error);
      }
    }, [modal.visible, isEditMode, projectId, initialBaseBranch]);

    // Fetch parent base branch when parentTaskAttemptId is provided
    useEffect(() => {
      if (
        modal.visible &&
        !isEditMode &&
        parentTaskAttemptId &&
        !initialBaseBranch &&
        branches.length > 0
      ) {
        attemptsApi
          .get(parentTaskAttemptId)
          .then((attempt) => {
            const parentBranch = attempt.branch || attempt.base_branch;
            if (parentBranch && branches.some((b) => b.name === parentBranch)) {
              setSelectedBranch(parentBranch);
            }
          })
          .catch(() => {
            // Silently fail, will use current branch fallback
          });
      }
    }, [
      modal.visible,
      isEditMode,
      parentTaskAttemptId,
      initialBaseBranch,
      branches,
    ]);

    // Set default executor from config (following TaskDetailsToolbar pattern)
    useEffect(() => {
      if (system.config?.executor_profile) {
        setSelectedExecutorProfile(system.config.executor_profile);
      }
    }, [system.config?.executor_profile]);

    // Set default executor from config (following TaskDetailsToolbar pattern)
    useEffect(() => {
      if (system.config?.executor_profile) {
        setSelectedExecutorProfile(system.config.executor_profile);
      }
    }, [system.config?.executor_profile]);

    // Handle template selection
    const handleTemplateChange = (templateId: string) => {
      setSelectedTemplate(templateId);
      if (templateId === 'none') {
        // Clear the form when "No template" is selected
        setTitle('');
        setDescription('');
      } else if (templateId) {
        const template = templates.find((t) => t.id === templateId);
        if (template) {
          setTitle(template.title);
          setDescription(template.description || '');
        }
      }
    };

    // Handle image upload success by inserting markdown into description
    const handleImageUploaded = useCallback((image: ImageResponse) => {
      const markdownText = `![${image.original_name}](${image.file_path})`;
      setDescription((prev) => {
        if (prev.trim() === '') {
          return markdownText;
        } else {
          return prev + ' ' + markdownText;
        }
      });

      setImages((prev) => [...prev, image]);
      // Track as newly uploaded for backend association
      setNewlyUploadedImageIds((prev) => [...prev, image.id]);
    }, []);

    const handleImagesChange = useCallback((updatedImages: ImageResponse[]) => {
      setImages(updatedImages);
      // Also update newlyUploadedImageIds to remove any deleted image IDs
      setNewlyUploadedImageIds((prev) =>
        prev.filter((id) => updatedImages.some((img) => img.id === id))
      );
    }, []);

    const handleSubmit = useCallback(async () => {
      if (!title.trim() || !projectId) return;

      setIsSubmitting(true);
      try {
        let imageIds: string[] | undefined;

        if (isEditMode) {
          // In edit mode, send all current image IDs (existing + newly uploaded)
          imageIds =
            images.length > 0 ? images.map((img) => img.id) : undefined;
        } else {
          // In create mode, only send newly uploaded image IDs
          imageIds =
            newlyUploadedImageIds.length > 0
              ? newlyUploadedImageIds
              : undefined;
        }

        const assignedMcps = assignedMcpsInput
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean);
        const tags = tagsInput
          .split(',')
          .map((item) => item.trim())
          .filter(Boolean);
        const dueDateIso = dueDate ? new Date(dueDate).toISOString() : null;

        if (isEditMode && task) {
          updateTask.mutate(
            {
              taskId: task.id,
              data: {
                title,
                description: description || null,
                status,
                parent_task_attempt: parentTaskAttemptId || null,
                image_ids: imageIds || null,
                board_id: selectedBoardId ?? null,
                priority,
                assignee_id: assigneeId.trim() || null,
                assigned_agent: assignedAgent.trim() || null,
                assigned_mcps: assignedMcps.length ? assignedMcps : null,
                requires_approval: requiresApproval,
                approval_status: task.approval_status,
                parent_task_id: task.parent_task_id,
                tags: tags.length ? tags : null,
                due_date: dueDateIso,
              },
            },
            {
              onSuccess: () => {
                modal.hide();
              },
            }
          );
        } else {
          const createdBy = createdByFallback;

          createTask.mutate(
            {
              project_id: projectId,
              title,
              description: description || null,
              parent_task_attempt: parentTaskAttemptId || null,
              image_ids: imageIds || null,
              board_id: selectedBoardId ?? undefined,
              priority,
              assignee_id: assigneeId.trim() || null,
              assigned_agent: assignedAgent.trim() || null,
              agent_id: null,  // Will be set via agent lookup in future
              assigned_mcps: assignedMcps.length ? assignedMcps : null,
              created_by: createdBy,
              requires_approval: requiresApproval,
              parent_task_id: null,
              tags: tags.length ? tags : null,
              due_date: dueDateIso,
              custom_properties: null,
              scheduled_start: null,
              scheduled_end: null,
            },
            {
              onSuccess: () => {
                modal.hide();
              },
            }
          );
        }
      } finally {
        setIsSubmitting(false);
      }
    }, [
      title,
      description,
      status,
      isEditMode,
      projectId,
      task,
      modal,
      newlyUploadedImageIds,
      images,
      createTask,
      updateTask,
      assignedMcpsInput,
      tagsInput,
      priority,
      assigneeId,
      assignedAgent,
      requiresApproval,
      dueDate,
      createdByFallback,
      selectedBoardId,
      parentTaskAttemptId,
    ]);

    const handleCreateAndStart = useCallback(async () => {
      if (!title.trim() || !projectId) return;

      setIsSubmittingAndStart(true);
      try {
        if (!isEditMode) {
          const imageIds =
            newlyUploadedImageIds.length > 0
              ? newlyUploadedImageIds
              : undefined;

          const assignedMcps = assignedMcpsInput
            .split(',')
            .map((item) => item.trim())
            .filter(Boolean);
          const tags = tagsInput
            .split(',')
            .map((item) => item.trim())
            .filter(Boolean);
          const dueDateIso = dueDate ? new Date(dueDate).toISOString() : null;
          const createdBy = createdByFallback;

          // Use selected executor profile or fallback to config default
          const finalExecutorProfile =
            selectedExecutorProfile || system.config?.executor_profile;
          if (!finalExecutorProfile || !selectedBranch) {
            console.warn(
              `Missing ${!finalExecutorProfile ? 'executor profile' : 'branch'} for Create & Start`
            );
            return;
          }

          createAndStart.mutate(
            {
              task: {
                project_id: projectId,
                title,
                description: description || null,
                parent_task_attempt: parentTaskAttemptId || null,
                image_ids: imageIds || null,
                board_id: selectedBoardId ?? undefined,
                priority,
                assignee_id: assigneeId.trim() || null,
                assigned_agent: assignedAgent.trim() || null,
                agent_id: null,
                assigned_mcps: assignedMcps.length ? assignedMcps : null,
                created_by: createdBy,
                requires_approval: requiresApproval,
                parent_task_id: null,
                tags: tags.length ? tags : null,
                due_date: dueDateIso,
                custom_properties: null,
                scheduled_start: null,
                scheduled_end: null,
              },
              executor_profile_id: finalExecutorProfile,
              base_branch: selectedBranch,
            },
            {
              onSuccess: () => {
                modal.hide();
              },
            }
          );
        }
      } finally {
        setIsSubmittingAndStart(false);
      }
    }, [
      title,
      description,
      isEditMode,
      projectId,
      modal,
      newlyUploadedImageIds,
      createAndStart,
      selectedExecutorProfile,
      selectedBranch,
      system.config?.executor_profile,
      assignedMcpsInput,
      tagsInput,
      priority,
      assigneeId,
      assignedAgent,
      requiresApproval,
      dueDate,
      createdByFallback,
      selectedBoardId,
      parentTaskAttemptId,
    ]);

    const handleCancel = useCallback(() => {
      // Check for unsaved changes before closing
      if (hasUnsavedChanges()) {
        setShowDiscardWarning(true);
      } else {
        modal.hide();
      }
    }, [modal, hasUnsavedChanges]);

    const handleDiscardChanges = useCallback(() => {
      // Close both dialogs
      setShowDiscardWarning(false);
      modal.hide();
    }, [modal]);

    // Handle keyboard shortcuts

    // Handle dialog close attempt
    const handleDialogOpenChange = (open: boolean) => {
      if (!open && hasUnsavedChanges()) {
        // Trying to close with unsaved changes
        setShowDiscardWarning(true);
      } else if (!open) {
        modal.hide();
      }
    };

    return (
      <>
        <Dialog open={modal.visible} onOpenChange={handleDialogOpenChange}>
          <DialogContent className="sm:max-w-[550px]">
            <DialogHeader>
              <DialogTitle>
                {isEditMode ? 'Edit Task' : 'Create New Task'}
              </DialogTitle>
            </DialogHeader>
            <div className="space-y-4">
              <div>
                <Label htmlFor="task-title" className="text-sm font-medium">
                  Title
                </Label>
                <Input
                  id="task-title"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="What needs to be done?"
                  className="mt-1.5"
                  disabled={isSubmitting || isSubmittingAndStart}
                  autoFocus
                  onCommandEnter={
                    isEditMode ? handleSubmit : handleCreateAndStart
                  }
                  onCommandShiftEnter={handleSubmit}
                />
              </div>

              <div>
                <Label
                  htmlFor="task-description"
                  className="text-sm font-medium"
                >
                  Description
                </Label>
                <RichTextEditor
                  value={description}
                  onChange={(value) => setDescription(value)}
                  placeholder="Add more details (optional). Supports Markdown formatting."
                  height={250}
                  enableToolbar={true}
                  className="mt-1.5"
                  readOnly={isSubmitting || isSubmittingAndStart}
                />
              </div>

              <div className="grid gap-4 md:grid-cols-4">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Priority</Label>
                  <Select
                    value={priority}
                    onValueChange={(value) => setPriority(value as Priority)}
                    disabled={isSubmitting || isSubmittingAndStart}
                  >
                    <SelectTrigger>
                      <SelectValue placeholder="Select priority" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="critical">Critical</SelectItem>
                      <SelectItem value="high">High</SelectItem>
                      <SelectItem value="medium">Medium</SelectItem>
                      <SelectItem value="low">Low</SelectItem>
                    </SelectContent>
                  </Select>
                </div>

                <div className="space-y-2">
                  <Label className="text-sm font-medium">Board</Label>
                  <Select
                    value={selectedBoardId ?? 'none'}
                    onValueChange={(value) =>
                      setSelectedBoardId(value === 'none' ? null : value)
                    }
                    disabled={
                      isSubmitting || isSubmittingAndStart || boardsLoading
                    }
                  >
                    <SelectTrigger>
                      <SelectValue
                        placeholder={
                          boardsLoading
                            ? 'Loading boards…'
                            : boards.length
                            ? 'Select board'
                            : 'No boards yet'
                        }
                      />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="none">No board</SelectItem>
                      {boards.map((board) => (
                        <SelectItem key={board.id} value={board.id}>
                          {board.name}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  {boardsError && (
                    <p className="text-xs text-destructive">{boardsError}</p>
                  )}
                  {!boardsLoading && boards.length === 0 && !boardsError && (
                    <p className="text-xs text-muted-foreground">
                      Boards live in the project overview. New projects get Brand, Dev, and
                      Social buckets automatically.
                    </p>
                  )}
                </div>

                <div className="space-y-2">
                  <Label className="text-sm font-medium">Due Date</Label>
                  <Input
                    type="date"
                    value={dueDate}
                    onChange={(e) => setDueDate(e.target.value)}
                    disabled={isSubmitting || isSubmittingAndStart}
                  />
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Assignee ID</Label>
                  <Input
                    value={assigneeId}
                    onChange={(e) => setAssigneeId(e.target.value)}
                    placeholder="e.g. user@example"
                    disabled={isSubmitting || isSubmittingAndStart}
                  />
                </div>

                <div className="space-y-2">
                  <Label className="text-sm font-medium">Assigned Agent</Label>
                  <Input
                    value={assignedAgent}
                    onChange={(e) => setAssignedAgent(e.target.value)}
                    placeholder="e.g. claude"
                    disabled={isSubmitting || isSubmittingAndStart}
                  />
                </div>
              </div>

              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label className="text-sm font-medium">Assigned MCPs (comma-separated)</Label>
                  <Textarea
                    value={assignedMcpsInput}
                    onChange={(e) => setAssignedMcpsInput(e.target.value)}
                    rows={2}
                    disabled={isSubmitting || isSubmittingAndStart}
                  />
                </div>

                <div className="space-y-2">
                  <Label className="text-sm font-medium">Tags (comma-separated)</Label>
                  <Textarea
                    value={tagsInput}
                    onChange={(e) => setTagsInput(e.target.value)}
                    rows={2}
                    disabled={isSubmitting || isSubmittingAndStart}
                  />
                </div>
              </div>

              <div className="flex items-center gap-3 pt-2">
                <Switch
                  id="requires-approval"
                  checked={requiresApproval}
                  onCheckedChange={(checked) => setRequiresApproval(checked)}
                  disabled={isSubmitting || isSubmittingAndStart}
                />
                <Label htmlFor="requires-approval" className="text-sm">
                  Requires approval before completion
                </Label>
              </div>

              <ImageUploadSection
                images={images}
                onImagesChange={handleImagesChange}
                onUpload={imagesApi.upload}
                onDelete={imagesApi.delete}
                onImageUploaded={handleImageUploaded}
                disabled={isSubmitting || isSubmittingAndStart}
                readOnly={isEditMode}
                collapsible={true}
                defaultExpanded={false}
              />

              {!isEditMode && templates.length > 0 && (
                <div className="pt-2">
                  <details className="group">
                    <summary className="cursor-pointer text-sm text-muted-foreground hover:text-foreground transition-colors list-none flex items-center gap-2">
                      <svg
                        className="h-3 w-3 transition-transform group-open:rotate-90"
                        viewBox="0 0 20 20"
                        fill="currentColor"
                      >
                        <path
                          fillRule="evenodd"
                          d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z"
                          clipRule="evenodd"
                        />
                      </svg>
                      Use a template
                    </summary>
                    <div className="mt-3 space-y-2">
                      <p className="text-xs text-muted-foreground">
                        Templates help you quickly create tasks with predefined
                        content.
                      </p>
                      <Select
                        value={selectedTemplate}
                        onValueChange={handleTemplateChange}
                      >
                        <SelectTrigger id="task-template" className="w-full">
                          <SelectValue placeholder="Choose a template to prefill this form" />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="none">No template</SelectItem>
                          {templates.map((template) => (
                            <SelectItem key={template.id} value={template.id}>
                              <div className="flex items-center gap-2">
                                {template.project_id === null && (
                                  <Globe2 className="h-3 w-3 text-muted-foreground" />
                                )}
                                <span>{template.template_name}</span>
                              </div>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                  </details>
                </div>
              )}

              {isEditMode && (
                <div className="pt-2">
                  <Label htmlFor="task-status" className="text-sm font-medium">
                    Status
                  </Label>
                  <Select
                    value={status}
                    onValueChange={(value) => setStatus(value as TaskStatus)}
                    disabled={isSubmitting || isSubmittingAndStart}
                  >
                    <SelectTrigger className="mt-1.5">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="todo">To Do</SelectItem>
                      <SelectItem value="inprogress">In Progress</SelectItem>
                      <SelectItem value="inreview">In Review</SelectItem>
                      <SelectItem value="done">Done</SelectItem>
                      <SelectItem value="cancelled">Cancelled</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              )}

              {!isEditMode &&
                (() => {
                  const quickstartSection = (
                    <div className="pt-2">
                      <details
                        className="group"
                        open={quickstartExpanded}
                        onToggle={(e) =>
                          setQuickstartExpanded(
                            (e.target as HTMLDetailsElement).open
                          )
                        }
                      >
                        <summary className="cursor-pointer text-sm text-muted-foreground hover:text-foreground transition-colors list-none flex items-center gap-2">
                          <ChevronRight className="h-3 w-3 transition-transform group-open:rotate-90" />
                          <Settings2 className="h-3 w-3" />
                          Quickstart
                        </summary>
                        <div className="mt-3 space-y-3">
                          <p className="text-xs text-muted-foreground">
                            Configuration for "Create & Start" workflow
                          </p>

                          {/* Executor Profile Selector */}
                          {profiles && selectedExecutorProfile && (
                            <ExecutorProfileSelector
                              profiles={profiles}
                              selectedProfile={selectedExecutorProfile}
                              onProfileSelect={setSelectedExecutorProfile}
                              disabled={isSubmitting || isSubmittingAndStart}
                            />
                          )}

                          {/* Branch Selector */}
                          {branches.length > 0 && (
                            <div>
                              <Label
                                htmlFor="base-branch"
                                className="text-sm font-medium"
                              >
                                Branch
                              </Label>
                              <div className="mt-1.5">
                                <BranchSelector
                                  branches={branches}
                                  selectedBranch={selectedBranch}
                                  onBranchSelect={setSelectedBranch}
                                  placeholder="Select branch"
                                  className={
                                    isSubmitting || isSubmittingAndStart
                                      ? 'opacity-50 cursor-not-allowed'
                                      : ''
                                  }
                                />
                              </div>
                            </div>
                          )}
                        </div>
                      </details>
                    </div>
                  );
                  return quickstartSection;
                })()}

              <div className="flex flex-col-reverse sm:flex-row sm:justify-end gap-2 pt-2">
                <Button
                  variant="outline"
                  onClick={handleCancel}
                  disabled={isSubmitting || isSubmittingAndStart}
                >
                  Cancel
                </Button>
                {isEditMode ? (
                  <Button
                    onClick={handleSubmit}
                    disabled={isSubmitting || !title.trim()}
                  >
                    {isSubmitting ? 'Updating...' : 'Update Task'}
                  </Button>
                ) : (
                  <>
                    <Button
                      variant="outline"
                      onClick={handleSubmit}
                      disabled={
                        isSubmitting || isSubmittingAndStart || !title.trim()
                      }
                    >
                      {isSubmitting ? 'Creating...' : 'Create Task'}
                    </Button>
                    <Button
                      onClick={handleCreateAndStart}
                      disabled={
                        isSubmitting || isSubmittingAndStart || !title.trim()
                      }
                      className={'font-medium'}
                    >
                      {isSubmittingAndStart
                        ? 'Creating & Starting...'
                        : 'Create & Start'}
                    </Button>
                  </>
                )}
              </div>
            </div>
          </DialogContent>
        </Dialog>

        {/* Discard Warning Dialog */}
        <Dialog open={showDiscardWarning} onOpenChange={setShowDiscardWarning}>
          <DialogContent className="sm:max-w-[425px]">
            <DialogHeader>
              <DialogTitle>Discard unsaved changes?</DialogTitle>
            </DialogHeader>
            <div className="py-4">
              <p className="text-sm text-muted-foreground">
                You have unsaved changes. Are you sure you want to discard them?
              </p>
            </div>
            <div className="flex justify-end gap-2">
              <Button
                variant="outline"
                onClick={() => setShowDiscardWarning(false)}
              >
                Continue Editing
              </Button>
              <Button variant="destructive" onClick={handleDiscardChanges}>
                Discard Changes
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      </>
    );
  }
);
