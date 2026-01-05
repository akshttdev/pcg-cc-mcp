-- Migration: Add Parallel Execution Support
-- Enables multiple TaskAttempts to execute concurrently with slot-based resource management

-- Execution slot tracking for resource management
CREATE TABLE IF NOT EXISTS execution_slots (
    id BLOB PRIMARY KEY,
    task_attempt_id BLOB NOT NULL REFERENCES task_attempts(id) ON DELETE CASCADE,
    slot_type TEXT NOT NULL CHECK (slot_type IN ('coding_agent', 'browser_agent', 'script')),
    resource_weight INTEGER NOT NULL DEFAULT 1,
    acquired_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    released_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Project capacity limits
ALTER TABLE projects ADD COLUMN max_concurrent_agents INTEGER DEFAULT 3;
ALTER TABLE projects ADD COLUMN max_concurrent_browser_agents INTEGER DEFAULT 1;

-- Process slot tracking and priority
ALTER TABLE execution_processes ADD COLUMN slot_id BLOB REFERENCES execution_slots(id);
ALTER TABLE execution_processes ADD COLUMN priority INTEGER DEFAULT 0;

-- Indexes for efficient slot queries
CREATE INDEX IF NOT EXISTS idx_execution_slots_active
    ON execution_slots(task_attempt_id) WHERE released_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_execution_slots_type_active
    ON execution_slots(slot_type) WHERE released_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_execution_processes_running
    ON execution_processes(status) WHERE status = 'running';

CREATE INDEX IF NOT EXISTS idx_execution_processes_slot
    ON execution_processes(slot_id) WHERE slot_id IS NOT NULL;
