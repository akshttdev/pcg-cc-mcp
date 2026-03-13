import { History, Circle } from 'lucide-react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';
import { useActivityStore } from '@/stores/useActivityStore';
import { formatTimeAgo } from '@/utils/timeUtils';
import type { ActivityType } from '@/types/activity';

interface ActivityFeedProps {
  taskId?: string;
  limit?: number;
  filterTypes?: ActivityType[];
  className?: string;
}

const activityIcons: Record<ActivityType, string> = {
  task_created: '✨',
  task_updated: '📝',
  task_deleted: '🗑️',
  status_changed: '🔄',
  priority_changed: '⚡',
  assignee_changed: '👤',
  comment_added: '💬',
  dependency_added: '🔗',
  dependency_removed: '🔓',
  time_logged: '⏱️',
  file_attached: '📎',
  agent_tool_call: '🤖',
  agent_workflow_triggered: '⚙️',
  agent_workflow_completed: '✅',
};

const activityColors: Record<ActivityType, string> = {
  task_created: 'bg-green-500',
  task_updated: 'bg-blue-500',
  task_deleted: 'bg-red-500',
  status_changed: 'bg-purple-500',
  priority_changed: 'bg-orange-500',
  assignee_changed: 'bg-cyan-500',
  comment_added: 'bg-yellow-500',
  dependency_added: 'bg-indigo-500',
  dependency_removed: 'bg-pink-500',
  time_logged: 'bg-teal-500',
  file_attached: 'bg-gray-500',
  agent_tool_call: 'bg-cyan-500',
  agent_workflow_triggered: 'bg-cyan-500',
  agent_workflow_completed: 'bg-cyan-500',
};

export function ActivityFeed({ taskId, limit = 50, filterTypes, className }: ActivityFeedProps) {
  const { getActivitiesForTask, getRecentActivities, getFilteredActivities } = useActivityStore();

  let activities = filterTypes
    ? getFilteredActivities({ types: filterTypes, taskIds: taskId ? [taskId] : undefined }).slice(0, limit)
    : taskId
      ? getActivitiesForTask(taskId)
      : getRecentActivities(limit);

  if (activities.length === 0) {
    return (
      <Card className={className}>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <History className="h-4 w-4" />
            Activity Feed
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground text-center py-4">
            No activity yet
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className={className}>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <History className="h-4 w-4" />
          Activity Feed ({activities.length})
        </CardTitle>
      </CardHeader>
      <CardContent>
        <ScrollArea className="h-[400px]">
          <div className="space-y-4 relative">
            {/* Timeline line */}
            <div className="absolute left-[9px] top-2 bottom-2 w-[2px] bg-border" />

            {activities.map((activity) => (
              <div key={activity.id} className="relative flex gap-3">
                {/* Timeline dot */}
                <div className="relative flex-shrink-0">
                  <div
                    className={`w-5 h-5 rounded-full flex items-center justify-center text-xs ${
                      activityColors[activity.type]
                    } text-white z-10 relative`}
                  >
                    <Circle className="h-2 w-2 fill-current" />
                  </div>
                </div>

                {/* Activity content */}
                <div className="flex-1 pb-4 min-w-0">
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-base">
                          {activityIcons[activity.type]}
                        </span>
                        <p className="text-sm font-medium">
                          {activity.description}
                        </p>
                      </div>
                      {activity.metadata && Object.keys(activity.metadata).length > 0 && (
                        <div className="mt-1 flex flex-wrap gap-1">
                          {Object.entries(activity.metadata).map(([key, value]) => (
                            <Badge key={key} variant="secondary" className="text-xs">
                              {key}: {String(value)}
                            </Badge>
                          ))}
                        </div>
                      )}
                    </div>
                    <span className="text-xs text-muted-foreground shrink-0">
                      {formatTimeAgo(new Date(activity.timestamp))}
                    </span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
