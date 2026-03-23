import { useMemo, useState } from 'react';
import { Button } from '@/components/ui/button';
import { EmptyState } from '@/components/ui/empty-state';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import {
  Mail,
  Phone,
  Calendar,
  FileText,
  ArrowRight,
  Trophy,
  XCircle,
  MessageSquare,
  Plus,
  Clock,
} from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { useCrmActivities } from '@/hooks/useCrmActivities';
import { CrmActivityForm } from './CrmActivityForm';
import type { CrmActivityRecord } from '@/lib/api';

interface CrmActivityTimelineProps {
  projectId: string;
  contactId?: string;
  dealId?: string;
  limit?: number;
}

const ACTIVITY_ICONS: Record<string, typeof Mail> = {
  email_sent: Mail,
  email_received: Mail,
  email_opened: Mail,
  email_clicked: Mail,
  call_made: Phone,
  call_received: Phone,
  call_scheduled: Phone,
  meeting_scheduled: Calendar,
  meeting_completed: Calendar,
  meeting_cancelled: Calendar,
  note_added: FileText,
  task_created: FileText,
  task_completed: FileText,
  deal_stage_changed: ArrowRight,
  deal_created: FileText,
  deal_won: Trophy,
  deal_lost: XCircle,
  social_mention: MessageSquare,
  social_dm: MessageSquare,
  social_comment: MessageSquare,
  custom: FileText,
};

const ACTIVITY_COLORS: Record<string, string> = {
  email_sent: 'bg-blue-100 text-blue-600',
  email_received: 'bg-blue-100 text-blue-600',
  call_made: 'bg-green-100 text-green-600',
  call_received: 'bg-green-100 text-green-600',
  meeting_scheduled: 'bg-purple-100 text-purple-600',
  meeting_completed: 'bg-purple-100 text-purple-600',
  note_added: 'bg-gray-100 text-gray-600',
  deal_stage_changed: 'bg-amber-100 text-amber-600',
  deal_created: 'bg-blue-100 text-blue-600',
  deal_won: 'bg-green-100 text-green-600',
  deal_lost: 'bg-red-100 text-red-600',
  social_mention: 'bg-pink-100 text-pink-600',
};

function groupByDate(activities: CrmActivityRecord[]): Map<string, CrmActivityRecord[]> {
  const groups = new Map<string, CrmActivityRecord[]>();
  for (const activity of activities) {
    const date = new Date(activity.activity_at).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'long',
      day: 'numeric',
    });
    if (!groups.has(date)) groups.set(date, []);
    groups.get(date)!.push(activity);
  }
  return groups;
}

export function CrmActivityTimeline({
  projectId,
  contactId,
  dealId,
  limit = 50,
}: CrmActivityTimelineProps) {
  const [formOpen, setFormOpen] = useState(false);

  const { data: activities = [], isLoading } = useCrmActivities({
    projectId,
    contactId,
    dealId,
    limit,
  });

  const grouped = useMemo(() => groupByDate(activities), [activities]);

  if (isLoading) {
    return (
      <div className="space-y-4">
        {[1, 2, 3].map((i) => (
          <div key={i} className="flex gap-3">
            <Skeleton className="h-8 w-8 rounded-full shrink-0" />
            <div className="flex-1 space-y-2">
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-3 w-1/2" />
            </div>
          </div>
        ))}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium">Activity Timeline</h3>
        <Button variant="outline" size="sm" onClick={() => setFormOpen(true)}>
          <Plus className="h-3 w-3 mr-1" />
          Log Activity
        </Button>
      </div>

      {activities.length === 0 ? (
        <EmptyState
          icon={Clock}
          title="No activity recorded yet"
          className="py-8"
        />
      ) : (
        <div className="space-y-6">
          {Array.from(grouped.entries()).map(([date, dateActivities]) => (
            <div key={date}>
              <div className="text-xs font-medium text-muted-foreground mb-3">
                {date}
              </div>
              <div className="space-y-3">
                {dateActivities.map((activity) => (
                  <ActivityItem key={activity.id} activity={activity} />
                ))}
              </div>
            </div>
          ))}
        </div>
      )}

      <CrmActivityForm
        open={formOpen}
        onOpenChange={setFormOpen}
        projectId={projectId}
        contactId={contactId}
        dealId={dealId}
      />
    </div>
  );
}

function ActivityItem({ activity }: { activity: CrmActivityRecord }) {
  const Icon = ACTIVITY_ICONS[activity.activity_type] || FileText;
  const colorClass = ACTIVITY_COLORS[activity.activity_type] || 'bg-gray-100 text-gray-600';

  const formatType = (type: string) => {
    return type
      .split('_')
      .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
      .join(' ');
  };

  return (
    <div className="flex gap-3 items-start">
      <div className={`p-1.5 rounded-full shrink-0 ${colorClass}`}>
        <Icon className="h-3.5 w-3.5" />
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium truncate">
            {activity.subject || formatType(activity.activity_type)}
          </span>
          <Badge variant="outline" className="text-[10px] px-1.5 py-0 shrink-0">
            {formatDistanceToNow(new Date(activity.activity_at), { addSuffix: true })}
          </Badge>
        </div>
        {activity.description && (
          <p className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
            {activity.description}
          </p>
        )}
        {activity.outcome && (
          <p className="text-xs text-muted-foreground mt-0.5">
            Outcome: {activity.outcome}
          </p>
        )}
        {activity.duration_minutes && (
          <span className="text-[10px] text-muted-foreground">
            {activity.duration_minutes} min
          </span>
        )}
      </div>
    </div>
  );
}
