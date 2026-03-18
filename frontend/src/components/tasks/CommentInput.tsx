import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { commentsApi, activityApi } from '@/lib/api';
import { taskKeys } from '@/lib/query-keys';
import type { CreateTaskComment, CommentType, TaskComment } from 'shared/types';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { toast } from 'sonner';
import { MessageSquare, Send, X } from 'lucide-react';

interface CommentInputProps {
  taskId: string;
  parentCommentId?: string;
  onSuccess?: () => void;
  onCancel?: () => void;
  placeholder?: string;
  currentUserId?: string;
}

const COMMENT_TYPE_OPTIONS: { value: CommentType; label: string }[] = [
  { value: 'comment', label: 'Comment' },
  { value: 'statusupdate', label: 'Status Update' },
  { value: 'review', label: 'Review' },
];

export function CommentInput({
  taskId,
  parentCommentId,
  onSuccess,
  onCancel,
  placeholder = 'Write a comment...',
  currentUserId = 'current-user',
}: CommentInputProps) {
  const queryClient = useQueryClient();
  const [content, setContent] = useState('');
  const [commentType, setCommentType] = useState<CommentType>('comment');

  const createMutation = useMutation({
    mutationFn: (newComment: CreateTaskComment) => commentsApi.create(newComment),
    onSuccess: async (comment: TaskComment) => {
      queryClient.invalidateQueries({ queryKey: taskKeys.comments(taskId) });
      queryClient.invalidateQueries({ queryKey: taskKeys.activity(taskId) });
      setContent('');
      setCommentType('comment');
      toast.success('Comment posted');
      onSuccess?.();
      try {
        await activityApi.create({
          task_id: taskId,
          actor_id: currentUserId,
          actor_type: 'human',
          action: 'commented',
          previous_state: null,
          new_state: null,
          metadata: {
            comment_id: comment.id,
            comment_type: comment.comment_type,
            preview: comment.content.slice(0, 120),
          },
        });
      } catch (error) {
        console.error('Failed to log comment activity', error);
      }
    },
    onError: () => {
      toast.error('Failed to post comment');
    },
  });

  const handleSubmit = () => {
    if (!content.trim()) {
      toast.error('Comment cannot be empty');
      return;
    }

    // Extract mentions from content (simple @username detection)
    const mentionRegex = /@(\w+)/g;
    const mentions: string[] = [];
    let match;
    while ((match = mentionRegex.exec(content)) !== null) {
      mentions.push(match[1]);
    }

    const newComment: CreateTaskComment = {
      task_id: taskId,
      author_id: currentUserId,
      author_type: 'human',
      content: content.trim(),
      comment_type: commentType,
      parent_comment_id: parentCommentId || null,
      mentions: mentions.length > 0 ? mentions : null,
      metadata: null,
    };

    createMutation.mutate(newComment);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
      e.preventDefault();
      handleSubmit();
    }
  };

  const charCount = content.length;
  const maxChars = 2000;
  const isOverLimit = charCount > maxChars;

  return (
    <div className="space-y-3 rounded-lg border p-4 bg-card">
      <div className="flex items-center gap-2">
        <MessageSquare className="h-4 w-4 text-muted-foreground" />
        <Select value={commentType} onValueChange={(value) => setCommentType(value as CommentType)}>
          <SelectTrigger className="w-[180px] h-8">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {COMMENT_TYPE_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <Textarea
        placeholder={placeholder}
        value={content}
        onChange={(e) => setContent(e.target.value)}
        onKeyDown={handleKeyDown}
        className="min-h-[100px] resize-none"
        disabled={createMutation.isPending}
      />

      <div className="flex items-center justify-between">
        <div className="text-xs text-muted-foreground">
          <span className={isOverLimit ? 'text-destructive' : ''}>
            {charCount} / {maxChars}
          </span>
          <span className="ml-2">• Use @ to mention team members</span>
          <span className="ml-2">• Cmd/Ctrl + Enter to submit</span>
        </div>

        <div className="flex items-center gap-2">
          {onCancel && (
            <Button
              variant="ghost"
              size="sm"
              onClick={onCancel}
              disabled={createMutation.isPending}
            >
              <X className="h-4 w-4 mr-1" />
              Cancel
            </Button>
          )}
          <Button
            size="sm"
            onClick={handleSubmit}
            disabled={!content.trim() || isOverLimit || createMutation.isPending}
          >
            <Send className="h-4 w-4 mr-1" />
            {createMutation.isPending ? 'Posting...' : 'Post'}
          </Button>
        </div>
      </div>
    </div>
  );
}
