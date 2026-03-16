import { CheckCircle2, Trash2, Info, AlertTriangle, ExternalLink, X } from 'lucide-react';
import type { InboxNotification } from './types';
import { timeAgo } from './utils';

function getNotificationIcon(type: string) {
  switch (type) {
    case 'success': return <CheckCircle2 className="h-3 w-3 text-green-500" />;
    case 'warning': return <AlertTriangle className="h-3 w-3 text-yellow-500" />;
    case 'error': return <Trash2 className="h-3 w-3 text-red-500" />;
    default: return <Info className="h-3 w-3 text-blue-500" />;
  }
}

interface InboxNotificationItemProps {
  item: InboxNotification;
  onClick: (item: InboxNotification) => void;
  onDismiss?: (item: InboxNotification) => void;
}

export function InboxNotificationItem({ item, onClick, onDismiss }: InboxNotificationItemProps) {
  return (
    <div
      className={`px-3 py-2 transition-colors cursor-pointer hover:bg-muted/50 ${
        item.read_at ? 'opacity-50' : 'bg-primary/5'
      }`}
      onClick={() => onClick(item)}
    >
      <div className="flex items-start gap-2">
        <div className="mt-0.5 shrink-0">{getNotificationIcon(item.notification_type)}</div>
        <div className="flex-1 min-w-0">
          <p className="text-xs font-medium leading-tight">{item.title}</p>
          {item.message && (
            <p className="text-[11px] text-muted-foreground mt-0.5 line-clamp-2">{item.message}</p>
          )}
          <p className="text-[10px] text-muted-foreground mt-0.5">
            {timeAgo(item.created_at)}
          </p>
          {item.source === 'task' && item.source_id && (
            <button
              className="inline-flex items-center gap-1 text-[10px] text-primary hover:underline mt-0.5"
              onClick={(e) => {
                e.stopPropagation();
                onClick(item);
              }}
            >
              <ExternalLink className="h-2.5 w-2.5" />
              View Task
            </button>
          )}
          {item.source === 'workflow' && item.source_id && (
            <button
              className="inline-flex items-center gap-1 text-[10px] text-primary hover:underline mt-0.5"
              onClick={(e) => {
                e.stopPropagation();
                onClick(item);
              }}
            >
              <ExternalLink className="h-2.5 w-2.5" />
              View Run
            </button>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          {!item.read_at && (
            <div className="mt-1.5 h-1.5 w-1.5 rounded-full bg-primary" />
          )}
          {onDismiss && (
            <button
              className="mt-0.5 p-0.5 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
              onClick={(e) => {
                e.stopPropagation();
                onDismiss(item);
              }}
              aria-label="Dismiss notification"
            >
              <X className="h-3 w-3" />
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
