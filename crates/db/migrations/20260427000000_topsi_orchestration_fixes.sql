-- Migration: Topsi Orchestration Fixes
-- 1. Add task_id / task_attempt_id to execution_artifacts for direct task linking
-- 2. Create workflow_interaction_logs for human+agent audit trail
-- 3. Indexes

-- ── 1. Link execution_artifacts → tasks ───────────────────────────────────────
ALTER TABLE execution_artifacts ADD COLUMN task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL;
ALTER TABLE execution_artifacts ADD COLUMN task_attempt_id TEXT REFERENCES task_attempts(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_execution_artifacts_task_id
    ON execution_artifacts(task_id) WHERE task_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_execution_artifacts_task_attempt_id
    ON execution_artifacts(task_attempt_id) WHERE task_attempt_id IS NOT NULL;

-- ── 2. Workflow interaction logs ───────────────────────────────────────────────
-- Unified audit trail: every human message, agent response, tool call, status
-- change, and cost event attached to a task or workflow.
CREATE TABLE IF NOT EXISTS workflow_interaction_logs (
    id          TEXT PRIMARY KEY,
    -- Context (at least one should be set)
    task_id     TEXT REFERENCES tasks(id) ON DELETE CASCADE,
    task_attempt_id TEXT REFERENCES task_attempts(id) ON DELETE CASCADE,
    workflow_run_id TEXT,                      -- workflow_runs.id (TEXT FK, no hard ref for flexibility)
    session_id  TEXT,                          -- Topsi / Nora conversation session

    -- Who acted
    actor_type  TEXT NOT NULL CHECK (actor_type IN ('human', 'agent', 'system')),
    actor_id    TEXT,                          -- user_id or agent short_name
    actor_name  TEXT,                          -- display label

    -- What happened
    action_type TEXT NOT NULL CHECK (action_type IN (
        'message',          -- human sent a message
        'response',         -- agent replied
        'tool_call',        -- agent invoked a tool
        'tool_result',      -- tool returned a result
        'task_created',     -- task was created
        'task_updated',     -- task status/field changed
        'artifact_created', -- artifact saved
        'cost_recorded',    -- VIBE usage recorded
        'execution_started',-- task attempt began
        'execution_ended',  -- task attempt finished
        'error'             -- something failed
    )),

    -- Content
    message     TEXT,                          -- human-readable summary / content
    metadata    TEXT,                          -- JSON: extra structured data

    -- Cost snapshot (filled for cost_recorded events)
    vibe_cost   REAL,
    input_tokens  INTEGER,
    output_tokens INTEGER,
    model_used  TEXT,

    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_wil_task_id
    ON workflow_interaction_logs(task_id, created_at DESC) WHERE task_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_wil_task_attempt_id
    ON workflow_interaction_logs(task_attempt_id) WHERE task_attempt_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_wil_session_id
    ON workflow_interaction_logs(session_id) WHERE session_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_wil_actor
    ON workflow_interaction_logs(actor_type, actor_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_wil_workflow_run_id
    ON workflow_interaction_logs(workflow_run_id) WHERE workflow_run_id IS NOT NULL;
