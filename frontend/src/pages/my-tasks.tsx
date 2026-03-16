import { useMemo, useState, useCallback } from 'react';
import { Link } from 'react-router-dom';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Skeleton } from '@/components/ui/skeleton';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { ListTodo, Clock, AlertCircle, CheckCircle2, ArrowRight, Eye, Bot, User, PenLine, CheckSquare, X, Trash2, ArrowUpDown } from 'lucide-react';
import { EmptyState } from '@/components/ui/empty-state';
import { useAuth } from '@/contexts/AuthContext';
import { cn } from '@/lib/utils';
import { tasksApi, type AssignedTask } from '@/lib/api';
import { taskKeys, sidebarKeys } from '@/lib/query-keys';
import type { TaskStatus } from 'shared/types';
import { toast } from 'sonner';
import NiceModal from '@ebay/nice-modal-react';
import { TagChips } from '@/components/ui/tag-chips';

const BATCH_STATUS_OPTIONS = [
  { value: 'todo', label: 'To Do' },
  { value: 'inprogress', label: 'In Progress' },
  { value: 'inreview', label: 'In Review' },
  { value: 'done', label: 'Done' },
  { value: 'cancelled', label: 'Cancelled' },
] as const;

type FilterTab = 'all' | 'assigned' | 'created' | 'watching';
type SortBy = 'priority' | 'due_date' | 'updated';

const priorityOrder: Record<string, number> = { critical: 0, urgent: 0, high: 1, medium: 2, low: 3 };

