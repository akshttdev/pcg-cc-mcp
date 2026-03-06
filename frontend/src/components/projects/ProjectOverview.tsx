import { useMemo } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  CheckCircle2,
  Clock,
  AlertTriangle,
  ListTodo,
  Users,
  BarChart3,
  Calendar,
} from 'lucide-react';
import type { TaskWithAttemptStatus, Project } from 'shared/types';

interface ProjectOverviewProps {
  project: Project;
  tasks: TaskWithAttemptStatus[];
  members?: { username: string; full_name: string; role: string }[];
}

export function ProjectOverview({ project, tasks, members }: ProjectOverviewProps) {
  const stats = useMemo(() => {
    const total = tasks.length;
    const done = tasks.filter((t) => t.status === 'done').length;
    const inProgress = tasks.filter((t) => t.status === 'inprogress').length;
    const inReview = tasks.filter((t) => t.status === 'inreview').length;
    const todo = tasks.filter((t) => t.status === 'todo').length;
    const cancelled = tasks.filter((t) => t.status === 'cancelled').length;
    const active = total - done - cancelled;
    const progress = total > 0 ? Math.round((done / (total - cancelled || 1)) * 100) : 0;

    const now = new Date();
    const overdue = tasks.filter(
      (t) =>
        t.due_date &&
        new Date(t.due_date) < now &&
        t.status !== 'done' &&
        t.status !== 'cancelled'
    ).length;
    const dueSoon = tasks.filter((t) => {
      if (!t.due_date || t.status === 'done' || t.status === 'cancelled') return false;
      const due = new Date(t.due_date);
      const diff = (due.getTime() - now.getTime()) / (1000 * 60 * 60 * 24);
      return diff >= 0 && diff <= 3;
    }).length;

    const critical = tasks.filter(
      (t) => t.priority === 'critical' && t.status !== 'done' && t.status !== 'cancelled'
    ).length;
    const high = tasks.filter(
      (t) => t.priority === 'high' && t.status !== 'done' && t.status !== 'cancelled'
    ).length;

    return { total, done, inProgress, inReview, todo, cancelled, active, progress, overdue, dueSoon, critical, high };
  }, [tasks]);

  return (
    <div className="p-6 space-y-6 max-w-5xl mx-auto">
      {/* Progress bar */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <h2 className="text-lg font-semibold">{project.name}</h2>
          <span className="text-sm text-muted-foreground">{stats.progress}% complete</span>
        </div>
        <div className="w-full bg-muted rounded-full h-2.5">
          <div
            className="bg-green-500 h-2.5 rounded-full transition-all"
            style={{ width: `${stats.progress}%` }}
          />
        </div>
        <p className="text-xs text-muted-foreground mt-1">
          {stats.done} of {stats.total - stats.cancelled} tasks completed
        </p>
      </div>

      {/* Stat cards */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
        <Card>
          <CardContent className="pt-4 pb-3 px-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-blue-50 dark:bg-blue-950/40">
                <ListTodo className="h-4 w-4 text-blue-600 dark:text-blue-400" />
              </div>
              <div>
                <p className="text-2xl font-bold">{stats.todo}</p>
                <p className="text-xs text-muted-foreground">To Do</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4 pb-3 px-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-yellow-50 dark:bg-yellow-950/40">
                <Clock className="h-4 w-4 text-yellow-600 dark:text-yellow-400" />
              </div>
              <div>
                <p className="text-2xl font-bold">{stats.inProgress + stats.inReview}</p>
                <p className="text-xs text-muted-foreground">In Progress</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4 pb-3 px-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-green-50 dark:bg-green-950/40">
                <CheckCircle2 className="h-4 w-4 text-green-600 dark:text-green-400" />
              </div>
              <div>
                <p className="text-2xl font-bold">{stats.done}</p>
                <p className="text-xs text-muted-foreground">Done</p>
              </div>
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-4 pb-3 px-4">
            <div className="flex items-center gap-3">
              <div className="p-2 rounded-lg bg-red-50 dark:bg-red-950/40">
                <AlertTriangle className="h-4 w-4 text-red-600 dark:text-red-400" />
              </div>
              <div>
                <p className="text-2xl font-bold">{stats.overdue}</p>
                <p className="text-xs text-muted-foreground">Overdue</p>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      {/* Attention needed */}
      {(stats.critical > 0 || stats.high > 0 || stats.overdue > 0 || stats.dueSoon > 0) && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium">Needs Attention</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {stats.overdue > 0 && (
              <div className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
                <Calendar className="h-4 w-4" />
                <span>{stats.overdue} overdue task{stats.overdue > 1 ? 's' : ''}</span>
              </div>
            )}
            {stats.dueSoon > 0 && (
              <div className="flex items-center gap-2 text-sm text-amber-600 dark:text-amber-400">
                <Clock className="h-4 w-4" />
                <span>{stats.dueSoon} task{stats.dueSoon > 1 ? 's' : ''} due in the next 3 days</span>
              </div>
            )}
            {stats.critical > 0 && (
              <div className="flex items-center gap-2 text-sm text-red-600 dark:text-red-400">
                <AlertTriangle className="h-4 w-4" />
                <span>{stats.critical} critical priority task{stats.critical > 1 ? 's' : ''}</span>
              </div>
            )}
            {stats.high > 0 && (
              <div className="flex items-center gap-2 text-sm text-orange-600 dark:text-orange-400">
                <BarChart3 className="h-4 w-4" />
                <span>{stats.high} high priority task{stats.high > 1 ? 's' : ''}</span>
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* Team */}
      {members && members.length > 0 && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium flex items-center gap-2">
              <Users className="h-4 w-4" />
              Team ({members.length})
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-3">
              {members.map((m) => (
                <div
                  key={m.username}
                  className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-muted text-sm"
                >
                  <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center text-xs font-medium text-primary">
                    {m.full_name.charAt(0).toUpperCase()}
                  </div>
                  <div>
                    <span className="font-medium">{m.full_name}</span>
                    <span className="text-xs text-muted-foreground ml-1">({m.role})</span>
                  </div>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      )}

      {/* Status breakdown */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-medium">Task Breakdown</CardTitle>
        </CardHeader>
        <CardContent>
          <div className="space-y-3">
            {[
              { label: 'To Do', count: stats.todo, color: 'bg-slate-400', total: stats.total },
              { label: 'In Progress', count: stats.inProgress, color: 'bg-blue-500', total: stats.total },
              { label: 'In Review', count: stats.inReview, color: 'bg-yellow-500', total: stats.total },
              { label: 'Done', count: stats.done, color: 'bg-green-500', total: stats.total },
            ].map((row) => (
              <div key={row.label} className="flex items-center gap-3">
                <span className="text-xs text-muted-foreground w-20">{row.label}</span>
                <div className="flex-1 bg-muted rounded-full h-2">
                  <div
                    className={`${row.color} h-2 rounded-full transition-all`}
                    style={{ width: `${row.total > 0 ? (row.count / row.total) * 100 : 0}%` }}
                  />
                </div>
                <span className="text-xs font-medium w-6 text-right">{row.count}</span>
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
