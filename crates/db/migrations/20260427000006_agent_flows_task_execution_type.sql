-- Migration: Add 'task_execution' to agent_flows.flow_type CHECK constraint
-- SQLite doesn't support ALTER TABLE to modify constraints, so we recreate the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE agent_flows_new (
    id BLOB PRIMARY KEY,
    task_id BLOB NOT NULL,
    flow_type TEXT NOT NULL CHECK (flow_type IN (
        'content_creation', 'research', 'engagement', 'scheduling',
        'campaign', 'analysis', 'monitoring', 'custom', 'task_execution'
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
    last_error TEXT,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL
);

INSERT INTO agent_flows_new SELECT * FROM agent_flows;

DROP TABLE agent_flows;
ALTER TABLE agent_flows_new RENAME TO agent_flows;

PRAGMA foreign_keys = ON;
