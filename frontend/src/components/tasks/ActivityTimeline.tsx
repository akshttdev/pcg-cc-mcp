import { useQuery } from '@tanstack/react-query';
import { activityApi } from '@/lib/api';
import type { ActivityLog, ActorType } from 'shared/types';
import { format } from 'date-fns';
import { Card, CardContent, CardHeader } from '@/components/ui/card';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Loader } from '@/components/ui/loader';
import { cn } from '@/lib/utils';
import {
  User,
  Bot,
  Server,
  MessageSquare,
  CheckCircle,
  XCircle,
  Clock,
  Edit,
  UserPlus,
  Tag,
  AlertCircle,
  ArrowRight,
  Coins,
} from 'lucide-react';

interface ActivityTimelineProps {
  taskId: string;
}

const ACTOR_TYPE_CONFIG: Record<ActorType, { icon: typeof User; color: string }> = {
  human: { icon: User, color: 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200' },
  agent: { icon: Bot, color: 'bg-purple-100 text-purple-800 dark:bg-purple-900 dark:text-purple-200' },
  mcp: { icon: Server, color: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-200' },
  system: { icon: MessageSquare, color: 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200' },
};

const ACTION_ICON_MAP: Record<string, typeof CheckCircle> = {
  created: CheckCircle,
  updated: Edit,
  'status_changed': ArrowRight,
  assigned: UserPlus,
  'priority_changed': AlertCircle,
  commented: MessageSquare,
  approved: CheckCircle,
  rejected: XCircle,
  'changes_requested': Clock,
  tagged: Tag,
};

interface StateChange {
  field: string;
  from: string;
  to: string;
}

function parseStateChanges(activity: ActivityLog): StateChange[] {
  const changes: StateChange[] = [];

  try {
    const prevState = activity.previous_state ? JSON.parse(activity.previous_state) : {};
    const newState = activity.new_state ? JSON.parse(activity.new_state) : {};

    const allKeys = new Set([...Object.keys(prevState), ...Object.keys(newState)]);

    allKeys.forEach((key) => {
      if (prevState[key] !== newState[key]) {
        changes.push({
          field: key,
          from: prevState[key] || 'none',
          to: newState[key] || 'none',
        });
      }
    });
  } catch (e) {
    console.error('Failed to parse state changes:', e);
  }

  return changes;
}

function formatActionText(activity: ActivityLog): string {
  const action = activity.action.toLowerCase();

  switch (action) {
    case 'created':
      return 'created this task';
    case 'updated':
      return 'updated this task';
    case 'status_changed':
      return 'changed the status';
    case 'assigned':
      return 'assigned this task';
    case 'priority_changed':
      return 'changed the priority';
    case 'commented':
      return 'added a comment';
    case 'approved':
      return 'approved this task';
    case 'rejected':
      return 'rejected this task';
    case 'changes_requested':
      return 'requested changes';
    case 'tagged':
      return 'updated tags';
    default:
      return activity.action;
  }
}

interface ParsedMetadata {
  message?: string;
  vibe_cost?: number;
  planning_vibe_cost?: number;
  execution_vibe_cost?: number;
  verification_vibe_cost?: number;
  breakdown?: { planning?: number; execution?: number };
  [key: string]: unknown;
}

function parseMetadata(activity: ActivityLog): ParsedMetadata | null {
  if (!activity.metadata) return null;
  try {
    return JSON.parse(activity.metadata) as ParsedMetadata;
  } catch {
    return null;
  }
}

function getVibeCost(metadata: ParsedMetadata | null): number | null {
  if (!metadata) return null;
  return metadata.vibe_cost ??
         metadata.planning_vibe_cost ??
         metadata.execution_vibe_cost ??
         metadata.verification_vibe_cost ??
         (metadata.breakdown ? (metadata.breakdown.planning ?? 0) + (metadata.breakdown.execution ?? 0) : null);
}

export function ActivityTimeline({ taskId }: ActivityTimelineProps) {
  const { data: activities = [], isLoading, error } = useQuery({
    queryKey: ['taskActivity', taskId],
    queryFn: () => activityApi.getAll(taskId),
    refetchInterval: 10000,
  });

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
        Failed to load activity
      </div>
    );
  }

  const sortedActivities = [...activities].sort(
    (a, b) => new Date(b.timestamp).getTime() - new Date(a.timestamp).getTime()
  );

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold">
        Activity ({activities.length})
      </h3>

      {activities.length === 0 ? (
        <div className="text-center py-8 text-muted-foreground">
          No activity yet
        </div>
      ) : (
        <ScrollArea className="h-[500px]">
          <div className="space-y-4 pr-4">
            {sortedActivities.map((activity, index) => {
              const actorConfig = ACTOR_TYPE_CONFIG[activity.actor_type];
              const ActorIcon = actorConfig.icon;
              const ActionIcon = ACTION_ICON_MAP[activity.action.toLowerCase()] || Edit;
              const stateChanges = parseStateChanges(activity);
              const metadata = parseMetadata(activity);
              const vibeCost = getVibeCost(metadata);

              return (
                <div key={activity.id} className="relative">
                  {index < sortedActivities.length - 1 && (
                    <div className="absolute left-4 top-12 bottom-0 w-px bg-border" />
                  )}

                  <Card>
                    <CardHeader className="pb-3">
                      <div className="flex items-start gap-3">
                        <div className="relative">
                          <Avatar className="h-8 w-8">
                            <AvatarFallback className={cn('text-xs', actorConfig.color)}>
                              <ActorIcon className="h-4 w-4" />
                            </AvatarFallback>
                          </Avatar>
                          <div className="absolute -bottom-1 -right-1 bg-background rounded-full p-0.5">
                            <ActionIcon className="h-3 w-3 text-muted-foreground" />
                          </div>
                        </div>

                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 flex-wrap">
                            <span className="font-semibold text-sm">{activity.actor_id}</span>
                            <span className="text-sm text-muted-foreground">
                              {formatActionText(activity)}
                            </span>
                            <Badge variant="outline" className={cn('text-xs', actorConfig.color)}>
                              {activity.actor_type}
                            </Badge>
                          </div>
                          <div className="text-xs text-muted-foreground mt-1">
                            {format(new Date(activity.timestamp), 'MMM d, yyyy h:mm a')}
                          </div>
                        </div>
                      </div>
                    </CardHeader>

                    {(stateChanges.length > 0 || metadata) && (
                      <CardContent className="pt-0">
                        {/* Display message from metadata */}
                        {metadata?.message && (
                          <div className="text-sm text-foreground mb-3 bg-muted/30 rounded-md px-3 py-2">
                            {metadata.message}
                          </div>
                        )}

                        {/* Display Vibe cost prominently */}
                        {vibeCost !== null && vibeCost > 0 && (
                          <div className="flex items-center gap-2 mb-3">
                            <Badge variant="secondary" className="bg-amber-100 text-amber-800 dark:bg-amber-900 dark:text-amber-200 flex items-center gap-1">
                              <Coins className="h-3 w-3" />
                              {vibeCost.toLocaleString()} VIBE
                            </Badge>
                            <span className="text-xs text-muted-foreground">
                              (${(vibeCost * 0.001).toFixed(2)} USD)
                            </span>
                          </div>
                        )}

                        {/* Display breakdown if available */}
                        {metadata?.breakdown && (
                          <div className="flex gap-2 mb-3 text-xs">
                            {metadata.breakdown.planning && (
                              <Badge variant="outline" className="text-purple-600 dark:text-purple-400">
                                Planning: {metadata.breakdown.planning.toLocaleString()} VIBE
                              </Badge>
                            )}
                            {metadata.breakdown.execution && (
                              <Badge variant="outline" className="text-blue-600 dark:text-blue-400">
                                Execution: {metadata.breakdown.execution.toLocaleString()} VIBE
                              </Badge>
                            )}
                          </div>
                        )}

                        {/* State changes */}
                        {stateChanges.length > 0 && (
                          <div className="space-y-2">
                            {stateChanges.map((change, i) => (
                              <div
                                key={i}
                                className="flex items-center gap-2 text-sm bg-muted/50 rounded px-2 py-1.5"
                              >
                                <span className="font-medium text-muted-foreground capitalize">
                                  {change.field}:
                                </span>
                                <Badge variant="outline" className="text-xs">
                                  {typeof change.from === 'object' ? JSON.stringify(change.from) : String(change.from)}
                                </Badge>
                                <ArrowRight className="h-3 w-3 text-muted-foreground" />
                                <Badge variant="outline" className="text-xs">
                                  {typeof change.to === 'object' ? JSON.stringify(change.to) : String(change.to)}
                                </Badge>
                              </div>
                            ))}
                          </div>
                        )}
                      </CardContent>
                    )}
                  </Card>
                </div>
              );
            })}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
