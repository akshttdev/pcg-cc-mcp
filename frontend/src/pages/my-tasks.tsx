import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { Card, CardContent } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { ListTodo, Clock, AlertCircle, CheckCircle2, ArrowRight } from 'lucide-react';
import { useAuth } from '@/contexts/AuthContext';
import { cn } from '@/lib/utils';
import { tasksApi, type AssignedTask } from '@/lib/api';

export function MyTasksPage() {
  const { user } = useAuth();

  const { data: tasks = [], isLoading, error } = useQuery<AssignedTask[]>({
    queryKey: ['my-tasks', user?.id],
    queryFn: () => tasksApi.getAssignedToMe(),
    enabled: !!user,
  });

  const groupedTasks = useMemo(() => {
    const high = tasks.filter((t) => t.priority === 'high' || t.priority === 'urgent');
    const medium = tasks.filter((t) => t.priority === 'medium');
    const low = tasks.filter((t) => t.priority === 'low' || !t.priority);
    return { high, medium, low };
  }, [tasks]);

  const getPriorityColor = (priority: string) => {
    switch (priority) {
      case 'urgent':
      case 'high':
        return 'bg-red-100 text-red-800 border-red-200';
      case 'medium':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200';
      default:
        return 'bg-green-100 text-green-800 border-green-200';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'completed':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case 'in_progress':
        return <Clock className="h-4 w-4 text-blue-500" />;
      default:
        return <AlertCircle className="h-4 w-4 text-gray-400" />;
    }
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader message="Loading your tasks..." size={32} />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
        <AlertCircle className="h-12 w-12 mb-4 text-destructive" />
        <p>Failed to load tasks</p>
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
              {tasks.length} task{tasks.length !== 1 ? 's' : ''} assigned to you
            </p>
          </div>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 overflow-auto p-6">
        {tasks.length === 0 ? (
          <Card className="border-dashed">
            <CardContent className="py-12 text-center">
              <CheckCircle2 className="h-12 w-12 mx-auto mb-4 text-green-500" />
              <h3 className="text-lg font-medium mb-2">All caught up!</h3>
              <p className="text-muted-foreground">
                You have no tasks assigned to you right now.
              </p>
            </CardContent>
          </Card>
        ) : (
          <div className="space-y-6">
            {/* High Priority */}
            {groupedTasks.high.length > 0 && (
              <div>
                <h2 className="text-sm font-semibold text-red-600 mb-3 flex items-center gap-2">
                  <span className="h-2 w-2 rounded-full bg-red-500" />
                  HIGH PRIORITY ({groupedTasks.high.length})
                </h2>
                <div className="space-y-2">
                  {groupedTasks.high.map((task) => (
                    <TaskCard key={task.id} task={task} getPriorityColor={getPriorityColor} getStatusIcon={getStatusIcon} />
                  ))}
                </div>
              </div>
            )}

            {/* Medium Priority */}
            {groupedTasks.medium.length > 0 && (
              <div>
                <h2 className="text-sm font-semibold text-yellow-600 mb-3 flex items-center gap-2">
                  <span className="h-2 w-2 rounded-full bg-yellow-500" />
                  MEDIUM PRIORITY ({groupedTasks.medium.length})
                </h2>
                <div className="space-y-2">
                  {groupedTasks.medium.map((task) => (
                    <TaskCard key={task.id} task={task} getPriorityColor={getPriorityColor} getStatusIcon={getStatusIcon} />
                  ))}
                </div>
              </div>
            )}

            {/* Low Priority */}
            {groupedTasks.low.length > 0 && (
              <div>
                <h2 className="text-sm font-semibold text-green-600 mb-3 flex items-center gap-2">
                  <span className="h-2 w-2 rounded-full bg-green-500" />
                  LOW PRIORITY ({groupedTasks.low.length})
                </h2>
                <div className="space-y-2">
                  {groupedTasks.low.map((task) => (
                    <TaskCard key={task.id} task={task} getPriorityColor={getPriorityColor} getStatusIcon={getStatusIcon} />
                  ))}
                </div>
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

function TaskCard({
  task,
  getPriorityColor,
  getStatusIcon,
}: {
  task: AssignedTask;
  getPriorityColor: (priority: string) => string;
  getStatusIcon: (status: string) => React.ReactNode;
}) {
  return (
    <Link to={`/projects/${task.project_id}/tasks/${task.id}`}>
      <Card className="hover:bg-accent/50 transition-colors cursor-pointer">
        <CardContent className="py-3 px-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3 flex-1 min-w-0">
              {getStatusIcon(task.status)}
              <div className="flex-1 min-w-0">
                <p className="font-medium truncate">{task.title}</p>
                <p className="text-xs text-muted-foreground">{task.project_name}</p>
              </div>
            </div>
            <div className="flex items-center gap-2">
              {task.due_date && (
                <span className="text-xs text-muted-foreground">
                  {new Date(task.due_date).toLocaleDateString()}
                </span>
              )}
              <Badge className={cn('text-xs', getPriorityColor(task.priority))}>
                {task.priority || 'low'}
              </Badge>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </div>
          </div>
        </CardContent>
      </Card>
    </Link>
  );
}
