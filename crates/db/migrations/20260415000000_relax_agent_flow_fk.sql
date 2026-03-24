-- Relax agent_flows FK constraints for BLOB/TEXT UUID compatibility.
-- Also adds needs_clarification to status CHECK.

CREATE TABLE agent_flows_new (
    id BLOB PRIMARY KEY,
    task_id BLOB NOT NULL,
    flow_type TEXT NOT NULL CHECK (flow_type IN (
        'content_creation', 'research', 'engagement', 'scheduling',
        'campaign', 'analysis', 'monitoring', 'custom'
    )),
    status TEXT NOT NULL DEFAULT 'planning' CHECK (status IN (
        'planning', 'executing', 'verifying', 'completed', 'failed', 'paused', 'awaiting_approval', 'needs_clarification'
    )),
    planner_agent_id BLOB,
    executor_agent_id BLOB,
    verifier_agent_id BLOB,
    current_phase TEXT NOT NULL DEFAULT 'planning' CHECK (current_phase IN (
        'planning', 'execution', 'verification'
    )),
    planning_started_at TEXT,
    planning_completed_at TEXT,
    execution_started_at TEXT,
    execution_completed_at TEXT,
    verification_started_at TEXT,
    verification_completed_at TEXT,
    flow_config TEXT,
    handoff_instructions TEXT,
    verification_score REAL,
    human_approval_required BOOLEAN NOT NULL DEFAULT 0,
    approved_by TEXT,
    approved_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    clarification_request TEXT,
    crm_deal_id TEXT,
    cancel_deadline TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);

INSERT INTO agent_flows_new SELECT * FROM agent_flows;
DROP TABLE agent_flows;
ALTER TABLE agent_flows_new RENAME TO agent_flows;

CREATE INDEX IF NOT EXISTS idx_agent_flows_crm_deal ON agent_flows(crm_deal_id);
CREATE INDEX IF NOT EXISTS idx_agent_flows_status ON agent_flows(status);
CREATE INDEX IF NOT EXISTS idx_agent_flows_task ON agent_flows(task_id);
