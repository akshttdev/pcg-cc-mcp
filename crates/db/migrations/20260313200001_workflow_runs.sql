CREATE TABLE IF NOT EXISTS workflow_runs (
    id TEXT PRIMARY KEY NOT NULL,
    workflow_id TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    data_source_id TEXT,
    organization_id TEXT,
    project_id TEXT,
    model_used TEXT,
    status TEXT NOT NULL DEFAULT 'completed' CHECK(status IN ('running', 'completed', 'failed')),
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0,
    total_estimated_cost_micros INTEGER DEFAULT 0,
    total_records_staged INTEGER DEFAULT 0,
    total_records_approved INTEGER DEFAULT 0,
    total_records_rejected INTEGER DEFAULT 0,
    total_records_committed INTEGER DEFAULT 0,
    total_duplicates_found INTEGER DEFAULT 0,
    total_validation_errors INTEGER DEFAULT 0,
    node_count INTEGER DEFAULT 0,
    llm_node_count INTEGER DEFAULT 0,
    duration_ms INTEGER,
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_runs_workflow_id ON workflow_runs(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_organization_id ON workflow_runs(organization_id);
CREATE INDEX IF NOT EXISTS idx_workflow_runs_created_at ON workflow_runs(created_at);
