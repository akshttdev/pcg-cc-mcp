import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { GripVertical, Loader2, RefreshCw, Repeat2 } from 'lucide-react';
import { useRef, useState } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { socialApi, type SocialPostRecord } from '@/lib/api';
import { cn } from '@/lib/utils';

interface QueueManagerProps {
  projectId: string;
  className?: string;
}

const CATEGORIES = [
  { value: '', label: 'All Categories' },
  { value: 'community', label: 'Community' },
  { value: 'events', label: 'Events' },
  { value: 'vibe', label: 'Vibe' },
  { value: 'drinks', label: 'Drinks' },
  { value: 'promo', label: 'Promo' },
];

function parsePlatform(platforms: string): string {
  try {
    const arr = JSON.parse(platforms);
    return Array.isArray(arr) ? (arr[0] ?? '') : '';
  } catch {
    return platforms ?? '';
  }
}

export function QueueManager({ projectId, className }: QueueManagerProps) {
  const queryClient = useQueryClient();
  const [category, setCategory] = useState('');
  const [orderedIds, setOrderedIds] = useState<string[] | null>(null);
  const dragIdx = useRef<number | null>(null);
  const [dragOver, setDragOver] = useState<number | null>(null);

  const { data: posts = [], isLoading } = useQuery({
    queryKey: ['social-queue', projectId, category],
    queryFn: () => socialApi.listQueue(projectId, category || undefined),
    select: (data) => {
      if (orderedIds) {
        const map = new Map(data.map((p) => [p.id, p]));
        return orderedIds
          .map((id) => map.get(id))
          .filter(Boolean) as SocialPostRecord[];
      }
      return data;
    },
  });

  const reorder = useMutation({
    mutationFn: (ids: string[]) => socialApi.reorderQueue(ids),
    onSuccess: () => {
      setOrderedIds(null);
      queryClient.invalidateQueries({ queryKey: ['social-queue', projectId] });
    },
  });

  const displayPosts = orderedIds
    ? (orderedIds
        .map((id) => posts.find((p) => p.id === id))
        .filter(Boolean) as SocialPostRecord[])
    : posts;

  function handleDragStart(idx: number) {
    dragIdx.current = idx;
  }

  function handleDragOver(e: React.DragEvent, idx: number) {
    e.preventDefault();
    setDragOver(idx);
  }

  function handleDrop(toIdx: number) {
    const fromIdx = dragIdx.current;
    if (fromIdx === null || fromIdx === toIdx) {
      setDragOver(null);
      return;
    }
    const ids = displayPosts.map((p) => p.id);
    const [removed] = ids.splice(fromIdx, 1);
    ids.splice(toIdx, 0, removed);
    setOrderedIds(ids);
    dragIdx.current = null;
    setDragOver(null);
  }

  return (
    <Card className={className}>
      <CardHeader className="flex flex-row items-start justify-between gap-3">
        <div>
          <CardTitle className="flex items-center gap-2">
            <Repeat2 className="h-4 w-4 text-muted-foreground" />
            Evergreen Queue
          </CardTitle>
          <CardDescription>Drag to reorder recycled content</CardDescription>
        </div>
        <div className="flex items-center gap-2">
          <Select
            value={category}
            onValueChange={(v) => {
              setCategory(v);
              setOrderedIds(null);
            }}
          >
            <SelectTrigger className="h-8 w-40 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {CATEGORIES.map((c) => (
                <SelectItem key={c.value} value={c.value}>
                  {c.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          {orderedIds && (
            <Button
              size="sm"
              className="h-8 text-xs"
              onClick={() => reorder.mutate(orderedIds)}
              disabled={reorder.isPending}
            >
              {reorder.isPending ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                <RefreshCw className="h-3 w-3" />
              )}
              Save order
            </Button>
          )}
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="flex items-center justify-center h-32">
            <Loader2 className="h-5 w-5 animate-spin text-muted-foreground" />
          </div>
        ) : displayPosts.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">
            No evergreen posts found. Mark posts as evergreen to add them here.
          </p>
        ) : (
          <ScrollArea className="max-h-96">
            <div className="space-y-2">
              {displayPosts.map((post, idx) => {
                const platform = parsePlatform(post.platforms);
                return (
                  <div
                    key={post.id}
                    draggable
                    onDragStart={() => handleDragStart(idx)}
                    onDragOver={(e) => handleDragOver(e, idx)}
                    onDragLeave={() => setDragOver(null)}
                    onDrop={() => handleDrop(idx)}
                    className={cn(
                      'flex items-center gap-2 p-2.5 rounded-lg border bg-card hover:bg-muted/30 cursor-grab active:cursor-grabbing transition-colors',
                      dragOver === idx && 'border-primary/50 bg-primary/5'
                    )}
                  >
                    <GripVertical className="h-4 w-4 text-muted-foreground/50 shrink-0" />
                    <span className="text-xs font-medium text-muted-foreground w-5 shrink-0">
                      {idx + 1}
                    </span>
                    <div className="flex-1 min-w-0">
                      <p className="text-sm line-clamp-1">
                        {post.caption || (
                          <span className="text-muted-foreground italic">
                            No caption
                          </span>
                        )}
                      </p>
                      <div className="flex items-center gap-2 mt-0.5">
                        {platform && (
                          <Badge variant="outline" className="text-xs h-4">
                            {platform}
                          </Badge>
                        )}
                        {post.category && (
                          <span className="text-xs text-muted-foreground">
                            {post.category}
                          </span>
                        )}
                        {post.recycle_after_days && (
                          <span className="text-xs text-muted-foreground">
                            ↻ every {post.recycle_after_days}d
                          </span>
                        )}
                      </div>
                    </div>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
        )}
      </CardContent>
    </Card>
  );
}
