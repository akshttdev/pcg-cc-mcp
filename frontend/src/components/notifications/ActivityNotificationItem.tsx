import { Bell, CheckCircle2, Edit, Plus, Trash2, ArrowRight, MessageSquare } from 'lucide-react';
import type { ActivityItem } from './types';
import { timeAgo, formatAction } from './utils';

function getActionIcon(action: string) {
  switch (action) {
    case 'created':
    case 'create':
    case 'task_created':
      return <Plus className="h-3 w-3 text-green-500" />;
    case 'updated':
    case 'update':
    case 'task_updated':
      return <Edit className="h-3 w-3 text-blue-500" />;
    case 'status_change':
      return <ArrowRight className="h-3 w-3 text-yellow-500" />;
    case 'complete':
    case 'done':
      return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case 'deleted':
    case 'delete':
      return <Trash2 className="h-3 w-3 text-red-500" />;
    case 'comment':
      return <MessageSquare className="h-3 w-3 text-purple-500" />;
    default:
      return <Bell className="h-3 w-3 text-muted-foreground" />;
  }
}

interface ActivityNotificationItemProps {
  item: ActivityItem;
  onClick: (item: ActivityItem) => void;
}

export function ActivityNotificationItem({ item, onClick }: ActivityNotificationItemProps) {
  return (
    <div
      className="px-3 py-2 transition-colors cursor-pointer hover:bg-muted/50 bg-blue-50/50 dark:bg-blue-950/20"
      onClick={() => onClick(item)}
    >
      <div className="flex items-start gap-2">
        <div className="mt-0.5 shrink-0">{getActionIcon(item.action)}</div>
        <div className="flex-1 min-w-0">
          <p className="text-xs leading-tight">{formatAction(item)}</p>
          <p className="text-[10px] text-muted-foreground mt-0.5">
            {timeAgo(item.timestamp)}
          </p>
        </div>
        <div className="mt-1.5 h-1.5 w-1.5 rounded-full bg-blue-500 shrink-0" />
      </div>
    </div>
  );
}
