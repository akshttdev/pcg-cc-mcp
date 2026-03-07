-- Phase 2: Multi-session support per task attempt
-- Each agent_session represents an independent agent conversation thread

CREATE TABLE IF NOT EXISTS agent_sessions (
    id              BLOB PRIMARY KEY NOT NULL,
    task_attempt_id BLOB NOT NULL REFERENCES task_attempts(id) ON DELETE CASCADE,
    executor        TEXT,
    agent_working_dir TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_agent_sessions_task_attempt_id ON agent_sessions(task_attempt_id);

-- Migrate: create one agent_session per existing task_attempt
INSERT INTO agent_sessions (id, task_attempt_id, executor, created_at, updated_at)
SELECT
    randomblob(16),
    ta.id,
    ta.executor,
    ta.created_at,
    ta.updated_at
FROM task_attempts ta;

-- Add session_id to execution_processes for multi-session support
ALTER TABLE execution_processes ADD COLUMN agent_session_id BLOB REFERENCES agent_sessions(id) ON DELETE SET NULL;

-- Backfill agent_session_id on execution_processes from the migrated sessions
UPDATE execution_processes
SET agent_session_id = (
    SELECT s.id FROM agent_sessions s
    WHERE s.task_attempt_id = execution_processes.task_attempt_id
    LIMIT 1
);

CREATE INDEX IF NOT EXISTS idx_execution_processes_agent_session_id ON execution_processes(agent_session_id);
