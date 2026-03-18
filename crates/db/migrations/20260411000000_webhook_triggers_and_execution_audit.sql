-- Add webhook trigger support and execution audit trail
--
-- 1. Recreate workflow_triggers with webhook type + new columns
-- 2. Create trigger_executions table for audit trail

-- Step 1: Recreate workflow_triggers with updated CHECK and new columns
CREATE TABLE IF NOT EXISTS workflow_triggers_new (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_id TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 0,
    trigger_type TEXT NOT NULL DEFAULT 'data_source_created'
        CHECK(trigger_type IN ('data_source_created', 'data_source_updated', 'schedule', 'webhook')),
    -- Filter criteria: which data sources match this trigger
    filter_data_source_types TEXT,        -- JSON array of data_source types e.g. ["conversation","transcript"]
    filter_organization_id TEXT,          -- Only trigger for this org's data sources
    filter_project_id TEXT,               -- Only trigger for this project's data sources
    filter_tags TEXT,                     -- JSON array of required tags / schedule config
    -- Execution config
    model_override TEXT,                  -- Override workflow default model
    auto_approve INTEGER NOT NULL DEFAULT 0,  -- Auto-approve valid (non-duplicate) records
    -- Webhook-specific fields
    webhook_secret TEXT,                  -- HMAC-SHA256 secret for verifying webhook payloads
    webhook_url TEXT,                     -- The URL to POST to (displayed to user for copy)
    -- Rate limiting
    cooldown_seconds INTEGER NOT NULL DEFAULT 0,  -- Minimum seconds between trigger fires (0 = no cooldown)
    -- Retry config
    max_retries INTEGER NOT NULL DEFAULT 0,       -- Max retry attempts on failure (0 = no retries)
    -- Error tracking
    last_error TEXT,                      -- Last execution error message
    retry_count INTEGER NOT NULL DEFAULT 0,       -- Current retry counter (resets on success)
    next_retry_at TEXT,                   -- When to attempt next retry
    -- Metadata
    last_triggered_at TEXT,
    trigger_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Copy existing data, fixing 'scheduled' -> 'schedule'
INSERT INTO workflow_triggers_new (
    id, workflow_id, name, enabled, trigger_type,
    filter_data_source_types, filter_organization_id, filter_project_id, filter_tags,
    model_override, auto_approve,
    last_triggered_at, trigger_count, created_at, updated_at
)
SELECT
    id, workflow_id, name, enabled,
    CASE WHEN trigger_type = 'scheduled' THEN 'schedule' ELSE trigger_type END,
    filter_data_source_types, filter_organization_id, filter_project_id, filter_tags,
    model_override, auto_approve,
    last_triggered_at, trigger_count, created_at, updated_at
FROM workflow_triggers;

DROP TABLE workflow_triggers;
ALTER TABLE workflow_triggers_new RENAME TO workflow_triggers;

CREATE INDEX IF NOT EXISTS idx_workflow_triggers_workflow_id ON workflow_triggers(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_triggers_enabled ON workflow_triggers(enabled);
CREATE INDEX IF NOT EXISTS idx_workflow_triggers_type ON workflow_triggers(trigger_type);

-- Step 2: Trigger executions audit table
CREATE TABLE IF NOT EXISTS trigger_executions (
    id TEXT PRIMARY KEY NOT NULL,
    trigger_id TEXT NOT NULL REFERENCES workflow_triggers(id) ON DELETE CASCADE,
    workflow_run_id TEXT,                 -- Links to workflow_runs.id if a run was created
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending', 'running', 'completed', 'failed', 'retrying')),
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at TEXT,
    duration_ms INTEGER,
    error TEXT,
    records_staged INTEGER NOT NULL DEFAULT 0,
    -- Source info (what triggered this execution)
    source_type TEXT,                     -- 'data_source', 'schedule', 'webhook', 'manual'
    source_id TEXT,                       -- data_source_id or webhook request ID
    metadata TEXT,                        -- JSON blob for extra context
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_trigger_executions_trigger_id ON trigger_executions(trigger_id);
CREATE INDEX IF NOT EXISTS idx_trigger_executions_status ON trigger_executions(status);
CREATE INDEX IF NOT EXISTS idx_trigger_executions_started_at ON trigger_executions(started_at);
