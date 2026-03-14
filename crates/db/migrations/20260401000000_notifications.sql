-- In-app notifications table
CREATE TABLE IF NOT EXISTS notifications (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    organization_id TEXT,
    title TEXT NOT NULL,
    message TEXT NOT NULL DEFAULT '',
    notification_type TEXT NOT NULL DEFAULT 'info',  -- info, warning, success, error
    source TEXT,           -- e.g. "workflow", "system", "task"
    source_id TEXT,        -- e.g. workflow_run_id or task_id
    read_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_notifications_user_id ON notifications(user_id);
CREATE INDEX IF NOT EXISTS idx_notifications_user_unread ON notifications(user_id, read_at);
CREATE INDEX IF NOT EXISTS idx_notifications_created_at ON notifications(created_at);
