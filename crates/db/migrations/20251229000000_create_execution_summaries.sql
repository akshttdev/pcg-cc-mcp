-- Execution Summaries: Structured outcome data for agent work visibility
-- Captures what the agent did, enabling post-execution review and workflow learning

CREATE TABLE IF NOT EXISTS execution_summaries (
    id TEXT PRIMARY KEY NOT NULL,
    task_attempt_id TEXT NOT NULL REFERENCES task_attempts(id) ON DELETE CASCADE,
    execution_process_id TEXT REFERENCES execution_processes(id) ON DELETE SET NULL,

    -- What was accomplished
    files_modified INTEGER NOT NULL DEFAULT 0,
    files_created INTEGER NOT NULL DEFAULT 0,
    files_deleted INTEGER NOT NULL DEFAULT 0,
    commands_run INTEGER NOT NULL DEFAULT 0,
    commands_failed INTEGER NOT NULL DEFAULT 0,
    tools_used TEXT, -- JSON array of tool names e.g. ["Edit", "Bash", "Read"]

    -- Outcome tracking
    completion_status TEXT NOT NULL DEFAULT 'full' CHECK (completion_status IN ('full', 'partial', 'blocked', 'failed')),
    blocker_summary TEXT, -- Brief description if blocked/partial
    error_summary TEXT, -- Key error message if failed

    -- Timing metrics
    execution_time_ms INTEGER NOT NULL DEFAULT 0,

    -- Human feedback for workflow learning
    human_rating INTEGER CHECK (human_rating IS NULL OR (human_rating >= 1 AND human_rating <= 5)),
    human_notes TEXT,
    is_reference_example BOOLEAN NOT NULL DEFAULT FALSE, -- Marked as good reference for future workflows

    -- Workflow tagging for pattern analysis
    workflow_tags TEXT, -- JSON array e.g. ["api-integration", "frontend-styling"]

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for fast lookup by task attempt
CREATE INDEX IF NOT EXISTS idx_execution_summaries_task_attempt_id ON execution_summaries(task_attempt_id);

-- Index for finding reference examples
CREATE INDEX IF NOT EXISTS idx_execution_summaries_reference ON execution_summaries(is_reference_example) WHERE is_reference_example = TRUE;

-- Index for workflow pattern analysis
CREATE INDEX IF NOT EXISTS idx_execution_summaries_completion_status ON execution_summaries(completion_status);

-- Add collaborators tracking to tasks for showing who worked on a task
ALTER TABLE tasks ADD COLUMN collaborators TEXT; -- JSON array of {actor_id, actor_type, last_action, last_action_at}
