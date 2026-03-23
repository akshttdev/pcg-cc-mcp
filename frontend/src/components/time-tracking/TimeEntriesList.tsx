import { Clock, Trash2 } from 'lucide-react';
import { toast } from 'sonner';

import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { IconButton } from '@/components/ui/icon-button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { useTimeTrackingStore } from '@/stores/useTimeTrackingStore';
import { formatDuration, formatTimeAgo } from '@/utils/timeUtils';

interface TimeEntriesListProps {
  taskId: string;
}

export function TimeEntriesList({ taskId }: TimeEntriesListProps) {
  const { getEntriesByTask, deleteEntry } = useTimeTrackingStore();
  const entries = getEntriesByTask(taskId).sort(
    (a, b) => b.startTime.getTime() - a.startTime.getTime()
  );

  const handleDelete = (id: string) => {
    if (confirm('Delete this time entry?')) {
      deleteEntry(id);
      toast.success('Time entry deleted');
    }
  };

  if (entries.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="text-base flex items-center gap-2">
            <Clock className="h-4 w-4" />
            Time Entries
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground text-center py-4">
            No time entries yet. Start the timer to track your work.
          </p>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base flex items-center gap-2">
          <Clock className="h-4 w-4" />
          Time Entries ({entries.length})
        </CardTitle>
      </CardHeader>
      <CardContent>
        <ScrollArea className="h-[300px]">
          <div className="space-y-2">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="flex items-center justify-between p-3 rounded-lg border hover:bg-muted/50 transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm">
                      {formatDuration(entry.duration || 0)}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {formatTimeAgo(entry.startTime)}
                    </span>
                  </div>
                  {entry.description && (
                    <p className="text-xs text-muted-foreground truncate mt-1">
                      {entry.description}
                    </p>
                  )}
                  <div className="flex items-center gap-2 mt-1 text-xs text-muted-foreground">
                    <span>{entry.startTime.toLocaleTimeString()}</span>
                    {entry.endTime && (
                      <>
                        <span>→</span>
                        <span>{entry.endTime.toLocaleTimeString()}</span>
                      </>
                    )}
                  </div>
                </div>
                <IconButton
                  variant="ghost" className="h-7 w-7 shrink-0"
                  onClick={() => handleDelete(entry.id)}
                  icon={Trash2}
                  label="Delete time entry"
                  iconClassName="h-3 w-3"
                />
              </div>
            ))}
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
