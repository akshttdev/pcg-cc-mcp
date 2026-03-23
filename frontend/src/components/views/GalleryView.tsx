import { useNavigate } from 'react-router-dom';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Button } from '@/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Clock, User, Calendar, MoreHorizontal, Edit, Copy, Trash2, Bot, Server, Tag, AlertCircle } from 'lucide-react';
import type { TaskWithAttemptStatus } from 'shared/types';
import { cn } from '@/lib/utils';
import { useBulkSelectionStore } from '@/stores/useBulkSelectionStore';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { format } from 'date-fns';

interface GalleryViewProps {
  tasks: TaskWithAttemptStatus[];
  projectId: string;
  onEditTask?: (task: TaskWithAttemptStatus) => void;
  onDeleteTask?: (taskId: string) => void;
  onDuplicateTask?: (task: TaskWithAttemptStatus) => void;
}

const getStatusColor = (status: string) => {
  switch (status.toLowerCase()) {
    case 'completed':
      return 'bg-green-500/10 text-green-700 dark:text-green-400 border-green-500/20';
    case 'in_progress':
      return 'bg-blue-500/10 text-blue-700 dark:text-blue-400 border-blue-500/20';
    case 'blocked':
      return 'bg-red-500/10 text-red-700 dark:text-red-400 border-red-500/20';
    case 'pending':
      return 'bg-yellow-500/10 text-yellow-700 dark:text-yellow-400 border-yellow-500/20';
    default:
      return 'bg-gray-500/10 text-gray-700 dark:text-gray-400 border-gray-500/20';
  }
};

const getPriorityColor = (priority: string) => {
  switch (priority.toLowerCase()) {
    case 'critical':
      return 'bg-red-500/10 text-red-700 dark:text-red-400 border-red-500/50';
    case 'high':
      return 'bg-orange-500/10 text-orange-700 dark:text-orange-400 border-orange-500/50';
    case 'medium':
      return 'bg-yellow-500/10 text-yellow-700 dark:text-yellow-400 border-yellow-500/50';
    case 'low':
      return 'bg-blue-500/10 text-blue-700 dark:text-blue-400 border-blue-500/50';
    default:
      return 'bg-gray-500/10 text-gray-700 dark:text-gray-400';
  }
};

const getApprovalColor = (status: string | null | undefined) => {
  if (!status) return '';
  switch (status.toLowerCase()) {
    case 'approved':
      return 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200';
    case 'pending':
      return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200';
    case 'rejected':
      return 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200';
    case 'changesrequested':
      return 'bg-orange-100 text-orange-800 dark:bg-orange-900 dark:text-orange-200';
    default:
      return '';
  }
};

