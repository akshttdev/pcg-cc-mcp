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

export interface TaskFormDialogProps {
  task?: Task | null;
  projectId?: string;
  initialTemplate?: TaskTemplate | null;
  initialTask?: Task | null;
  initialBaseBranch?: string;
  parentTaskAttemptId?: string;
  initialBoardId?: string | null;
  initialStatus?: TaskStatus;
}

export interface TaskFormState {
  title: string;
  setTitle: (v: string) => void;
  description: string;
  setDescription: (v: string) => void;
  status: TaskStatus;
  setStatus: (v: TaskStatus) => void;
  priority: Priority;
  setPriority: (v: Priority) => void;
  assigneeId: string;
  setAssigneeId: (v: string) => void;
  assignedAgent: string;
  setAssignedAgent: (v: string) => void;
  assignedMcpsInput: string;
  setAssignedMcpsInput: (v: string) => void;
  tagsInput: string;
  setTagsInput: (v: string) => void;
  requiresApproval: boolean;
  setRequiresApproval: (v: boolean) => void;
  dueDate: string;
  setDueDate: (v: string) => void;
  completionCriteria: string;
  setCompletionCriteria: (v: string) => void;
  outputFormat: string;
  setOutputFormat: (v: string) => void;
  isSubmitting: boolean;
  isSubmittingAndStart: boolean;
  templates: TaskTemplate[];
  selectedTemplate: string;
  images: ImageResponse[];
  branches: GitBranch[];
  selectedBranch: string;
  setSelectedBranch: (v: string) => void;
  selectedExecutorProfile: ExecutorProfileId | null;
  setSelectedExecutorProfile: (v: ExecutorProfileId | null) => void;
  boards: ProjectBoard[];
  boardsLoading: boolean;
  boardsError: string | null;
  selectedBoardId: string | null;
  setSelectedBoardId: (v: string | null) => void;
  quickstartExpanded: boolean;
  setQuickstartExpanded: (v: boolean) => void;
  simpleMode: boolean;
  setSimpleMode: (v: boolean) => void;
  showDiscardWarning: boolean;
  setShowDiscardWarning: (v: boolean) => void;
  isEditMode: boolean;
  handleSubmit: () => void;
  handleCreateAndStart: () => void;
  handleCancel: () => void;
  handleDiscardChanges: () => void;
  handleDialogOpenChange: (open: boolean) => void;
  handleTemplateChange: (templateId: string) => void;
  handleImagesChange: (images: ImageResponse[]) => void;
  handleImageUploaded: (imageId: string) => void;
}
