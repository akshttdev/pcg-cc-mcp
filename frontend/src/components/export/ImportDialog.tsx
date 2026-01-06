import { useState, useRef } from 'react';
import { Upload, FileText, AlertCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Progress } from '@/components/ui/progress';
import { parseCSV, parseJSON } from '@/utils/exportUtils';
import type { TaskExportData } from '@/types/export';
import type { Priority, TaskStatus } from 'shared/types';
import { toast } from 'sonner';
import { tasksApi } from '@/lib/api';

interface ImportDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  projectId: string;
  onImportComplete: () => void;
}

export function ImportDialog({
  open,
  onOpenChange,
  projectId,
  onImportComplete,
}: ImportDialogProps) {
  const [file, setFile] = useState<File | null>(null);
  const [importing, setImporting] = useState(false);
  const [progress, setProgress] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFile = e.target.files?.[0];
    if (selectedFile) {
      const extension = selectedFile.name.split('.').pop()?.toLowerCase();
      if (extension !== 'csv' && extension !== 'json') {
        setError('Please select a CSV or JSON file');
        setFile(null);
        return;
      }
      setFile(selectedFile);
      setError(null);
    }
  };

  const handleImport = async () => {
    if (!file) {
      setError('Please select a file');
      return;
    }

    setImporting(true);
    setProgress(0);
    setError(null);

    try {
      // Read file content
      const content = await file.text();
      setProgress(20);

      // Parse file
      let tasks: Partial<TaskExportData>[];
      const extension = file.name.split('.').pop()?.toLowerCase();

      if (extension === 'csv') {
        tasks = parseCSV(content);
      } else if (extension === 'json') {
        tasks = parseJSON(content);
      } else {
        throw new Error('Unsupported file format');
      }

      setProgress(40);

      // Import tasks one by one
      let successCount = 0;
      let failCount = 0;
      const errors: string[] = [];

      const normalizePriority = (value?: string): Priority | null => {
        if (!value) return null;
        const lower = value.toLowerCase();
        if (['critical', 'high', 'medium', 'low'].includes(lower)) {
          return lower as Priority;
        }
        return null;
      };

      const normalizeStatus = (value?: string): TaskStatus => {
        const fallback: TaskStatus = 'todo';
        if (!value) return fallback;
        const lower = value.toLowerCase();
        if (
          ['todo', 'inprogress', 'inreview', 'done', 'cancelled'].includes(lower)
        ) {
          return lower as TaskStatus;
        }
        return fallback;
      };

      for (let i = 0; i < tasks.length; i++) {
        try {
          const task = tasks[i];
          const priority = normalizePriority(task.priority);
          const status = normalizeStatus(task.status);
          const assigneeId = task.assignee?.trim() || null;

          const createdTask = await tasksApi.create({
            project_id: projectId,
            title: task.title || 'Imported Task',
            description: task.description ?? null,
            priority: priority ?? 'medium',
            assignee_id: assigneeId,
            assigned_agent: null,
            agent_id: null,
            assigned_mcps: null,
            parent_task_id: task.parent_task_id ?? null,
            parent_task_attempt: null,
            image_ids: null,
            created_by: 'importer',
            requires_approval: false,
            tags: null,
            due_date: null,
            custom_properties: null,
            scheduled_start: null,
            scheduled_end: null,
          });
          if (status !== 'todo') {
            await tasksApi.update(createdTask.id, { status });
          }
          successCount++;
        } catch (err) {
          failCount++;
          errors.push(`Row ${i + 1}: ${err instanceof Error ? err.message : 'Unknown error'}`);
        }

        // Update progress
        setProgress(40 + Math.floor((i / tasks.length) * 60));
      }

      setProgress(100);

      // Show results
      if (successCount > 0) {
        toast.success(`Successfully imported ${successCount} task${successCount > 1 ? 's' : ''}`);
      }
      if (failCount > 0) {
        toast.error(`Failed to import ${failCount} task${failCount > 1 ? 's' : ''}`);
        if (errors.length > 0) {
          console.error('Import errors:', errors);
        }
      }

      onImportComplete();
      onOpenChange(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Import failed');
      toast.error('Import failed');
    } finally {
      setImporting(false);
    }
  };

  const handleClose = () => {
    if (!importing) {
      setFile(null);
      setError(null);
      setProgress(0);
      onOpenChange(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Import Tasks</DialogTitle>
          <DialogDescription>
            Import tasks from a CSV or JSON file
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* File Input */}
          <div className="space-y-2">
            <Label>Select File</Label>
            <div className="flex items-center gap-2">
              <input
                ref={fileInputRef}
                type="file"
                accept=".csv,.json"
                onChange={handleFileChange}
                className="hidden"
              />
              <Button
                variant="outline"
                onClick={() => fileInputRef.current?.click()}
                disabled={importing}
                className="w-full"
              >
                <Upload className="h-4 w-4 mr-2" />
                {file ? file.name : 'Choose File'}
              </Button>
            </div>
          </div>

          {/* File Info */}
          {file && !importing && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <FileText className="h-4 w-4" />
              <span>{(file.size / 1024).toFixed(1)} KB</span>
            </div>
          )}

          {/* Progress */}
          {importing && (
            <div className="space-y-2">
              <Label>Importing...</Label>
              <Progress value={progress} />
              <p className="text-sm text-muted-foreground text-center">
                {progress}%
              </p>
            </div>
          )}

          {/* Error */}
          {error && (
            <Alert variant="destructive">
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}

          {/* Instructions */}
          {!importing && (
            <div className="text-sm text-muted-foreground">
              <p className="font-medium mb-2">File Format Requirements:</p>
              <ul className="list-disc list-inside space-y-1">
                <li>CSV must have headers: Title, Description, Status, Priority, Assignee</li>
                <li>JSON must be an array of task objects</li>
                <li>At minimum, each task must have a "title" field</li>
              </ul>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={importing}>
            Cancel
          </Button>
          <Button onClick={handleImport} disabled={!file || importing}>
            {importing ? 'Importing...' : 'Import'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
