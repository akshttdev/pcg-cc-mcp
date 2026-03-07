-- Workflow definitions: user-editable data processing pipelines
CREATE TABLE IF NOT EXISTS workflow_definitions (
    id              TEXT PRIMARY KEY NOT NULL,         -- slug-style ID (e.g. "default_analysis")
    organization_id BLOB,                              -- optional org scope
    name            TEXT NOT NULL,
    description     TEXT,
    steps           TEXT NOT NULL DEFAULT '[]',         -- JSON array of WorkflowStep objects
    is_system       BOOLEAN NOT NULL DEFAULT 0,        -- true for built-in workflows
    created_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_definitions_org ON workflow_definitions(organization_id) WHERE organization_id IS NOT NULL;
