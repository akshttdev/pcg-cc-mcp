import { useQuery } from '@tanstack/react-query';
import {
  AlertCircle,
  ArrowRight,
  CheckCircle2,
  Clock,
  Filter,
  FolderKanban,
  LayoutGrid,
  List,
  Search,
} from 'lucide-react';
import { useMemo,useState } from 'react';
import { Link } from 'react-router-dom';
import type { TaskWithAttemptStatus } from 'shared/types';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { CardGrid } from '@/components/ui/card-grid';
import { EmptyState } from '@/components/ui/empty-state';
import { Input } from '@/components/ui/input';
import { Loader } from '@/components/ui/loader';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { useProjectList } from '@/hooks/queries';
import { taskKeys } from '@/lib/query-keys';
import { cn } from '@/lib/utils';

interface GlobalTask extends TaskWithAttemptStatus {
  project_name: string;
}

export function GlobalTasksPage() {
  const [viewMode, setViewMode] = useState<'table' | 'cards'>('table');
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('all');
  const [priorityFilter, setPriorityFilter] = useState<string>('all');
  const [projectFilter, setProjectFilter] = useState<string>('all');

  const { data: projects = [], isLoading: projectsLoading } = useProjectList();

  const { data: allTasks = [], isLoading: tasksLoading } = useQuery<GlobalTask[]>({
    queryKey: taskKeys.global(projects.map(p => p.id)),
    queryFn: async () => {
      const taskPromises = projects.map(async (project) => {
        try {
          const response = await fetch(`/api/tasks?project_id=${project.id}`);
          if (!response.ok) return [];
          const data = await response.json();
          return (data.data || []).map((task: TaskWithAttemptStatus) => ({
            ...task,
            project_name: project.name,
          }));
        } catch {
          return [];
        }
      });
      const results = await Promise.all(taskPromises);
      return results.flat();
    },
    enabled: projects.length > 0,
  });

  const filteredTasks = useMemo(() => {
    return allTasks.filter((task) => {
      if (searchQuery) {
        const query = searchQuery.toLowerCase();
        if (
          !task.title.toLowerCase().includes(query) &&
          !task.project_name.toLowerCase().includes(query)
        ) {
          return false;
        }
      }
      if (statusFilter !== 'all' && task.status !== statusFilter) return false;
      if (priorityFilter !== 'all' && task.priority !== priorityFilter) return false;
      if (projectFilter !== 'all' && task.project_id !== projectFilter) return false;
      return true;
    });
  }, [allTasks, searchQuery, statusFilter, priorityFilter, projectFilter]);

  const stats = useMemo(() => {
    const total = allTasks.length;
    const todo = allTasks.filter(t => t.status === 'todo').length;
    const inProgress = allTasks.filter(t => t.status === 'inprogress').length;
    const completed = allTasks.filter(t => t.status === 'done').length;
    return { total, todo, inProgress, completed };
  }, [allTasks]);

  const getPriorityColor = (priority: string) => {
    switch (priority) {
      case 'critical':
      case 'high':
        return 'bg-red-100 text-red-800 border-red-200 dark:bg-red-950/40 dark:text-red-300 dark:border-red-800';
      case 'medium':
        return 'bg-yellow-100 text-yellow-800 border-yellow-200 dark:bg-yellow-950/40 dark:text-yellow-300 dark:border-yellow-800';
      default:
        return 'bg-green-100 text-green-800 border-green-200 dark:bg-green-950/40 dark:text-green-300 dark:border-green-800';
    }
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case 'done':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case 'inprogress':
        return <Clock className="h-4 w-4 text-blue-500" />;
      default:
        return <AlertCircle className="h-4 w-4 text-muted-foreground/50" />;
    }
  };

  const isLoading = projectsLoading || tasksLoading;

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader message="Loading tasks..." size={32} />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="relative border-b border-border/40 bg-card/50 backdrop-blur-sm overflow-hidden">
        <div className="ambient-glow -top-48 -right-32" />
        <div className="page-header px-4 sm:px-6 lg:px-8 py-4 max-w-[1600px] mx-auto relative">
          <div className="flex items-center gap-3">
            <div className="section-header-icon">
              <FolderKanban className="h-5 w-5" />
            </div>
            <div>
              <h1 className="page-title">Global Tasks</h1>
              <p className="page-description">
                {stats.total} total across {projects.length} projects
              </p>
            </div>
          </div>

          <div className="flex items-center gap-1 bg-surface-2 rounded-lg p-1">
            <Button
              variant={viewMode === 'table' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setViewMode('table')}
              className="h-7 w-7 p-0"
            >
              <List className="h-4 w-4" />
            </Button>
            <Button
              variant={viewMode === 'cards' ? 'secondary' : 'ghost'}
              size="sm"
              onClick={() => setViewMode('cards')}
              className="h-7 w-7 p-0"
            >
              <LayoutGrid className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </div>

      {/* Stats */}
      <div className="border-b border-border/40">
        <div className="stat-grid animate-stagger px-4 sm:px-6 lg:px-8 py-4 max-w-[1600px] mx-auto">
          <div className="stat-card">
            <FolderKanban className="stat-card-icon" />
            <div className="stat-card-value">{stats.total}</div>
            <div className="stat-card-label">Total Tasks</div>
          </div>
          <div className="stat-card">
            <AlertCircle className="stat-card-icon" />
            <div className="stat-card-value text-muted-foreground">{stats.todo}</div>
            <div className="stat-card-label">To Do</div>
          </div>
          <div className="stat-card">
            <Clock className="stat-card-icon" />
            <div className="stat-card-value text-info">{stats.inProgress}</div>
            <div className="stat-card-label">In Progress</div>
          </div>
          <div className="stat-card">
            <CheckCircle2 className="stat-card-icon" />
            <div className="stat-card-value text-success">{stats.completed}</div>
            <div className="stat-card-label">Completed</div>
          </div>
        </div>
      </div>

      {/* Filters */}
      <div className="border-b border-border/40">
        <div className="action-bar px-4 sm:px-6 lg:px-8 py-4 max-w-[1600px] mx-auto">
        <div className="relative flex-1 max-w-md">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            placeholder="Search tasks..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="pl-9"
          />
        </div>

        <div className="flex items-center gap-2 flex-wrap">
          <Select value={projectFilter} onValueChange={setProjectFilter}>
            <SelectTrigger className="w-[180px]">
              <Filter className="h-4 w-4 mr-2" />
              <SelectValue placeholder="All Projects" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Projects</SelectItem>
              {projects.map((project) => (
                <SelectItem key={project.id} value={project.id}>
                  {project.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Select value={statusFilter} onValueChange={setStatusFilter}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder="All Status" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Status</SelectItem>
              <SelectItem value="todo">To Do</SelectItem>
              <SelectItem value="inprogress">In Progress</SelectItem>
              <SelectItem value="done">Done</SelectItem>
            </SelectContent>
          </Select>

          <Select value={priorityFilter} onValueChange={setPriorityFilter}>
            <SelectTrigger className="w-[140px]">
              <SelectValue placeholder="All Priority" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="all">All Priority</SelectItem>
              <SelectItem value="critical">Critical</SelectItem>
              <SelectItem value="high">High</SelectItem>
              <SelectItem value="medium">Medium</SelectItem>
              <SelectItem value="low">Low</SelectItem>
            </SelectContent>
          </Select>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-4 sm:p-6 lg:p-8 max-w-[1600px] mx-auto w-full">
        {filteredTasks.length === 0 ? (
          <EmptyState
            icon={FolderKanban}
            title="No tasks found"
            description={searchQuery || statusFilter !== 'all' || priorityFilter !== 'all' || projectFilter !== 'all'
              ? 'Try adjusting your filters'
              : 'No tasks have been created yet'}
          />
        ) : viewMode === 'table' ? (
          <Card className="card-elevated overflow-hidden">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-10">Status</TableHead>
                  <TableHead>Title</TableHead>
                  <TableHead className="hidden md:table-cell">Project</TableHead>
                  <TableHead>Priority</TableHead>
                  <TableHead className="hidden sm:table-cell">Due Date</TableHead>
                  <TableHead className="text-right w-20">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredTasks.map((task) => (
                  <TableRow key={task.id} className="group">
                    <TableCell>{getStatusIcon(task.status)}</TableCell>
                    <TableCell>
                      <div className="font-medium">{task.title}</div>
                      <div className="text-xs text-muted-foreground md:hidden mt-0.5">
                        {task.project_name}
                      </div>
                    </TableCell>
                    <TableCell className="hidden md:table-cell">
                      <Badge variant="outline">{task.project_name}</Badge>
                    </TableCell>
                    <TableCell>
                      <Badge className={cn('text-xs', getPriorityColor(task.priority))}>
                        {task.priority}
                      </Badge>
                    </TableCell>
                    <TableCell className="hidden sm:table-cell">
                      {task.due_date ? (
                        <span className="text-sm text-muted-foreground">
                          {new Date(task.due_date).toLocaleDateString()}
                        </span>
                      ) : (
                        <span className="text-sm text-muted-foreground">-</span>
                      )}
                    </TableCell>
                    <TableCell className="text-right">
                      <Link to={task.project_id && task.project_id.length > 0 ? `/projects/${task.project_id}/tasks/${task.id}` : '/my-tasks'}>
                        <Button variant="ghost" size="sm" className="opacity-0 group-hover:opacity-100 transition-opacity">
                          View
                          <ArrowRight className="h-4 w-4 ml-1" />
                        </Button>
                      </Link>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </Card>
        ) : (
          <CardGrid columns={{ sm: 2, lg: 3 }} gap={4} className="animate-stagger">
            {filteredTasks.map((task) => (
              <Link
                key={task.id}
                to={task.project_id && task.project_id.length > 0 ? `/projects/${task.project_id}/tasks/${task.id}` : '/my-tasks'}
              >
                <Card className="card-interactive h-full">
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between">
                      <div className="flex items-center gap-2">
                        {getStatusIcon(task.status)}
                        <Badge variant="outline" className="text-xs">
                          {task.project_name}
                        </Badge>
                      </div>
                      <Badge className={cn('text-xs', getPriorityColor(task.priority))}>
                        {task.priority}
                      </Badge>
                    </div>
                    <CardTitle className="text-base mt-2">{task.title}</CardTitle>
                  </CardHeader>
                  <CardContent>
                    {task.description && (
                      <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
                        {task.description}
                      </p>
                    )}
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                      {task.due_date ? (
                        <span>Due: {new Date(task.due_date).toLocaleDateString()}</span>
                      ) : (
                        <span>No due date</span>
                      )}
                      <ArrowRight className="h-4 w-4" />
                    </div>
                  </CardContent>
                </Card>
              </Link>
            ))}
          </CardGrid>
        )}
      </div>
    </div>
  );
}
