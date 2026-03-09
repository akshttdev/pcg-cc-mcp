ALTER TABLE workflow_runs ADD COLUMN content_hash TEXT;
CREATE INDEX IF NOT EXISTS idx_workflow_runs_content_hash ON workflow_runs(content_hash);
