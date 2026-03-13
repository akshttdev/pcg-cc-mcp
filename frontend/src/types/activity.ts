export type ActivityType =
  | 'task_created'
  | 'task_updated'
  | 'task_deleted'
  | 'status_changed'
  | 'priority_changed'
  | 'assignee_changed'
  | 'comment_added'
  | 'dependency_added'
  | 'dependency_removed'
  | 'time_logged'
  | 'file_attached'
  | 'agent_tool_call'
  | 'agent_workflow_triggered'
  | 'agent_workflow_completed';

export interface ActivityEntry {
  id: string;
  taskId: string;
  type: ActivityType;
  userId?: string;
  userName?: string;
  description: string;
  metadata?: Record<string, any>;
  timestamp: string;
}

export interface ActivityFilter {
  types?: ActivityType[];
  taskIds?: string[];
  startDate?: Date;
  endDate?: Date;
}
