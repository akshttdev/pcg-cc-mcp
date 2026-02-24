-- Migration: Add 'sales' and 'delivery' to crm_pipelines.pipeline_type CHECK constraint
-- SQLite doesn't support ALTER CONSTRAINT, so we recreate the table

-- Disable FK enforcement during table swap
PRAGMA foreign_keys = OFF;

-- Step 1: Create new table with updated constraint
CREATE TABLE crm_pipelines_new (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    name TEXT NOT NULL,
    description TEXT,
    pipeline_type TEXT NOT NULL CHECK (pipeline_type IN (
        'conferences', 'clients', 'sales', 'delivery', 'custom'
    )),

    is_active INTEGER DEFAULT 1,
    is_default INTEGER DEFAULT 0,

    icon TEXT,
    color TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, name)
);

-- Step 2: Copy existing data
INSERT INTO crm_pipelines_new SELECT * FROM crm_pipelines;

-- Step 3: Drop old table
DROP TABLE crm_pipelines;

-- Step 4: Rename new table
ALTER TABLE crm_pipelines_new RENAME TO crm_pipelines;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_crm_pipelines_project ON crm_pipelines(project_id);
CREATE INDEX IF NOT EXISTS idx_crm_pipelines_type ON crm_pipelines(pipeline_type);

-- Re-enable FK enforcement
PRAGMA foreign_keys = ON;
