import { useState, useEffect, useCallback, useMemo } from 'react';
import { imagesApi } from '@/lib/api';
import { useTaskMutations } from '@/hooks/useTaskMutations';
import { useUserSystem } from '@/components/config-provider';
import type {
  TaskStatus,
  ImageResponse,
  ExecutorProfileId,
  Priority,
} from 'shared/types';
import type { useModal } from '@ebay/nice-modal-react';
import type { TaskFormDialogProps, TaskFormState } from './types';
import { buildCreateTaskPayload } from './buildTaskPayload';
import { useBoardLoader } from './useBoardLoader';
import { useBranchLoader } from './useBranchLoader';

export function useTaskFormState({
  props,
  modal,
}: {
  props: TaskFormDialogProps;
  modal: ReturnType<typeof useModal>;
}): TaskFormState {
  const {
    task,
    projectId,
    initialTemplate,
    initialTask,
    initialBaseBranch,
    parentTaskAttemptId,
    initialBoardId,
    initialStatus,
  } = props;

  // Debug: Log initialBoardId when component renders
  console.log('[TaskFormDialog] Props received:', { initialBoardId, projectId, isEditMode: Boolean(task) });

  const { createTask, createAndStart, updateTask } =
    useTaskMutations(projectId);
  const { system } = useUserSystem();
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [status, setStatus] = useState<TaskStatus>(initialStatus || 'todo');
  const [priority, setPriority] = useState<Priority>('medium');
  const [assigneeId, setAssigneeId] = useState('');
  const [assignedAgent, setAssignedAgent] = useState('');
  const [assignedMcpsInput, setAssignedMcpsInput] = useState('');
  const [tagsInput, setTagsInput] = useState('');
  const [requiresApproval, setRequiresApproval] = useState(false);
  const [dueDate, setDueDate] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isSubmittingAndStart, setIsSubmittingAndStart] = useState(false);
  const [selectedTemplate, setSelectedTemplate] = useState<string>('');
  const [showDiscardWarning, setShowDiscardWarning] = useState(false);
  const [images, setImages] = useState<ImageResponse[]>([]);
  const [newlyUploadedImageIds, setNewlyUploadedImageIds] = useState<
    string[]
  >([]);
  const [selectedExecutorProfile, setSelectedExecutorProfile] =
    useState<ExecutorProfileId | null>(null);
  const [quickstartExpanded, setQuickstartExpanded] =
    useState<boolean>(false);
  const [completionCriteria, setCompletionCriteria] = useState('');
  const [outputFormat, setOutputFormat] = useState('');
  const [simpleMode, setSimpleMode] = useState<boolean>(
    () => localStorage.getItem('pcg-task-simple-mode') === 'true'
  );

  const isEditMode = Boolean(task);

  const createdByFallback = useMemo(
    () => system.config?.github?.username || 'current-user',
    [system.config?.github?.username]
  );

  // Board loading via extracted hook
  const {
    boards,
    boardsLoading,
    boardsError,
    selectedBoardId,
    setSelectedBoardId,
  } = useBoardLoader({
    projectId,
    isEditMode,
    modalVisible: modal.visible,
    initialBoardId,
  });

  // Branch/template loading via extracted hook
  const {
    templates,
    branches,
    selectedBranch,
    setSelectedBranch,
  } = useBranchLoader({
    projectId,
    isEditMode,
    modalVisible: modal.visible,
    initialBaseBranch,
    parentTaskAttemptId,
  });

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
    if (!modal.visible) return; // dialog closed -> nothing to do

    // always re-evaluate latest fields via hasUnsavedChanges()
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (hasUnsavedChanges()) {
        e.preventDefault();
        // Chrome / Edge still require returnValue to be set
        e.returnValue = '';
        return '';
      }
      // nothing returned -> no prompt
    };

    window.addEventListener('beforeunload', handleBeforeUnload);
    return () =>
      window.removeEventListener('beforeunload', handleBeforeUnload);
  }, [modal.visible, hasUnsavedChanges]); // hasUnsavedChanges is memoised with title/descr deps

  useEffect(() => {
    // Only run form reset when modal is visible
    if (!modal.visible) return;

    console.log('[TaskFormDialog] Form reset effect running:', {
      task: !!task,
      initialTask: !!initialTask,
      initialTemplate: !!initialTemplate,
      initialBoardId
    });

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
      setCompletionCriteria(task.completion_criteria || '');
      setOutputFormat(task.output_format || '');
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
      setCompletionCriteria(initialTask.completion_criteria || '');
      setOutputFormat(initialTask.output_format || '');
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
      setCompletionCriteria('');
      setOutputFormat('');
      setDueDate('');
      setSelectedTemplate('');
      setSelectedBoardId(initialBoardId ?? null);
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
      setCompletionCriteria('');
      setOutputFormat('');
      setDueDate('');
      setSelectedTemplate('');
      setImages([]);
      setNewlyUploadedImageIds([]);
      setSelectedBranch('');
      setSelectedExecutorProfile(system.config?.executor_profile || null);
      setQuickstartExpanded(false);
      setSelectedBoardId(initialBoardId ?? null);
    }
  }, [
    task,
    initialTask,
    initialTemplate,
    initialBoardId,
    modal.visible,
    system.config?.executor_profile,
    setSelectedBoardId,
    setSelectedBranch,
  ]);

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
  const handleImageUploaded = useCallback((imageId: string) => {
    // This callback receives just the image ID from QuickstartSection
    // The actual image object handling is done via handleImagesChange
    setNewlyUploadedImageIds((prev) => [...prev, imageId]);
  }, []);

  const handleImagesChange = useCallback((updatedImages: ImageResponse[]) => {
    setImages(updatedImages);
    // Also update newlyUploadedImageIds to remove any deleted image IDs
    setNewlyUploadedImageIds((prev) =>
      prev.filter((id) => updatedImages.some((img) => img.id === id))
    );
  }, []);

  const handleSubmit = useCallback(async () => {
    if (!title.trim() || !projectId) {
      return;
    }

    setIsSubmitting(true);

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

    // Close modal FIRST before any async operations
    modal.hide();
    setIsSubmitting(false);

    if (isEditMode && task) {
      const assignedMcps = assignedMcpsInput
        .split(',')
        .map((item) => item.trim())
        .filter(Boolean);
      const tags = tagsInput
        .split(',')
        .map((item) => item.trim())
        .filter(Boolean);
      const dueDateIso = dueDate ? new Date(dueDate).toISOString() : null;

      updateTask.mutate({
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
          completion_criteria: completionCriteria || null,
          output_format: outputFormat || null,
        },
      });
    } else {
      const payload = buildCreateTaskPayload({
        title,
        description,
        projectId,
        priority,
        assigneeId,
        assignedAgent,
        assignedMcpsInput,
        tagsInput,
        requiresApproval,
        dueDate,
        completionCriteria,
        outputFormat,
        selectedBoardId,
        parentTaskAttemptId,
        createdBy: createdByFallback,
        imageIds,
      });
      createTask.mutate(payload);
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
    completionCriteria,
    outputFormat,
    dueDate,
    createdByFallback,
    selectedBoardId,
    parentTaskAttemptId,
  ]);

  const handleCreateAndStart = useCallback(async () => {
    if (!title.trim() || !projectId) {
      return;
    }

    if (isEditMode) {
      return;
    }

    setIsSubmittingAndStart(true);

    const imageIds =
      newlyUploadedImageIds.length > 0
        ? newlyUploadedImageIds
        : undefined;

    // Use selected executor profile or fallback to config default
    const finalExecutorProfile =
      selectedExecutorProfile || system.config?.executor_profile;
    if (!finalExecutorProfile || !selectedBranch) {
      console.warn('[TaskFormDialog] Missing executor profile or branch for Create & Start');
      setIsSubmittingAndStart(false);
      return;
    }

    // Close modal FIRST before mutation
    modal.hide();
    setIsSubmittingAndStart(false);

    const payload = buildCreateTaskPayload({
      title,
      description,
      projectId,
      priority,
      assigneeId,
      assignedAgent,
      assignedMcpsInput,
      tagsInput,
      requiresApproval,
      dueDate,
      completionCriteria,
      outputFormat,
      selectedBoardId,
      parentTaskAttemptId,
      createdBy: createdByFallback,
      imageIds,
    });

    createAndStart.mutate({
      task: payload,
      executor_profile_id: finalExecutorProfile,
      base_branch: selectedBranch,
    });
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
    completionCriteria,
    outputFormat,
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

  return {
    title,
    setTitle,
    description,
    setDescription,
    status,
    setStatus,
    priority,
    setPriority,
    assigneeId,
    setAssigneeId,
    assignedAgent,
    setAssignedAgent,
    assignedMcpsInput,
    setAssignedMcpsInput,
    tagsInput,
    setTagsInput,
    requiresApproval,
    setRequiresApproval,
    dueDate,
    setDueDate,
    completionCriteria,
    setCompletionCriteria,
    outputFormat,
    setOutputFormat,
    isSubmitting,
    isSubmittingAndStart,
    templates,
    selectedTemplate,
    images,
    branches,
    selectedBranch,
    setSelectedBranch,
    selectedExecutorProfile,
    setSelectedExecutorProfile,
    boards,
    boardsLoading,
    boardsError,
    selectedBoardId,
    setSelectedBoardId,
    quickstartExpanded,
    setQuickstartExpanded,
    simpleMode,
    setSimpleMode,
    showDiscardWarning,
    setShowDiscardWarning,
    isEditMode,
    handleSubmit,
    handleCreateAndStart,
    handleCancel,
    handleDiscardChanges,
    handleDialogOpenChange,
    handleTemplateChange,
    handleImagesChange,
    handleImageUploaded,
  };
}
