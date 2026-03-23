// Bulk Operations Types

export interface BulkOperationResult {
  success: number;
  failed: number;
  total: number;
  errors: Array<{ taskId: string; error: string }>;
}

export type BulkActionType =
  | 'status_change'
  | 'tag_add'
  | 'tag_remove'
  | 'delete'
  | 'export';

export interface BulkAction {
  type: BulkActionType;
  taskIds: string[];
  params?: Record<string, unknown>; // Action-specific parameters
}