-- Phase 1: Workspace enhancements on task_attempts
-- Adds archiving, pinning, naming, and seen tracking

-- Add workspace-style flags to task_attempts
ALTER TABLE task_attempts ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE task_attempts ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0;
ALTER TABLE task_attempts ADD COLUMN name TEXT;
ALTER TABLE task_attempts ADD COLUMN seen_at TEXT;

-- Auto-archive attempts whose parent tasks are done or cancelled
UPDATE task_attempts
SET archived = 1
WHERE task_id IN (
    SELECT id FROM tasks WHERE status IN ('done', 'cancelled')
);

-- Index for filtering archived/pinned
CREATE INDEX IF NOT EXISTS idx_task_attempts_archived ON task_attempts(archived);
CREATE INDEX IF NOT EXISTS idx_task_attempts_pinned ON task_attempts(pinned);
