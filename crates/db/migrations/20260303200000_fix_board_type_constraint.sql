-- Fix project_boards CHECK constraint to allow 'default' board type
-- The simplify_board_types migration (20260105100000) intended 'default' and 'custom'
-- to be the two valid types, but the CHECK constraint was never updated to include 'default'.
-- This causes Topsi's ensure_default_board() to fail with code 1811 (Invalid board type).

PRAGMA foreign_keys = OFF;

-- Step 1: Create replacement table with correct CHECK constraint
CREATE TABLE project_boards_new (
    id            BLOB PRIMARY KEY,
    project_id    BLOB NOT NULL,
    name          TEXT NOT NULL,
    slug          TEXT NOT NULL,
    board_type    TEXT NOT NULL CHECK (board_type IN (
        'default', 'custom', 'executive_assets', 'brand_assets', 'dev_assets', 'social_assets'
    )),
    description   TEXT,
    metadata      TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    CONSTRAINT project_boards_new_unique_slug UNIQUE (project_id, slug)
);

-- Step 2: Copy all data
INSERT INTO project_boards_new SELECT * FROM project_boards;

-- Step 3: Drop old table and indexes
DROP INDEX IF EXISTS idx_project_boards_project;
DROP TABLE project_boards;

-- Step 4: Rename new table
ALTER TABLE project_boards_new RENAME TO project_boards;

-- Step 5: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_project_boards_project ON project_boards(project_id);

PRAGMA foreign_keys = ON;