export function GalleryView({ tasks, projectId, onEditTask, onDeleteTask, onDuplicateTask }: GalleryViewProps) {
  const navigate = useNavigate();
  const { selectionMode, isSelected, toggleTask } = useBulkSelectionStore();

  const handleCardClick = (taskId: string) => {
    if (selectionMode) {
      toggleTask(taskId);
    } else {
      navigate(`/projects/${projectId}/tasks/${taskId}`);
    }
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
      {tasks.map((task) => (
        <Card
          key={task.id}
          className={cn(
            'cursor-pointer transition-all hover:shadow-md hover:scale-[1.02] border-l-4 relative',
            getStatusColor(task.status || 'pending'),
            selectionMode && isSelected(task.id) && 'ring-2 ring-primary'
          )}
          onClick={() => handleCardClick(task.id)}
        >
          {/* Checkbox overlay for selection mode */}
          {selectionMode && (
            <div
              className="absolute top-2 right-2 z-10"
              onClick={(e) => {
                e.stopPropagation();
                toggleTask(task.id);
              }}
            >
              <Checkbox
                checked={isSelected(task.id)}
                onCheckedChange={() => toggleTask(task.id)}
              />
            </div>
          )}
          <CardHeader className="space-y-2">
            <div className="flex items-start justify-between gap-2">
              <CardTitle className="text-base line-clamp-2 flex-1">{task.title}</CardTitle>
              <div className="flex items-center gap-1 shrink-0">
                {task.priority && (
                  <Badge className={cn('capitalize', getPriorityColor(task.priority))}>
                    {task.priority}
                  </Badge>
                )}
                {!selectionMode && (
                  <div
                    onClick={(e) => {
                      e.stopPropagation();
                    }}
                    onPointerDown={(e) => e.stopPropagation()}
                    onMouseDown={(e) => e.stopPropagation()}
                  >
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-6 w-6 p-0"
                        >
                          <MoreHorizontal className="h-3 w-3" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        {onEditTask && (
                          <DropdownMenuItem onClick={() => onEditTask(task)}>
                            <Edit className="h-4 w-4 mr-2" />
                            Edit
                          </DropdownMenuItem>
                        )}
                        {onDuplicateTask && (
                          <DropdownMenuItem onClick={() => onDuplicateTask(task)}>
                            <Copy className="h-4 w-4 mr-2" />
                            Duplicate
                          </DropdownMenuItem>
                        )}
                        {onDeleteTask && (
                          <DropdownMenuItem
                            onClick={() => onDeleteTask(task.id)}
                            className="text-destructive"
                          >
                            <Trash2 className="h-4 w-4 mr-2" />
                            Delete
                          </DropdownMenuItem>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                )}
              </div>
            </div>
            {task.description && (
              <CardDescription className="line-clamp-2">
                {task.description}
              </CardDescription>
            )}
          </CardHeader>
          <CardContent>
            <div className="space-y-2 text-xs text-muted-foreground">
              <div className="flex items-center gap-2">
                <Clock className="h-3 w-3" />
                <Badge className={cn('capitalize text-xs', getStatusColor(task.status || 'pending'))}>
                  {(task.status || 'pending').replace('_', ' ')}
                </Badge>
              </div>

              {/* Assignee */}
              {task.assignee_id && (
                <div className="flex items-center gap-2">
                  <Avatar className="h-4 w-4">
                    <AvatarFallback className="text-[8px]">
                      <User className="h-3 w-3" />
                    </AvatarFallback>
                  </Avatar>
                  <span className="truncate">{task.assignee_id}</span>
                </div>
              )}

              {/* Assigned Agent */}
              {task.assigned_agent && (
                <div className="flex items-center gap-2">
                  <Bot className="h-3 w-3" />
                  <Badge variant="outline" className="text-xs bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200">
                    {task.assigned_agent}
                  </Badge>
                </div>
              )}

              {/* Assigned MCPs */}
              {task.assigned_mcps && JSON.parse(task.assigned_mcps).length > 0 && (
                <div className="flex items-center gap-2">
                  <Server className="h-3 w-3" />
                  <div className="flex flex-wrap gap-1">
                    {JSON.parse(task.assigned_mcps).map((mcp: string) => (
                      <Badge key={mcp} variant="outline" className="text-xs bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200">
                        {mcp}
                      </Badge>
                    ))}
                  </div>
                </div>
              )}

              {/* Tags */}
              {task.tags && JSON.parse(task.tags).length > 0 && (
                <div className="flex items-center gap-2">
                  <Tag className="h-3 w-3" />
                  <div className="flex flex-wrap gap-1">
                    {JSON.parse(task.tags).slice(0, 3).map((tag: string) => (
                      <Badge key={tag} variant="secondary" className="text-xs">
                        {tag}
                      </Badge>
                    ))}
                    {JSON.parse(task.tags).length > 3 && (
                      <Badge variant="secondary" className="text-xs">
                        +{JSON.parse(task.tags).length - 3}
                      </Badge>
                    )}
                  </div>
                </div>
              )}

              {/* Due Date */}
              {task.due_date && (
                <div className="flex items-center gap-2">
                  <Calendar className="h-3 w-3" />
                  <span>{format(new Date(task.due_date), 'MMM d, yyyy')}</span>
                </div>
              )}

              {/* Approval Status */}
              {task.requires_approval && task.approval_status && (
                <div className="flex items-center gap-2">
                  <AlertCircle className="h-3 w-3" />
                  <Badge className={cn('text-xs capitalize', getApprovalColor(task.approval_status))}>
                    {task.approval_status.replace('changesrequested', 'Changes Requested')}
                  </Badge>
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      ))}
    </div>
  );
}