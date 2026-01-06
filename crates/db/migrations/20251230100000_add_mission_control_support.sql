-- Migration: Add Mission Control Support
-- Enables structured artifact tracking and agent task plan visualization

-- Structured artifacts from executions
CREATE TABLE IF NOT EXISTS execution_artifacts (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL CHECK (artifact_type IN (
        'plan', 'screenshot', 'walkthrough', 'diff_summary', 'test_result', 'checkpoint', 'error_report'
    )),
    title TEXT NOT NULL,
    content TEXT,
    file_path TEXT,
    metadata TEXT, -- JSON for flexible additional data
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Agent task plans for step-by-step tracking
CREATE TABLE IF NOT EXISTS agent_task_plans (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    plan_json TEXT NOT NULL, -- JSON array of plan steps
    current_step INTEGER DEFAULT 0,
    total_steps INTEGER,
    status TEXT NOT NULL DEFAULT 'planning' CHECK (status IN ('planning', 'executing', 'completed', 'failed', 'paused')),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_process
    ON execution_artifacts(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_execution_artifacts_type
    ON execution_artifacts(artifact_type);

CREATE INDEX IF NOT EXISTS idx_agent_task_plans_process
    ON agent_task_plans(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_agent_task_plans_status
    ON agent_task_plans(status);
