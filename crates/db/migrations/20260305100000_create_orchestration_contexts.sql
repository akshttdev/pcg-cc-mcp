-- Orchestration Context: shared blackboard for multi-agent task execution.
-- Allows agents working on related tasks to share findings, decisions,
-- blockers, and intermediate results through a common context surface.

CREATE TABLE IF NOT EXISTS orchestration_contexts (
    id              BLOB PRIMARY KEY,
    root_task_id    BLOB NOT NULL,                          -- top-level task owning the context tree
    project_id      BLOB NOT NULL,
    task_id         BLOB,                                   -- specific subtask this entry relates to (optional)
    entry_type      TEXT NOT NULL CHECK(entry_type IN (
                        'finding',              -- discovered fact relevant to the task tree
                        'decision',             -- choice made that affects downstream work
                        'blocker',              -- issue preventing progress
                        'intermediate_result',  -- partial output from a subtask
                        'directive'             -- instruction from orchestrator to child agents
                    )),
    title           TEXT NOT NULL,
    content         TEXT NOT NULL,
    source          TEXT NOT NULL,                           -- agent name or user ID that wrote this
    status          TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'resolved', 'superseded')),
    priority        TEXT NOT NULL DEFAULT 'normal' CHECK(priority IN ('low', 'normal', 'high', 'critical')),
    resolved_by     TEXT,
    resolved_at     DATETIME,
    metadata        TEXT,                                    -- JSON for extensibility
    created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id),
    FOREIGN KEY (root_task_id) REFERENCES tasks(id)
);

-- Index for fast lookups by task tree
CREATE INDEX IF NOT EXISTS idx_orch_ctx_root_task ON orchestration_contexts(root_task_id);
CREATE INDEX IF NOT EXISTS idx_orch_ctx_project   ON orchestration_contexts(project_id);
CREATE INDEX IF NOT EXISTS idx_orch_ctx_task      ON orchestration_contexts(task_id);
CREATE INDEX IF NOT EXISTS idx_orch_ctx_type      ON orchestration_contexts(entry_type, status);

-- Trigger to auto-update updated_at
CREATE TRIGGER IF NOT EXISTS orch_ctx_updated_at
    AFTER UPDATE ON orchestration_contexts
BEGIN
    UPDATE orchestration_contexts SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
