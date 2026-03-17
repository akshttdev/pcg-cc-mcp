import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { commentsApi } from '@/lib/api';
import { taskKeys } from '@/lib/query-keys';
import { useMutationWithToast } from '@/hooks/useMutationWithToast';
import type { TaskComment, AuthorType, CommentType } from 'shared/types';
import { format } from 'date-fns';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { cn } from '@/lib/utils';
import { User, Bot, Server, Trash2, Reply, MessageSquare } from 'lucide-react';
import { CommentInput } from './CommentInput';
import { Loader } from '@/components/ui/loader';

interface TaskCommentThreadProps {
  taskId: string;
  currentUserId?: string;
}

const AUTHOR_TYPE_CONFIG: Record<AuthorType, { icon: typeof User; color: string; label: string }> = {
  human: { icon: User, color: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200', label: 'Human' },
  agent: { icon: Bot, color: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200', label: 'Agent' },
  mcp: { icon: Server, color: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200', label: 'MCP' },
  system: { icon: MessageSquare, color: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200', label: 'System' },
};

const COMMENT_TYPE_LABELS: Record<CommentType, string> = {
  comment: 'Comment',
  statusupdate: 'Status Update',
  review: 'Review',
  approval: 'Approval',
  system: 'System',
  handoff: 'Handoff',
  mcpnotification: 'MCP Notification',
};

export function TaskCommentThread({ taskId, currentUserId = 'current-user' }: TaskCommentThreadProps) {
  const [replyingTo, setReplyingTo] = useState<string | null>(null);

  const { data: comments = [], isLoading, error } = useQuery({
    queryKey: taskKeys.comments(taskId),
    queryFn: () => commentsApi.getAll(taskId),
    refetchInterval: 10000, // Refresh every 10 seconds
  });

  const deleteMutation = useMutationWithToast({
    mutationFn: (commentId: string) => commentsApi.delete(commentId),
    successMessage: 'Comment deleted',
    errorMessage: 'Failed to delete comment',
    invalidateKeys: [taskKeys.comments(taskId)],
  });

  const handleDelete = (commentId: string) => {
    if (confirm('Are you sure you want to delete this comment?')) {
      deleteMutation.mutate(commentId);
    }
  };

  // Build thread structure
  interface CommentWithReplies extends TaskComment {
    replies: CommentWithReplies[];
  }

  const commentMap = new Map<string, CommentWithReplies>();
  const rootComments: CommentWithReplies[] = [];

  comments.forEach((comment) => {
    commentMap.set(comment.id, { ...comment, replies: [] });
  });

  comments.forEach((comment) => {
    if (comment.parent_comment_id) {
      const parent = commentMap.get(comment.parent_comment_id);
      if (parent) {
        parent.replies.push(commentMap.get(comment.id)!);
      }
    } else {
      rootComments.push(commentMap.get(comment.id)!);
    }
  });

  const renderComment = (comment: CommentWithReplies, depth = 0) => {
    const authorConfig = AUTHOR_TYPE_CONFIG[comment.author_type];
    const AuthorIcon = authorConfig.icon;
    const isReplyingToThis = replyingTo === comment.id;

    return (
      <div key={comment.id} className={cn('mb-4', depth > 0 && 'ml-8 border-l-2 pl-4')}>
        <Card>
          <CardHeader className="pb-3">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <Avatar className="h-8 w-8">
                  <AvatarFallback>
                    <AuthorIcon className="h-4 w-4" />
                  </AvatarFallback>
                </Avatar>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-sm">{comment.author_id}</span>
                    <Badge variant="outline" className={cn('text-xs', authorConfig.color)}>
                      {authorConfig.label}
                    </Badge>
                    {comment.comment_type !== 'comment' && (
                      <Badge variant="secondary" className="text-xs">
                        {COMMENT_TYPE_LABELS[comment.comment_type]}
                      </Badge>
                    )}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {format(new Date(comment.created_at), 'MMM d, yyyy h:mm a')}
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => setReplyingTo(isReplyingToThis ? null : comment.id)}
                >
                  <Reply className="h-4 w-4" />
                </Button>
                {comment.author_id === currentUserId && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleDelete(comment.id)}
                    className="text-destructive hover:text-destructive"
                  >
                    <Trash2 className="h-4 w-4" />
                  </Button>
                )}
              </div>
            </div>
          </CardHeader>
          <CardContent className="pt-0">
            <div className="text-sm whitespace-pre-wrap">{comment.content}</div>
            {comment.mentions && JSON.parse(comment.mentions).length > 0 && (
              <div className="mt-2 flex flex-wrap gap-1">
                {JSON.parse(comment.mentions).map((mention: string) => (
                  <Badge key={mention} variant="secondary" className="text-xs">
                    @{mention}
                  </Badge>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {isReplyingToThis && (
          <div className="mt-3 ml-8">
            <CommentInput
              taskId={taskId}
              parentCommentId={comment.id}
              onSuccess={() => setReplyingTo(null)}
              onCancel={() => setReplyingTo(null)}
              placeholder="Write a reply..."
            />
          </div>
        )}

        {comment.replies.length > 0 && (
          <div className="mt-4">
            {comment.replies.map((reply) => renderComment(reply, depth + 1))}
          </div>
        )}
      </div>
    );
  };

  if (isLoading) {
    return (
      <div className="flex justify-center py-8">
        <Loader />
      </div>
    );
  }

  if (error) {
    return (
      <div className="text-center py-8 text-destructive">
        Failed to load comments
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold mb-4">
          Comments ({comments.length})
        </h3>
        <CommentInput taskId={taskId} />
      </div>

      {comments.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground">
          No comments yet. Be the first to comment!
        </div>
      ) : (
        <ScrollArea className="h-[600px]">
          {rootComments.map((comment) => renderComment(comment))}
        </ScrollArea>
      )}
    </div>
  );
}
