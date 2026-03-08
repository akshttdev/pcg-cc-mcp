-- Workflow definitions: user-editable data processing pipelines
-- Polymorphic ownership: owner_type + owner_id can represent user or organization
CREATE TABLE IF NOT EXISTS workflow_definitions (
    id              TEXT PRIMARY KEY NOT NULL,         -- slug-style ID (e.g. "default_analysis")
    owner_type      TEXT NOT NULL DEFAULT 'system',    -- 'system', 'organization', 'user'
    owner_id        BLOB,                              -- UUID of org or user (NULL for system)
    name            TEXT NOT NULL,
    description     TEXT,
    steps           TEXT NOT NULL DEFAULT '[]',         -- JSON: {"nodes":[], "connections":[]} or legacy step array
    is_system       BOOLEAN NOT NULL DEFAULT 0,        -- true for built-in workflows
    created_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_definitions_owner ON workflow_definitions(owner_type, owner_id) WHERE owner_id IS NOT NULL;
