import { Bell, Bot, CheckCircle2, Edit, Plus, Trash2, ArrowRight, MessageSquare, User, Cog } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import type { ActivityItem } from './types';
import { timeAgo, formatAction } from './utils';

/** Icon background colors vary by actor type for visual distinction. */
function getActorStyle(actorType: string) {
  switch (actorType) {
    case 'agent':
      return {
        badge: 'bg-violet-100 text-violet-700 dark:bg-violet-900/40 dark:text-violet-300',
        ring: 'ring-1 ring-violet-300 dark:ring-violet-700',
        label: 'Agent',
        Icon: Bot,
      };
    case 'system':
    case 'mcp':
      return {
        badge: 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400',
        ring: 'ring-1 ring-slate-300 dark:ring-slate-600',
        label: actorType === 'mcp' ? 'MCP' : 'System',
        Icon: Cog,
      };
    default: // human
      return {
        badge: 'bg-sky-100 text-sky-700 dark:bg-sky-900/40 dark:text-sky-300',
        ring: '',
        label: 'User',
        Icon: User,
      };
  }
}

function getActionIcon(action: string, actorType: string) {
  const { ring } = getActorStyle(actorType);
  const base = `flex items-center justify-center h-6 w-6 rounded-full ${ring}`;

  switch (action) {
    case 'created':
    case 'create':
    case 'task_created':
      return (
        <div className={`${base} bg-green-100 dark:bg-green-900/40`}>
          <Plus className="h-3.5 w-3.5 text-green-600 dark:text-green-400" />
        </div>
      );
    case 'updated':
    case 'update':
    case 'task_updated':
      return (
        <div className={`${base} bg-blue-100 dark:bg-blue-900/40`}>
          <Edit className="h-3.5 w-3.5 text-blue-600 dark:text-blue-400" />
        </div>
      );
    case 'status_change':
      return (
        <div className={`${base} bg-amber-100 dark:bg-amber-900/40`}>
          <ArrowRight className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400" />
        </div>
      );
    case 'complete':
    case 'done':
      return (
        <div className={`${base} bg-green-100 dark:bg-green-900/40`}>
          <CheckCircle2 className="h-3.5 w-3.5 text-green-600 dark:text-green-400" />
        </div>
      );
    case 'deleted':
    case 'delete':
      return (
        <div className={`${base} bg-red-100 dark:bg-red-900/40`}>
          <Trash2 className="h-3.5 w-3.5 text-red-600 dark:text-red-400" />
        </div>
      );
    case 'comment':
      return (
        <div className={`${base} bg-purple-100 dark:bg-purple-900/40`}>
          <MessageSquare className="h-3.5 w-3.5 text-purple-600 dark:text-purple-400" />
        </div>
      );
    default:
      return (
        <div className={`${base} bg-muted`}>
          <Bell className="h-3.5 w-3.5 text-muted-foreground" />
        </div>
      );
  }
}

interface ActivityNotificationItemProps {
  item: ActivityItem;
  onClick: (item: ActivityItem) => void;
  isRead?: boolean;
}

export function ActivityNotificationItem({ item, onClick, isRead }: ActivityNotificationItemProps) {
  const actorStyle = getActorStyle(item.actor_type);

  return (
    <div
      className={`px-3 py-2 transition-colors cursor-pointer hover:bg-muted/50 ${
        isRead ? 'opacity-50' : 'bg-blue-50/50 dark:bg-blue-950/20'
      }`}
      onClick={() => onClick(item)}
    >
      <div className="flex items-start gap-2">
        <div className="mt-0.5 shrink-0">{getActionIcon(item.action, item.actor_type)}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1.5">
            <p className="text-xs leading-tight">{formatAction(item)}</p>
            <span className={`inline-flex items-center gap-0.5 px-1.5 py-0.5 rounded text-[9px] font-medium shrink-0 ${actorStyle.badge}`}>
              <actorStyle.Icon className="h-2.5 w-2.5" />
              {actorStyle.label}
            </span>
          </div>
          <p className="text-[10px] text-muted-foreground mt-0.5">
            {timeAgo(item.timestamp)}
          </p>
          {item.action === 'status_change' && item.new_state && (
            <Badge variant="secondary" className="text-[9px] px-1 py-0 mt-0.5">
              &rarr; {item.new_state}
            </Badge>
          )}
        </div>
        {!isRead && (
          <div className="mt-1.5 h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />
        )}
      </div>
    </div>
  );
}
