-- Relax agent_flows FK constraints for BLOB/TEXT UUID compatibility.
-- The task_id FK references tasks(id) which stores TEXT UUIDs,
-- but agent_flows.task_id was BLOB. SQLite can't cross-type FK match.
-- Solution: recreate table without FK constraint on task_id.
-- agent_flows.crm_deal_id (TEXT) already works correctly.

CREATE TABLE agent_flows_new (
    id BLOB PRIMARY KEY,
    task_id BLOB NOT NULL,  -- no FK constraint (BLOB/TEXT mismatch)
    flow_type TEXT NOT NULL CHECK (flow_type IN (
        'content_creation', 'research', 'engagement', 'scheduling',
        'campaign', 'analysis', 'monitoring', 'custom'
    )),
    status TEXT NOT NULL DEFAULT 'planning' CHECK (status IN (
        'planning', 'executing', 'verifying', 'completed', 'failed', 'paused', 'awaiting_approval', 'needs_clarification'
    )),
    planner_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    executor_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    verifier_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    current_phase TEXT NOT NULL DEFAULT 'planning' CHECK (current_phase IN (
        'planning', 'execution', 'verification'
    )),
    flow_config TEXT,
    handoff_instructions TEXT,
    human_approval_required BOOLEAN NOT NULL DEFAULT 0,
    verification_score REAL,
    approved_by TEXT,
    planning_started_at TEXT,
    executing_started_at TEXT,
    verifying_started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    crm_deal_id TEXT,
    cancel_deadline TEXT,
    clarification_request TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

INSERT INTO agent_flows_new SELECT * FROM agent_flows;
DROP TABLE agent_flows;
ALTER TABLE agent_flows_new RENAME TO agent_flows;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_agent_flows_crm_deal ON agent_flows(crm_deal_id);
CREATE INDEX IF NOT EXISTS idx_agent_flows_status ON agent_flows(status);
CREATE INDEX IF NOT EXISTS idx_agent_flows_task ON agent_flows(task_id);
