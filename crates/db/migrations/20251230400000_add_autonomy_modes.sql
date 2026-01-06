-- Migration: Add Autonomy Modes Support
-- Enables configurable autonomy levels with checkpoint verification and approval gates

-- Add autonomy mode to tasks
ALTER TABLE tasks ADD COLUMN autonomy_mode TEXT DEFAULT 'agent_assisted'
    CHECK (autonomy_mode IN ('agent_driven', 'agent_assisted', 'review_driven'));

-- Checkpoint definitions (reusable checkpoint templates)
CREATE TABLE IF NOT EXISTS checkpoint_definitions (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    checkpoint_type TEXT NOT NULL CHECK (checkpoint_type IN (
        'file_change', 'external_call', 'cost_threshold', 'time_threshold', 'custom'
    )),
    -- Configuration for the checkpoint type (JSON)
    -- e.g., {"max_files": 10} for file_change, {"max_cost": 5.00} for cost_threshold
    config TEXT NOT NULL DEFAULT '{}',
    requires_approval INTEGER NOT NULL DEFAULT 1,
    -- Auto-approve after X minutes (NULL = never auto-approve)
    auto_approve_after_minutes INTEGER,
    -- Priority: higher = checked first
    priority INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Checkpoint instances (actual checkpoints triggered during execution)
CREATE TABLE IF NOT EXISTS execution_checkpoints (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    checkpoint_definition_id BLOB REFERENCES checkpoint_definitions(id),
    -- Checkpoint data captured at trigger time (JSON)
    checkpoint_data TEXT NOT NULL,
    -- What triggered this checkpoint
    trigger_reason TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'auto_approved', 'skipped', 'expired')),
    reviewer_id TEXT,
    reviewer_name TEXT,
    review_note TEXT,
    reviewed_at TEXT,
    -- When this checkpoint expires (for auto-approval or timeout)
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Approval gates (define when approvals are required)
CREATE TABLE IF NOT EXISTS approval_gates (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE CASCADE,
    task_id BLOB REFERENCES tasks(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    gate_type TEXT NOT NULL CHECK (gate_type IN (
        'pre_execution', 'post_plan', 'pre_commit', 'post_execution', 'custom'
    )),
    -- JSON array of required approver IDs or roles
    required_approvers TEXT NOT NULL DEFAULT '[]',
    min_approvals INTEGER NOT NULL DEFAULT 1,
    -- Conditions for when this gate applies (JSON)
    -- e.g., {"autonomy_modes": ["review_driven"], "file_patterns": ["*.sql"]}
    conditions TEXT DEFAULT '{}',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Approval records (actual approvals for gates)
CREATE TABLE IF NOT EXISTS gate_approvals (
    id BLOB PRIMARY KEY,
    approval_gate_id BLOB NOT NULL REFERENCES approval_gates(id) ON DELETE CASCADE,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    approver_id TEXT NOT NULL,
    approver_name TEXT,
    decision TEXT NOT NULL CHECK (decision IN ('approved', 'rejected', 'abstained')),
    comment TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Pending gate instances (gates that need approval for a specific execution)
CREATE TABLE IF NOT EXISTS pending_gates (
    id BLOB PRIMARY KEY,
    approval_gate_id BLOB NOT NULL REFERENCES approval_gates(id) ON DELETE CASCADE,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'rejected', 'bypassed')),
    -- Context about what triggered this gate (JSON)
    trigger_context TEXT,
    -- Count of approvals received
    approval_count INTEGER NOT NULL DEFAULT 0,
    rejection_count INTEGER NOT NULL DEFAULT 0,
    resolved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_checkpoint_definitions_project
    ON checkpoint_definitions(project_id);

CREATE INDEX IF NOT EXISTS idx_checkpoint_definitions_type
    ON checkpoint_definitions(checkpoint_type);

CREATE INDEX IF NOT EXISTS idx_execution_checkpoints_execution
    ON execution_checkpoints(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_execution_checkpoints_status
    ON execution_checkpoints(status)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_approval_gates_project
    ON approval_gates(project_id);

CREATE INDEX IF NOT EXISTS idx_approval_gates_task
    ON approval_gates(task_id);

CREATE INDEX IF NOT EXISTS idx_gate_approvals_gate
    ON gate_approvals(approval_gate_id);

CREATE INDEX IF NOT EXISTS idx_gate_approvals_execution
    ON gate_approvals(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_pending_gates_execution
    ON pending_gates(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_pending_gates_status
    ON pending_gates(status)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_tasks_autonomy_mode
    ON tasks(autonomy_mode);

-- Default checkpoint definitions (global, apply to all projects)
INSERT INTO checkpoint_definitions (id, project_id, name, description, checkpoint_type, config, requires_approval, priority) VALUES
    (randomblob(16), NULL, 'Large File Change', 'Triggered when more than 10 files are modified', 'file_change', '{"max_files": 10}', 1, 10),
    (randomblob(16), NULL, 'External API Call', 'Triggered on any external API call', 'external_call', '{}', 1, 20),
    (randomblob(16), NULL, 'Cost Threshold', 'Triggered when API cost exceeds $5', 'cost_threshold', '{"max_cost": 5.00}', 1, 15),
    (randomblob(16), NULL, 'Long Running Task', 'Triggered after 30 minutes of execution', 'time_threshold', '{"max_minutes": 30}', 0, 5);
