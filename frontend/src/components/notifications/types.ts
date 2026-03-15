export interface ActivityItem {
  id: string;
  task_id: string;
  actor_id: string;
  actor_type: string;
  action: string;
  previous_state: string | null;
  new_state: string | null;
  metadata: string | null;
  timestamp: string;
  actor_name: string | null;
}

export interface InboxNotification {
  id: string;
  user_id: string;
  organization_id: string | null;
  title: string;
  message: string;
  notification_type: string;
  source: string | null;
  source_id: string | null;
  read_at: string | null;
  created_at: string;
}
