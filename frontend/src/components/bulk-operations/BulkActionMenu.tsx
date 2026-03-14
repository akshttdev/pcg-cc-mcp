import { useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { ChevronDown, GitPullRequest, Tag as TagIcon, Trash2 } from 'lucide-react';
import { useBulkSelectionStore } from '@/stores/useBulkSelectionStore';
import { useTagStore } from '@/stores/useTagStore';
import { tasksApi } from '@/lib/api';
import { toast } from 'sonner';
import type { TaskStatus } from 'shared/types';
import type { BulkOperationResult } from '@/types/bulk-operations';

interface BulkActionMenuProps {
  projectId: string;
  selectedCount: number;
  onComplete: () => void;
}

type DialogType = 'status' | 'tags' | 'delete' | null;

export function BulkActionMenu({
  projectId,
  selectedCount,
  onComplete,
}: BulkActionMenuProps) {
  const queryClient = useQueryClient();
  const { getSelectedIds } = useBulkSelectionStore();
  const { getTagsForProject } = useTagStore();
  const projectTags = getTagsForProject(projectId);

  const [dialogType, setDialogType] = useState<DialogType>(null);
  const [selectedStatus, setSelectedStatus] = useState<TaskStatus>('todo');
  const [selectedTagIds, setSelectedTagIds] = useState<string[]>([]);
  const [isProcessing, setIsProcessing] = useState(false);
  const [progress, setProgress] = useState(0);

  const handleBulkStatusChange = async () => {
    const taskIds = getSelectedIds();
    if (taskIds.length === 0) return;

    setIsProcessing(true);
    setProgress(0);

    const results: BulkOperationResult = {
      success: 0,
      failed: 0,
      total: taskIds.length,
      errors: [],
    };

    for (let i = 0; i < taskIds.length; i++) {
      try {
        await tasksApi.update(taskIds[i], { status: selectedStatus });
        results.success++;
      } catch (error) {
        results.failed++;
        results.errors.push({
          taskId: taskIds[i],
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
      setProgress(((i + 1) / taskIds.length) * 100);
    }

    setIsProcessing(false);
    setDialogType(null);

    queryClient.invalidateQueries({ queryKey: ['tasks'] });

    if (results.failed === 0) {
      toast.success(
        `Successfully updated ${results.success} task${results.success > 1 ? 's' : ''}`
      );
    } else {
      toast.error(
        `Updated ${results.success} tasks, ${results.failed} failed`
      );
    }

    onComplete();
  };

  const handleBulkDelete = async () => {
    const taskIds = getSelectedIds();
    if (taskIds.length === 0) return;

    setIsProcessing(true);
    setProgress(0);

    const results: BulkOperationResult = {
      success: 0,
      failed: 0,
      total: taskIds.length,
      errors: [],
    };

    for (let i = 0; i < taskIds.length; i++) {
      try {
        await tasksApi.delete(taskIds[i]);
        results.success++;
      } catch (error) {
        results.failed++;
        results.errors.push({
          taskId: taskIds[i],
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
      setProgress(((i + 1) / taskIds.length) * 100);
    }

    setIsProcessing(false);
    setDialogType(null);

    queryClient.invalidateQueries({ queryKey: ['tasks'] });

    if (results.failed === 0) {
      toast.success(
        `Successfully deleted ${results.success} task${results.success > 1 ? 's' : ''}`
      );
    } else {
      toast.error(
        `Deleted ${results.success} tasks, ${results.failed} failed`
      );
    }

    onComplete();
  };

  const toggleTag = (tagId: string) => {
    setSelectedTagIds((prev) =>
      prev.includes(tagId)
        ? prev.filter((id) => id !== tagId)
        : [...prev, tagId]
    );
  };

  return (
    <>
      <DropdownMenu>
        <DropdownMenuTrigger asChild>
          <Button variant="default" size="sm" className="gap-2">
            Actions
            <ChevronDown className="h-4 w-4" />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="end" className="w-48">
          <DropdownMenuItem onClick={() => setDialogType('status')}>
            <GitPullRequest className="h-4 w-4 mr-2" />
            Change Status
          </DropdownMenuItem>
          <DropdownMenuItem onClick={() => setDialogType('tags')}>
            <TagIcon className="h-4 w-4 mr-2" />
            Manage Tags
          </DropdownMenuItem>
          <DropdownMenuItem
            onClick={() => setDialogType('delete')}
            className="text-destructive"
          >
            <Trash2 className="h-4 w-4 mr-2" />
            Delete
          </DropdownMenuItem>
        </DropdownMenuContent>
      </DropdownMenu>

      {/* Status Change Dialog */}
      <Dialog
        open={dialogType === 'status'}
        onOpenChange={(open) => !open && setDialogType(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Change Status</DialogTitle>
            <DialogDescription>
              Update the status for {selectedCount} selected task
              {selectedCount > 1 ? 's' : ''}
            </DialogDescription>
          </DialogHeader>

          {isProcessing ? (
            <div className="space-y-2">
              <Progress value={progress} />
              <p className="text-sm text-muted-foreground text-center">
                Processing... {Math.round(progress)}%
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              <div>
                <label className="text-sm font-medium">New Status</label>
                <Select
                  value={selectedStatus}
                  onValueChange={(value) =>
                    setSelectedStatus(value as TaskStatus)
                  }
                >
                  <SelectTrigger>
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
            </div>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogType(null)}
              disabled={isProcessing}
            >
              Cancel
            </Button>
            <Button
              onClick={handleBulkStatusChange}
              disabled={isProcessing}
            >
              Update {selectedCount} Task{selectedCount > 1 ? 's' : ''}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Tags Dialog */}
      <Dialog
        open={dialogType === 'tags'}
        onOpenChange={(open) => !open && setDialogType(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Manage Tags</DialogTitle>
            <DialogDescription>
              Add or remove tags for {selectedCount} selected task
              {selectedCount > 1 ? 's' : ''}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4">
            <div>
              <p className="text-sm font-medium mb-2">Select Tags</p>
              {projectTags.length === 0 ? (
                <p className="text-sm text-muted-foreground">
                  No tags available. Create tags first.
                </p>
              ) : (
                <div className="flex flex-wrap gap-2">
                  {projectTags.map((tag) => (
                    <Badge
                      key={tag.id}
                      style={{
                        backgroundColor: selectedTagIds.includes(tag.id)
                          ? tag.color
                          : `${tag.color}20`,
                        color: selectedTagIds.includes(tag.id)
                          ? 'white'
                          : tag.color,
                        borderColor: `${tag.color}40`,
                        cursor: 'pointer',
                      }}
                      className="border"
                      onClick={() => toggleTag(tag.id)}
                    >
                      {tag.name}
                    </Badge>
                  ))}
                </div>
              )}
            </div>
            <p className="text-sm text-muted-foreground">
              Note: Selected tags will be added to all selected tasks. This
              operation does not remove existing tags.
            </p>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => {
                setDialogType(null);
                setSelectedTagIds([]);
              }}
            >
              Cancel
            </Button>
            <Button
              onClick={() => {
                toast.info('Bulk tag assignment coming soon!');
                setDialogType(null);
                setSelectedTagIds([]);
              }}
              disabled={selectedTagIds.length === 0}
            >
              Add Tags
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Dialog */}
      <Dialog
        open={dialogType === 'delete'}
        onOpenChange={(open) => !open && setDialogType(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete Tasks</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete {selectedCount} task
              {selectedCount > 1 ? 's' : ''}? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>

          {isProcessing && (
            <div className="space-y-2">
              <Progress value={progress} />
              <p className="text-sm text-muted-foreground text-center">
                Deleting... {Math.round(progress)}%
              </p>
            </div>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogType(null)}
              disabled={isProcessing}
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleBulkDelete}
              disabled={isProcessing}
            >
              Delete {selectedCount} Task{selectedCount > 1 ? 's' : ''}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
