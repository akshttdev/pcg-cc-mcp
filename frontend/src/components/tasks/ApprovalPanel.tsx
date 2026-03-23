import {
  AlertCircle,
  CheckCircle,
  Clock,
  MessageSquare,
  XCircle,
} from 'lucide-react';
import { useState } from 'react';
import type { ApprovalStatus, Task, TaskWithAttemptStatus } from 'shared/types';
import { toast } from 'sonner';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { DialogFooter } from '@/components/ui/dialog';
import { Textarea } from '@/components/ui/textarea';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import { taskApprovalApi } from '@/lib/api';
import { taskKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';

interface ApprovalPanelProps {
  task: Task | TaskWithAttemptStatus;
  canApprove?: boolean;
}

const APPROVAL_STATUS_CONFIG: Record<
  ApprovalStatus,
  { icon: typeof CheckCircle; color: string; label: string }
> = {
  pending: {
    icon: Clock,
    color:
      'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200',
    label: 'Pending Approval',
  },
  approved: {
    icon: CheckCircle,
    color: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200',
    label: 'Approved',
  },
  rejected: {
    icon: XCircle,
    color: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200',
    label: 'Rejected',
  },
  changesrequested: {
    icon: AlertCircle,
    color:
      'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200',
    label: 'Changes Requested',
  },
};

export function ApprovalPanel({ task, canApprove = true }: ApprovalPanelProps) {
  const [showCommentInput, setShowCommentInput] = useState(false);
  const [comment, setComment] = useState('');

  const approveMutation = useMutationWithToast({
    mutationFn: () => taskApprovalApi.approve(task.id),
    successMessage: 'Task approved',
    errorMessage: 'Failed to approve task',
    invalidateKeys: [taskKeys.all, taskKeys.activity(task.id)],
  });

  const rejectMutation = useMutationWithToast({
    mutationFn: () => taskApprovalApi.reject(task.id),
    successMessage: 'Task rejected',
    errorMessage: 'Failed to reject task',
    invalidateKeys: [taskKeys.all, taskKeys.activity(task.id)],
  });

  const requestChangesMutation = useMutationWithToast({
    mutationFn: () => taskApprovalApi.requestChanges(task.id),
    successMessage: 'Changes requested',
    errorMessage: 'Failed to request changes',
    invalidateKeys: [taskKeys.all, taskKeys.activity(task.id)],
    onSuccess: () => {
      setShowCommentInput(false);
      setComment('');
    },
  });

  if (!task.requires_approval) {
    return null;
  }

  const status = task.approval_status || 'pending';
  const statusConfig = APPROVAL_STATUS_CONFIG[status];
  const StatusIcon = statusConfig.icon;
  const isPending = status === 'pending';
  const isApproved = status === 'approved';

  const handleApprove = () => {
    if (confirm('Are you sure you want to approve this task?')) {
      approveMutation.mutate();
    }
  };

  const handleReject = () => {
    if (confirm('Are you sure you want to reject this task?')) {
      rejectMutation.mutate();
    }
  };

  const handleRequestChanges = () => {
    if (!comment.trim()) {
      toast.error('Please add a comment explaining what changes are needed');
      return;
    }
    requestChangesMutation.mutate();
  };

  return (
    <Card
      className={cn(
        'border-2',
        isApproved && 'border-green-200 dark:border-green-800'
      )}
    >
      <CardHeader>
        <CardTitle className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <MessageSquare className="h-5 w-5" />
            <span>Approval Status</span>
          </div>
          <Badge className={cn('text-sm', statusConfig.color)}>
            <StatusIcon className="h-4 w-4 mr-1" />
            {statusConfig.label}
          </Badge>
        </CardTitle>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Current Status Info */}
        <div className="text-sm text-muted-foreground">
          {isPending && (
            <p>
              This task requires approval before it can be marked as complete.
            </p>
          )}
          {isApproved && (
            <p>This task has been approved and can proceed to completion.</p>
          )}
          {status === 'rejected' && (
            <p>This task has been rejected and needs to be revised.</p>
          )}
          {status === 'changesrequested' && (
            <p>
              Changes have been requested. Please review the feedback and make
              necessary updates.
            </p>
          )}
        </div>

        {/* Action Buttons - Only show if pending and user has permission */}
        {isPending && canApprove && (
          <div className="space-y-3">
            <div className="flex gap-2">
              <Button
                onClick={handleApprove}
                disabled={approveMutation.isPending}
                className="flex-1 bg-green-600 hover:bg-green-700"
              >
                <CheckCircle className="h-4 w-4 mr-2" />
                {approveMutation.isPending ? 'Approving...' : 'Approve'}
              </Button>

              <Button
                variant="outline"
                onClick={() => setShowCommentInput(!showCommentInput)}
                className="flex-1"
              >
                <AlertCircle className="h-4 w-4 mr-2" />
                Request Changes
              </Button>

              <Button
                variant="destructive"
                onClick={handleReject}
                disabled={rejectMutation.isPending}
                className="flex-1"
              >
                <XCircle className="h-4 w-4 mr-2" />
                {rejectMutation.isPending ? 'Rejecting...' : 'Reject'}
              </Button>
            </div>

            {/* Comment Input for Request Changes */}
            {showCommentInput && (
              <div className="space-y-2 p-3 bg-muted/50 rounded-lg">
                <label className="text-sm font-medium">
                  Explain what changes are needed:
                </label>
                <Textarea
                  placeholder="Describe the changes that need to be made..."
                  value={comment}
                  onChange={(e) => setComment(e.target.value)}
                  className="min-h-[100px]"
                />
                <DialogFooter>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => {
                      setShowCommentInput(false);
                      setComment('');
                    }}
                  >
                    Cancel
                  </Button>
                  <Button
                    size="sm"
                    onClick={handleRequestChanges}
                    disabled={
                      !comment.trim() || requestChangesMutation.isPending
                    }
                  >
                    {requestChangesMutation.isPending
                      ? 'Sending...'
                      : 'Send Request'}
                  </Button>
                </DialogFooter>
              </div>
            )}
          </div>
        )}

        {/* Re-request Approval Button (for rejected/changes requested) */}
        {(status === 'rejected' || status === 'changesrequested') && (
          <div className="p-3 bg-muted/50 rounded-lg">
            <p className="text-sm text-muted-foreground mb-2">
              Once you've made the necessary changes, you can re-request
              approval.
            </p>
            <Button
              variant="outline"
              onClick={() => {
                // This would reset status to pending
                // For now, just show a toast
                toast.info(
                  'Feature coming soon: Re-request approval after making changes'
                );
              }}
              className="w-full"
            >
              <Clock className="h-4 w-4 mr-2" />
              Re-request Approval
            </Button>
          </div>
        )}

        {/* Permission Notice */}
        {!canApprove && isPending && (
          <div className="p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg border border-yellow-200 dark:border-yellow-800">
            <p className="text-sm text-yellow-800 dark:text-yellow-200">
              You don't have permission to approve this task. Please contact an
              admin or the task owner.
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
