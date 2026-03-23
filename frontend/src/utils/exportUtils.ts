import type { TaskWithAttemptStatus } from 'shared/types';
import type { ExportFormat, TaskExportData } from '@/types/export';

/**
 * Converts tasks to CSV format
 */
export function tasksToCSV(tasks: TaskWithAttemptStatus[]): string {
  if (tasks.length === 0) {
    return '';
  }

  // CSV headers
  const headers = [
    'ID',
    'Title',
    'Description',
    'Status',
    'Priority',
    'Assignee',
    'Created At',
    'Updated At',
    'Parent Task ID',
  ];

  // Escape CSV values
  const escapeCSV = (value: unknown): string => {
    if (value === null || value === undefined) {
      return '';
    }
    const str = String(value);
    if (str.includes(',') || str.includes('"') || str.includes('\n')) {
      return `"${str.replace(/"/g, '""')}"`;
    }
    return str;
  };

  // Build CSV rows
  const rows = tasks.map((task) => [
    escapeCSV(task.id),
    escapeCSV(task.title),
    escapeCSV(task.description || ''),
    escapeCSV(task.status),
    escapeCSV(task.priority || ''),
    escapeCSV(task.assignee_id || ''),
    escapeCSV(task.created_at),
    escapeCSV(task.updated_at),
    escapeCSV(task.parent_task_id || ''),
  ]);

  // Combine headers and rows
  return [headers.join(','), ...rows.map((row) => row.join(','))].join('\n');
}

/**
 * Converts tasks to JSON format
 */
export function tasksToJSON(tasks: TaskWithAttemptStatus[]): string {
  const exportData: TaskExportData[] = tasks.map((task) => ({
    id: task.id,
    title: task.title,
    description: task.description ?? undefined,
    status: task.status,
    priority: task.priority ?? undefined,
    assignee: task.assignee_id ?? undefined,
    created_at: task.created_at,
    updated_at: task.updated_at,
    parent_task_id: task.parent_task_id ?? undefined,
  }));

  return JSON.stringify(exportData, null, 2);
}

/**
 * Downloads data as a file
 */
export function downloadFile(content: string, filename: string, mimeType: string) {
  const blob = new Blob([content], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/**
 * Exports tasks to specified format and downloads
 */
export function exportTasks(
  tasks: TaskWithAttemptStatus[],
  format: ExportFormat,
  projectName: string
) {
  const timestamp = new Date().toISOString().split('T')[0];
  const filename = `${projectName}-tasks-${timestamp}.${format}`;

  let content: string;
  let mimeType: string;

  switch (format) {
    case 'csv':
      content = tasksToCSV(tasks);
      mimeType = 'text/csv;charset=utf-8;';
      break;
    case 'json':
      content = tasksToJSON(tasks);
      mimeType = 'application/json;charset=utf-8;';
      break;
    default:
      throw new Error(`Unsupported export format: ${format}`);
  }

  downloadFile(content, filename, mimeType);
}

/**
 * Parses CSV content to task data
 */
export function parseCSV(content: string): Partial<TaskExportData>[] {
  const lines = content.split('\n').filter((line) => line.trim());
  if (lines.length < 2) {
    throw new Error('CSV file is empty or invalid');
  }

  // Parse headers
  const headers = lines[0].split(',').map((h) => h.trim().replace(/^"|"$/g, ''));

  // Parse rows
  const tasks: Partial<TaskExportData>[] = [];
  for (let i = 1; i < lines.length; i++) {
    const values = lines[i].split(',').map((v) => v.trim().replace(/^"|"$/g, ''));
    const task: Partial<TaskExportData> = {};

    headers.forEach((header, index) => {
      const value = values[index];
      if (value) {
        switch (header.toLowerCase()) {
          case 'id':
            task.id = value;
            break;
          case 'title':
            task.title = value;
            break;
          case 'description':
            task.description = value;
            break;
          case 'status':
            task.status = value;
            break;
          case 'priority':
            task.priority = value;
            break;
          case 'assignee':
            task.assignee = value;
            break;
          case 'parent task id':
            task.parent_task_id = value;
            break;
        }
      }
    });

    if (task.title) {
      tasks.push(task);
    }
  }

  return tasks;
}

/**
 * Parses JSON content to task data
 */
export function parseJSON(content: string): Partial<TaskExportData>[] {
  try {
    const data = JSON.parse(content);
    if (!Array.isArray(data)) {
      throw new Error('JSON must contain an array of tasks');
    }
    return data;
  } catch (error) {
    throw new Error(`Invalid JSON format: ${error instanceof Error ? error.message : 'Unknown error'}`);
  }
}
