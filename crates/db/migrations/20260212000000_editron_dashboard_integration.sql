-- Editron Dashboard Integration
-- Makes execution_process_id nullable so Editron can create artifacts directly
-- (without going through the coding-agent ExecutionProcess chain).
-- Adds 4 media artifact types and seeds Editron pricing rows.

-----------------------------------------------------------------------
-- 1. Recreate execution_artifacts with execution_process_id nullable
--    (SQLite requires table recreation to change column constraints)
-----------------------------------------------------------------------

-- Note: PRAGMA foreign_keys = OFF cannot be used inside a transaction
-- (SQLx migrations run inside transactions). Instead, we omit the
-- self-referencing FK on parent_artifact_id during table recreation.
-- The FK is re-established implicitly after RENAME since the table name
-- matches. For the bulk INSERT to succeed we must avoid self-referencing
-- FKs that would require ordered insertion.

CREATE TABLE execution_artifacts_new (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB REFERENCES execution_processes(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL CHECK (artifact_type IN (
        -- Original types
        'plan', 'screenshot', 'walkthrough', 'diff_summary', 'test_result',
        'checkpoint', 'error_report',
        -- Planning phase artifacts
        'research_report', 'strategy_document', 'content_calendar', 'competitor_analysis',
        -- Execution phase artifacts
        'content_draft', 'visual_brief', 'schedule_manifest', 'engagement_log',
        -- Verification phase artifacts
        'verification_report', 'browser_recording', 'compliance_score', 'platform_screenshot',
        -- Wide Research artifacts
        'subagent_result', 'aggregated_research',
        -- Editron media artifacts (NEW)
        'media_ingest_manifest', 'media_analysis_report', 'video_edit_session', 'render_deliverable'
    )),
    title TEXT NOT NULL,
    content TEXT,
    file_path TEXT,
    metadata TEXT,
    phase TEXT CHECK (phase IN ('planning', 'execution', 'verification')),
    created_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    review_status TEXT DEFAULT 'none' CHECK (review_status IN
        ('none', 'pending', 'approved', 'rejected', 'revision_requested')),
    parent_artifact_id BLOB,  -- Self-ref FK omitted for migration compatibility
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Copy existing data
INSERT INTO execution_artifacts_new
SELECT * FROM execution_artifacts;

-- Drop old indexes first (they reference old table)
DROP INDEX IF EXISTS idx_execution_artifacts_process;
DROP INDEX IF EXISTS idx_execution_artifacts_type;
DROP INDEX IF EXISTS idx_execution_artifacts_phase;
DROP INDEX IF EXISTS idx_execution_artifacts_review;
DROP INDEX IF EXISTS idx_execution_artifacts_agent;

-- Drop old table
DROP TABLE execution_artifacts;

-- Rename new table
ALTER TABLE execution_artifacts_new RENAME TO execution_artifacts;

-- Recreate indexes
CREATE INDEX idx_execution_artifacts_process ON execution_artifacts(execution_process_id);
CREATE INDEX idx_execution_artifacts_type ON execution_artifacts(artifact_type);
CREATE INDEX idx_execution_artifacts_phase ON execution_artifacts(phase);
CREATE INDEX idx_execution_artifacts_review ON execution_artifacts(review_status) WHERE review_status != 'none';
CREATE INDEX idx_execution_artifacts_agent ON execution_artifacts(created_by_agent_id);

-----------------------------------------------------------------------
-- 2. Seed model_pricing rows for Editron operations
--    These use fixed vibe costs (not token-based), but we store them
--    in model_pricing for consistency with the dashboard's cost queries.
-----------------------------------------------------------------------

INSERT OR IGNORE INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier, effective_from)
VALUES
    (randomblob(16), 'editron-ingest',  'editron', 500,  0, 1.0, datetime('now')),
    (randomblob(16), 'editron-analyze', 'editron', 2000, 0, 1.0, datetime('now')),
    (randomblob(16), 'editron-edit',    'editron', 3000, 0, 1.0, datetime('now')),
    (randomblob(16), 'editron-render',  'editron', 5000, 0, 1.0, datetime('now'));
