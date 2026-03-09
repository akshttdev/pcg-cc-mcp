CREATE TABLE IF NOT EXISTS workflow_triggers (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_id TEXT NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    enabled INTEGER NOT NULL DEFAULT 0,
    trigger_type TEXT NOT NULL DEFAULT 'data_source_created' CHECK(trigger_type IN ('data_source_created', 'data_source_updated', 'scheduled')),
    -- Filter criteria: which data sources match this trigger
    filter_data_source_types TEXT,        -- JSON array of data_source types e.g. ["conversation","transcript"]
    filter_organization_id TEXT,          -- Only trigger for this org's data sources
    filter_project_id TEXT,               -- Only trigger for this project's data sources
    filter_tags TEXT,                     -- JSON array of required tags
    -- Execution config
    model_override TEXT,                  -- Override workflow default model
    auto_approve INTEGER NOT NULL DEFAULT 0,  -- Auto-approve valid (non-duplicate) records
    -- Metadata
    last_triggered_at TEXT,
    trigger_count INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_triggers_workflow_id ON workflow_triggers(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_triggers_enabled ON workflow_triggers(enabled);
