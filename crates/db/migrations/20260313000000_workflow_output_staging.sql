-- Workflow output staging table: holds records extracted by workflows
-- pending review before committing to CRM/tasks
CREATE TABLE IF NOT EXISTS workflow_output_staging (
    id BLOB PRIMARY KEY,                    -- UUID
    workflow_run_id BLOB NOT NULL,           -- groups records from same execution run
    workflow_id TEXT NOT NULL,               -- which workflow definition produced this
    node_id TEXT NOT NULL,                   -- which output node produced this record
    data_source_id BLOB,                    -- source data that was analyzed
    organization_id BLOB,                   -- org scope for CRM operations
    project_id BLOB,                        -- project scope
    target_type TEXT NOT NULL               -- 'crm_contact', 'company', 'crm_deal', 'task'
        CHECK (target_type IN ('crm_contact', 'company', 'crm_deal', 'task')),
    record_data TEXT NOT NULL,              -- JSON: the individual record to create
    status TEXT NOT NULL DEFAULT 'pending_review'
        CHECK (status IN ('pending_review', 'approved', 'rejected', 'committed', 'error')),
    duplicate_of_id BLOB,                   -- if dedup found a match, UUID of existing record
    duplicate_of_type TEXT,                 -- what table the duplicate is in
    confidence REAL,                        -- LLM confidence score if available
    error_message TEXT,                     -- error details if commit failed
    reviewed_by BLOB,                       -- user who reviewed
    reviewed_at DATETIME,
    committed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_staging_workflow_run ON workflow_output_staging(workflow_run_id);
CREATE INDEX IF NOT EXISTS idx_staging_status ON workflow_output_staging(status);
CREATE INDEX IF NOT EXISTS idx_staging_org ON workflow_output_staging(organization_id);
CREATE INDEX IF NOT EXISTS idx_staging_target ON workflow_output_staging(target_type);
