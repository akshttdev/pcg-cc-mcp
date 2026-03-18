import type { CreateTask, Priority } from 'shared/types';

export interface TaskPayloadInputs {
  title: string;
  description: string;
  projectId: string;
  priority: Priority;
  assigneeId: string;
  assignedAgent: string;
  assignedMcpsInput: string;
  tagsInput: string;
  requiresApproval: boolean;
  dueDate: string;
  completionCriteria: string;
  outputFormat: string;
  selectedBoardId: string | null;
  parentTaskAttemptId?: string;
  createdBy: string;
  imageIds?: string[];
}

export function buildCreateTaskPayload(inputs: TaskPayloadInputs): CreateTask {
  const assignedMcps = inputs.assignedMcpsInput
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
  const tags = inputs.tagsInput
    .split(',')
    .map((item) => item.trim())
    .filter(Boolean);
  const dueDateIso = inputs.dueDate
    ? new Date(inputs.dueDate).toISOString()
    : null;

  return {
    project_id: inputs.projectId,
    title: inputs.title,
    description: inputs.description || null,
    parent_task_attempt: inputs.parentTaskAttemptId || null,
    image_ids: inputs.imageIds || null,
    board_id: inputs.selectedBoardId ?? undefined,
    priority: inputs.priority,
    assignee_id: inputs.assigneeId.trim() || null,
    assigned_agent: inputs.assignedAgent.trim() || null,
    agent_id: null,
    assigned_mcps: assignedMcps.length ? assignedMcps : null,
    created_by: inputs.createdBy,
    requires_approval: inputs.requiresApproval,
    parent_task_id: null,
    tags: tags.length ? tags : null,
    due_date: dueDateIso,
    assignee_type: null,
    screenshot: null,
    custom_properties: null,
    scheduled_start: null,
    scheduled_end: null,
    completion_criteria: inputs.completionCriteria || null,
    output_format: inputs.outputFormat || null,
    collaborators: null,
  };
}
