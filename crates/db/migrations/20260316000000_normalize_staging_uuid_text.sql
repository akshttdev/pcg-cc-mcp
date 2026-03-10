-- Normalize UUID columns in workflow_output_staging from BLOB to TEXT.
-- Some UUIDs may have been stored as 16-byte blobs by sqlx; this migration
-- converts them to the canonical 'xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx' text form.

CREATE TABLE IF NOT EXISTS workflow_output_staging_new (
    id TEXT NOT NULL PRIMARY KEY,
    workflow_run_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    data_source_id TEXT,
    organization_id TEXT,
    project_id TEXT,
    target_type TEXT NOT NULL
        CHECK (target_type IN ('crm_contact', 'company', 'crm_deal', 'task')),
    record_data TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending_review'
        CHECK (status IN ('pending_review', 'approved', 'rejected', 'committed', 'error')),
    duplicate_of_id TEXT,
    duplicate_of_type TEXT,
    confidence REAL,
    error_message TEXT,
    reviewed_by TEXT,
    reviewed_at DATETIME,
    committed_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    validation_errors TEXT
);

-- Convert data: BLOB UUIDs → hex with dashes, TEXT UUIDs → pass through as-is.
INSERT INTO workflow_output_staging_new (
    id, workflow_run_id, workflow_id, node_id, data_source_id,
    organization_id, project_id, target_type, record_data, status,
    duplicate_of_id, duplicate_of_type, confidence, error_message,
    reviewed_by, reviewed_at, committed_at, created_at, updated_at,
    validation_errors
)
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(workflow_run_id) = 'blob' THEN lower(substr(hex(workflow_run_id),1,8)||'-'||substr(hex(workflow_run_id),9,4)||'-'||substr(hex(workflow_run_id),13,4)||'-'||substr(hex(workflow_run_id),17,4)||'-'||substr(hex(workflow_run_id),21,12)) ELSE workflow_run_id END,
    workflow_id,
    node_id,
    CASE WHEN data_source_id IS NULL THEN NULL WHEN typeof(data_source_id) = 'blob' THEN lower(substr(hex(data_source_id),1,8)||'-'||substr(hex(data_source_id),9,4)||'-'||substr(hex(data_source_id),13,4)||'-'||substr(hex(data_source_id),17,4)||'-'||substr(hex(data_source_id),21,12)) ELSE data_source_id END,
    CASE WHEN organization_id IS NULL THEN NULL WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN project_id IS NULL THEN NULL WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    target_type,
    record_data,
    status,
    CASE WHEN duplicate_of_id IS NULL THEN NULL WHEN typeof(duplicate_of_id) = 'blob' THEN lower(substr(hex(duplicate_of_id),1,8)||'-'||substr(hex(duplicate_of_id),9,4)||'-'||substr(hex(duplicate_of_id),13,4)||'-'||substr(hex(duplicate_of_id),17,4)||'-'||substr(hex(duplicate_of_id),21,12)) ELSE duplicate_of_id END,
    duplicate_of_type,
    confidence,
    error_message,
    CASE WHEN reviewed_by IS NULL THEN NULL WHEN typeof(reviewed_by) = 'blob' THEN lower(substr(hex(reviewed_by),1,8)||'-'||substr(hex(reviewed_by),9,4)||'-'||substr(hex(reviewed_by),13,4)||'-'||substr(hex(reviewed_by),17,4)||'-'||substr(hex(reviewed_by),21,12)) ELSE reviewed_by END,
    reviewed_at,
    committed_at,
    created_at,
    updated_at,
    validation_errors
FROM workflow_output_staging;

DROP TABLE workflow_output_staging;
ALTER TABLE workflow_output_staging_new RENAME TO workflow_output_staging;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_staging_workflow_run ON workflow_output_staging(workflow_run_id);
CREATE INDEX IF NOT EXISTS idx_staging_status ON workflow_output_staging(status);
CREATE INDEX IF NOT EXISTS idx_staging_org ON workflow_output_staging(organization_id);
CREATE INDEX IF NOT EXISTS idx_staging_target ON workflow_output_staging(target_type);
