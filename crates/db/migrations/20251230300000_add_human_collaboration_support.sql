-- Migration: Add Human-Agent Collaboration Support
-- Enables pause/resume, context injection, and handoff between humans and agents

-- Add control state columns to execution_processes
ALTER TABLE execution_processes ADD COLUMN control_state TEXT DEFAULT 'running'
    CHECK (control_state IN ('running', 'paused', 'human_takeover', 'awaiting_input'));
ALTER TABLE execution_processes ADD COLUMN paused_at TEXT;
ALTER TABLE execution_processes ADD COLUMN pause_reason TEXT;

-- Context injections (human notes mid-execution)
-- Allows humans to add notes, corrections, or directives during agent execution
CREATE TABLE IF NOT EXISTS context_injections (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    injector_id TEXT NOT NULL,
    injector_name TEXT,
    injection_type TEXT NOT NULL CHECK (injection_type IN (
        'note', 'correction', 'approval', 'rejection', 'directive', 'question', 'answer'
    )),
    content TEXT NOT NULL,
    metadata TEXT, -- JSON for additional context
    acknowledged INTEGER NOT NULL DEFAULT 0,
    acknowledged_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Execution handoffs between humans and agents
-- Records when control is transferred between actors
CREATE TABLE IF NOT EXISTS execution_handoffs (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    from_actor_type TEXT NOT NULL CHECK (from_actor_type IN ('agent', 'human', 'system')),
    from_actor_id TEXT NOT NULL,
    from_actor_name TEXT,
    to_actor_type TEXT NOT NULL CHECK (to_actor_type IN ('agent', 'human', 'system')),
    to_actor_id TEXT NOT NULL,
    to_actor_name TEXT,
    handoff_type TEXT NOT NULL CHECK (handoff_type IN (
        'takeover', 'return', 'escalation', 'delegation', 'assistance', 'review_request'
    )),
    reason TEXT,
    context_snapshot TEXT, -- JSON snapshot of state at handoff
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Pause/resume history for audit trail
CREATE TABLE IF NOT EXISTS execution_pause_history (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    action TEXT NOT NULL CHECK (action IN ('pause', 'resume')),
    reason TEXT,
    initiated_by TEXT NOT NULL,
    initiated_by_name TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_context_injections_execution
    ON context_injections(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_context_injections_unacknowledged
    ON context_injections(execution_process_id, acknowledged)
    WHERE acknowledged = 0;

CREATE INDEX IF NOT EXISTS idx_execution_handoffs_execution
    ON execution_handoffs(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_execution_handoffs_to_actor
    ON execution_handoffs(to_actor_type, to_actor_id);

CREATE INDEX IF NOT EXISTS idx_execution_pause_history_execution
    ON execution_pause_history(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_execution_processes_control_state
    ON execution_processes(control_state)
    WHERE control_state != 'running';
