import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, Clock, Loader2, XCircle } from 'lucide-react';
import { useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { socialApi, type SocialPostRecord } from '@/lib/api';
import { cn } from '@/lib/utils';

interface ApprovalQueueProps {
  projectId: string;
  reviewerName?: string;
  className?: string;
  onPostClick?: (postId: string) => void;
}

function platformBadgeColor(platform: string) {
  const map: Record<string, string> = {
    linkedin: 'bg-blue-600/15 text-blue-700 border-blue-300/40',
    instagram: 'bg-pink-500/15 text-pink-700 border-pink-300/40',
    twitter: 'bg-sky-400/15 text-sky-700 border-sky-300/40',
    facebook: 'bg-blue-700/15 text-blue-800 border-blue-300/40',
    tiktok: 'bg-muted text-foreground border-border/50',
    youtube: 'bg-red-500/15 text-red-700 border-red-300/40',
    threads: 'bg-muted text-foreground border-border/50',
  };
  return map[platform] ?? 'bg-muted text-muted-foreground border-border/50';
}

function parsePlatform(platforms: string): string {
  try {
    const arr = JSON.parse(platforms);
    return Array.isArray(arr) ? (arr[0] ?? 'unknown') : 'unknown';
  } catch {
    return platforms ?? 'unknown';
  }
}

export function ApprovalQueue({
  projectId,
  reviewerName,
  className,
  onPostClick,
}: ApprovalQueueProps) {
  const queryClient = useQueryClient();
  const [actionStates, setActionStates] = useState<
    Record<string, 'approving' | 'rejecting'>
  >({});

  const { data: posts = [], isLoading } = useQuery<SocialPostRecord[]>({
    queryKey: ['approval-queue', projectId],
    queryFn: () =>
      socialApi.listPostsFiltered({
        projectId,
        status: 'pending_review',
        limit: 50,
      }),
    refetchInterval: 30_000,
  });

  const transition = useMutation({
    mutationFn: ({ id, status }: { id: string; status: string }) =>
      socialApi.transitionStatus(id, status, reviewerName),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['approval-queue', projectId],
      });
      queryClient.invalidateQueries({ queryKey: ['calendar-posts'] });
    },
  });

  async function handle(id: string, action: 'approving' | 'rejecting') {
    setActionStates((prev) => ({ ...prev, [id]: action }));
    try {
      await transition.mutateAsync({
        id,
        status: action === 'approving' ? 'approved' : 'draft',
      });
    } finally {
      setActionStates((prev) => {
        const next = { ...prev };
        delete next[id];
        return next;
      });
    }
  }

  if (isLoading) {
    return (
      <div className={cn('flex items-center justify-center py-8', className)}>
        <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
      </div>
    );
  }

  if (!posts.length) {
    return (
      <div
        className={cn(
          'flex flex-col items-center justify-center py-10 text-center',
          className
        )}
      >
        <CheckCircle2 className="h-8 w-8 text-green-500 mb-2" />
        <p className="text-sm font-medium">All clear</p>
        <p className="text-xs text-muted-foreground mt-1">
          No posts awaiting approval
        </p>
      </div>
    );
  }

  return (
    <ScrollArea className={cn('h-full', className)}>
      <div className="space-y-3 p-1">
        {posts.map((post) => {
          const platform = parsePlatform(post.platforms);
          const busy = actionStates[post.id];
          const scheduledAt = post.scheduled_for
            ? new Date(post.scheduled_for).toLocaleString('en-US', {
                month: 'short',
                day: 'numeric',
                hour: 'numeric',
                minute: '2-digit',
              })
            : null;

          return (
            <Card key={post.id} className="overflow-hidden">
              <CardContent className="p-4">
                <div className="flex items-start gap-3">
                  <div className="flex-1 min-w-0 space-y-2">
                    <div className="flex items-center gap-2 flex-wrap">
                      <Badge
                        variant="outline"
                        className={cn('text-xs', platformBadgeColor(platform))}
                      >
                        {platform}
                      </Badge>
                      {post.category && (
                        <Badge variant="secondary" className="text-xs">
                          {post.category}
                        </Badge>
                      )}
                      {scheduledAt && (
                        <span className="flex items-center gap-1 text-xs text-muted-foreground">
                          <Clock className="h-3 w-3" />
                          {scheduledAt}
                        </span>
                      )}
                    </div>
                    <p
                      className={cn(
                        'text-sm line-clamp-3 text-foreground/90',
                        onPostClick && 'cursor-pointer hover:text-primary'
                      )}
                      onClick={() => onPostClick?.(post.id)}
                    >
                      {post.caption || (
                        <span className="text-muted-foreground italic">
                          No caption
                        </span>
                      )}
                    </p>
                  </div>

                  <div className="flex flex-col gap-1.5 shrink-0">
                    <Button
                      size="sm"
                      className="h-7 gap-1 bg-green-600 hover:bg-green-700 text-white"
                      disabled={!!busy}
                      onClick={() => handle(post.id, 'approving')}
                    >
                      {busy === 'approving' ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <CheckCircle2 className="h-3 w-3" />
                      )}
                      Approve
                    </Button>
                    <Button
                      size="sm"
                      variant="outline"
                      className="h-7 gap-1 text-destructive border-destructive/30 hover:bg-destructive/10"
                      disabled={!!busy}
                      onClick={() => handle(post.id, 'rejecting')}
                    >
                      {busy === 'rejecting' ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <XCircle className="h-3 w-3" />
                      )}
                      Reject
                    </Button>
                  </div>
                </div>
              </CardContent>
            </Card>
          );
        })}
      </div>
    </ScrollArea>
  );
}