export function MyTasksPage() {
  const { user } = useAuth();
  const queryClient = useQueryClient();
  const [activeTab, setActiveTab] = useState<FilterTab>('all');
  const [sortBy, setSortBy] = useState<SortBy>('priority');
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());

  const { data: assignedTasks = [], isLoading: loadingAssigned } = useQuery<AssignedTask[]>({
    queryKey: [...taskKeys.my(), user?.id],
    queryFn: () => tasksApi.getAssignedToMe(),
    enabled: !!user,
  });

  const { data: createdTasks = [], isLoading: loadingCreated } = useQuery<AssignedTask[]>({
    queryKey: [...taskKeys.myCreated(), user?.id],
    queryFn: () => tasksApi.getCreatedByMe(),
    enabled: !!user,
  });

  const { data: watchedTasks = [] } = useQuery<AssignedTask[]>({
    queryKey: [...taskKeys.myWatched(), user?.id],
    queryFn: () => tasksApi.getWatchedTasks(),
    enabled: !!user,
  });

  const isLoading = loadingAssigned || loadingCreated;

  // Deduplicated "all" union
  const allTasks = useMemo(() => {
    const seen = new Set<string>();
    const combined: AssignedTask[] = [];
    for (const t of [...assignedTasks, ...createdTasks]) {
      if (!seen.has(t.id)) {
        seen.add(t.id);
        combined.push(t);
      }
    }
    return combined;
  }, [assignedTasks, createdTasks]);

  const filteredTasks = useMemo(() => {
    let tasks: AssignedTask[];
    switch (activeTab) {
      case 'assigned': tasks = assignedTasks; break;
      case 'created': tasks = createdTasks; break;
      case 'watching': tasks = watchedTasks; break;
      default: tasks = allTasks;
    }

    return [...tasks].sort((a, b) => {
      if (sortBy === 'priority') {
        return (priorityOrder[a.priority] ?? 4) - (priorityOrder[b.priority] ?? 4);
      }
      if (sortBy === 'due_date') {
        if (!a.due_date && !b.due_date) return 0;
        if (!a.due_date) return 1;
        if (!b.due_date) return -1;
        return new Date(a.due_date).getTime() - new Date(b.due_date).getTime();
      }
      return 0; // 'updated' - already ordered by backend
    });
  }, [activeTab, sortBy, assignedTasks, createdTasks, watchedTasks, allTasks]);

  const toggleSelection = useCallback((taskId: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      if (next.has(taskId)) next.delete(taskId);
      else next.add(taskId);
      return next;
    });
  }, []);

  const exitSelectionMode = useCallback(() => {
    setSelectionMode(false);
    setSelectedIds(new Set());
  }, []);

  const invalidateMyTasks = useCallback(() => {
    queryClient.invalidateQueries({ queryKey: taskKeys.my() });
    queryClient.invalidateQueries({ queryKey: taskKeys.myCreated() });
    queryClient.invalidateQueries({ queryKey: taskKeys.myWatched() });
    queryClient.invalidateQueries({ queryKey: sidebarKeys.tree() });
  }, [queryClient]);

  const handleBatchStatusChange = useCallback(async (newStatus: string) => {
    const ids = [...selectedIds];
    const results = await Promise.allSettled(
      ids.map((id) => tasksApi.update(id, { status: newStatus as TaskStatus }))
    );
    const successCount = results.filter((r) => r.status === 'fulfilled').length;
    const failCount = results.length - successCount;
    invalidateMyTasks();
    if (successCount > 0) toast.success(`Updated ${successCount} task${successCount !== 1 ? 's' : ''} to ${newStatus}`);
    if (failCount > 0) toast.error(`Failed to update ${failCount} task${failCount !== 1 ? 's' : ''}`);
    exitSelectionMode();
  }, [selectedIds, invalidateMyTasks, exitSelectionMode]);

  const handleBatchDelete = useCallback(async () => {
    const result = await NiceModal.show('confirm', {
      title: 'Delete Tasks',
      message: `Are you sure you want to delete ${selectedIds.size} task${selectedIds.size !== 1 ? 's' : ''}? This cannot be undone.`,
      confirmText: 'Delete',
      variant: 'destructive',
    });
    if (result !== 'confirmed') return;

    const ids = [...selectedIds];
    const results = await Promise.allSettled(
      ids.map((id) => tasksApi.delete(id))
    );
    const successCount = results.filter((r) => r.status === 'fulfilled').length;
    const failCount = results.length - successCount;
    invalidateMyTasks();
    if (successCount > 0) toast.success(`Deleted ${successCount} task${successCount !== 1 ? 's' : ''}`);
    if (failCount > 0) toast.error(`Failed to delete ${failCount} task${failCount !== 1 ? 's' : ''}`);
    exitSelectionMode();
  }, [selectedIds, invalidateMyTasks, exitSelectionMode]);

  const tabs: { key: FilterTab; label: string; count: number; icon: typeof ListTodo }[] = [
    { key: 'all', label: 'All', count: allTasks.length, icon: ListTodo },
    { key: 'assigned', label: 'Assigned to Me', count: assignedTasks.length, icon: User },
    { key: 'created', label: 'Created by Me', count: createdTasks.length, icon: PenLine },
    { key: 'watching', label: 'Watching', count: watchedTasks.length, icon: Eye },
  ];

  if (isLoading) {
    return (
      <div className="flex flex-col h-full">
        <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
          <div className="flex items-center gap-3">
            <Skeleton className="h-9 w-9 rounded-lg" />
            <div>
              <Skeleton className="h-5 w-24 mb-1" />
              <Skeleton className="h-3 w-40" />
            </div>
          </div>
          <Skeleton className="h-8 w-32" />
        </div>
        <div className="flex gap-1 px-6 pt-3 pb-2 border-b bg-background">
          {[1, 2, 3, 4].map((i) => (
            <Skeleton key={i} className="h-8 w-24 rounded-md" />
          ))}
        </div>
        <div className="flex-1 overflow-auto p-6 space-y-2">
          {[1, 2, 3, 4].map((i) => (
            <Card key={i}>
              <CardContent className="py-3 px-4">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex-1 space-y-2">
                    <div className="flex items-center gap-2">
                      <Skeleton className="h-4 w-4 rounded-full" />
                      <Skeleton className="h-4 w-48" />
                    </div>
                    <Skeleton className="h-3 w-64 ml-6" />
                    <div className="flex items-center gap-2 ml-6">
                      <Skeleton className="h-3 w-20" />
                      <Skeleton className="h-3 w-16" />
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <Skeleton className="h-5 w-16 rounded-full" />
                    <Skeleton className="h-5 w-12 rounded-full" />
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b bg-background">
        <div className="flex items-center gap-3">
          <div className="rounded-lg bg-primary p-2">
            <ListTodo className="h-5 w-5 text-primary-foreground" />
          </div>
          <div>
            <h1 className="text-xl font-semibold">My Tasks</h1>
            <p className="text-sm text-muted-foreground">
              {assignedTasks.length} assigned, {createdTasks.length} created, {watchedTasks.length} watching
            </p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant={selectionMode ? 'default' : 'outline'}
            size="sm"
            onClick={() => selectionMode ? exitSelectionMode() : setSelectionMode(true)}
          >
            <CheckSquare className="h-3.5 w-3.5 mr-1.5" />
            {selectionMode ? 'Cancel' : 'Select'}
          </Button>
          <select
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as SortBy)}
            className="text-sm border rounded-md px-2 py-1 bg-background"
          >
            <option value="priority">Sort: Priority</option>
            <option value="due_date">Sort: Due Date</option>
            <option value="updated">Sort: Recently Updated</option>
          </select>
        </div>
      </div>

      {/* Filter Tabs */}
      <div className="flex gap-1 px-6 pt-3 pb-2 border-b bg-background overflow-x-auto">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.key}
              onClick={() => setActiveTab(tab.key)}
              className={cn(
                'flex items-center gap-1.5 px-3 py-1.5 text-sm rounded-md transition-colors whitespace-nowrap',
                activeTab === tab.key
                  ? 'bg-primary/10 text-primary font-medium'
                  : 'text-muted-foreground hover:bg-accent'
              )}
            >
              <Icon className="h-3.5 w-3.5" />
              {tab.label}
              <span className="text-xs opacity-70">({tab.count})</span>
            </button>
          );
        })}
      </div>

      {/* Task list */}
      <div className="flex-1 overflow-auto p-6">
        {filteredTasks.length === 0 ? (
          <Card className="border-dashed">
            <CardContent className="py-0">
              <EmptyState
                icon={CheckCircle2}
                title={activeTab === 'all' ? 'All caught up!' : `No ${activeTab === 'watching' ? 'watched' : activeTab} tasks`}
                description={
                  activeTab === 'all'
                    ? 'You have no tasks right now.'
                    : activeTab === 'assigned'
                      ? 'No tasks are assigned to you.'
                      : activeTab === 'created'
                        ? 'You haven\'t created any tasks yet.'
                        : 'You\'re not watching any tasks.'
                }
                className="py-8"
              />
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-2">
            {filteredTasks.map((task) => (
              <MyTaskCard
                key={task.id}
                task={task}
                selectionMode={selectionMode}
                isSelected={selectedIds.has(task.id)}
                onToggleSelection={toggleSelection}
              />
            ))}
          </div>
        )}
      </div>

      {/* Floating batch action bar */}
      {selectionMode && selectedIds.size > 0 && (
        <div className="sticky bottom-0 border-t bg-background/95 backdrop-blur-sm px-6 py-3 flex items-center gap-3 shadow-lg">
          <span className="text-sm font-medium">{selectedIds.size} selected</span>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm">
                <ArrowUpDown className="h-3.5 w-3.5 mr-1.5" />
                Change Status
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent>
              {BATCH_STATUS_OPTIONS.map((opt) => (
                <DropdownMenuItem key={opt.value} onClick={() => handleBatchStatusChange(opt.value)}>
                  {opt.label}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
          <Button variant="destructive" size="sm" onClick={handleBatchDelete}>
            <Trash2 className="h-3.5 w-3.5 mr-1.5" />
            Delete
          </Button>
          <Button variant="ghost" size="sm" onClick={exitSelectionMode}>
            <X className="h-3.5 w-3.5 mr-1.5" />
            Cancel
          </Button>
        </div>
      )}
    </div>
  );
}

const statusColors: Record<string, string> = {
  todo: 'bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300',
  inprogress: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300',
  inreview: 'bg-purple-100 text-purple-700 dark:bg-purple-900 dark:text-purple-300',
  done: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300',
  cancelled: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300',
};

const priorityColors: Record<string, string> = {
  critical: 'bg-red-100 text-red-800 border-red-200',
  urgent: 'bg-red-100 text-red-800 border-red-200',
  high: 'bg-orange-100 text-orange-800 border-orange-200',
  medium: 'bg-yellow-100 text-yellow-800 border-yellow-200',
  low: 'bg-green-100 text-green-800 border-green-200',
};

interface MyTaskCardProps {
  task: AssignedTask;
  selectionMode?: boolean;
  isSelected?: boolean;
  onToggleSelection?: (id: string) => void;
}

function MyTaskCard({ task, selectionMode, isSelected, onToggleSelection }: MyTaskCardProps) {
  const cardContent = (
    <Card className={cn(
      'hover:bg-accent/50 transition-colors cursor-pointer',
      isSelected && 'ring-2 ring-primary bg-primary/5',
    )}>
      <CardContent className="py-3 px-4">
        <div className="flex items-start justify-between gap-3">
          <div className="flex-1 min-w-0 space-y-1">
            <div className="flex items-center gap-2">
              {selectionMode && (
                <div
                  onClick={(e) => { e.preventDefault(); e.stopPropagation(); onToggleSelection?.(task.id); }}
                  className="shrink-0"
                >
                  <Checkbox checked={isSelected} onCheckedChange={() => onToggleSelection?.(task.id)} />
                </div>
              )}
              {task.status === 'done' ? (
                <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" />
              ) : task.status === 'inprogress' ? (
                <Clock className="h-4 w-4 text-blue-500 shrink-0" />
              ) : (
                <AlertCircle className="h-4 w-4 text-gray-400 shrink-0" />
              )}
              <p className="font-medium truncate">{task.title}</p>
            </div>

            {task.description && (
              <p className="text-xs text-muted-foreground line-clamp-1 pl-6">
                {task.description.slice(0, 100)}
              </p>
            )}

            <div className="flex items-center gap-2 pl-6 flex-wrap">
              <span className="text-xs text-muted-foreground">{task.project_name}</span>

              {task.assigned_agent && (
                <span className="flex items-center gap-0.5 text-xs text-blue-600 dark:text-blue-400">
                  <Bot className="h-3 w-3" />
                  {task.assigned_agent}
                </span>
              )}

              <TagChips tags={task.tags} maxVisible={2} size="xs" />
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {task.due_date && (
              <span className="text-xs text-muted-foreground">
                {new Date(task.due_date).toLocaleDateString()}
              </span>
            )}
            <Badge className={cn('text-xs', statusColors[task.status] || statusColors.todo)}>
              {task.status}
            </Badge>
            <Badge className={cn('text-xs', priorityColors[task.priority] || priorityColors.low)}>
              {task.priority || 'low'}
            </Badge>
            <ArrowRight className="h-4 w-4 text-muted-foreground" />
          </div>
        </div>
      </CardContent>
    </Card>
  );

  if (selectionMode) {
    return (
      <div onClick={() => onToggleSelection?.(task.id)}>
        {cardContent}
      </div>
    );
  }

  return (
    <Link to={`/projects/${task.project_id}/tasks/${task.id}`}>
      {cardContent}
    </Link>
  );
}
