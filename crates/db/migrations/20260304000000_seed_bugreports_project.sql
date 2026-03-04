-- Seed a constant Bug Reports project and board for tracking dashboard issues
-- These have static UUIDs that are referenced as constants in the codebase

-- Static UUIDs (stored as lowercase hex BLOBs in SQLite):
-- Bug Reports Project: 00000000-0000-0000-0000-000000000001
-- Bug Reports Board:   00000000-0000-0000-0000-000000000002

-- Insert the Bug Reports project if it doesn't exist
INSERT INTO projects (id, name, git_repo_path, created_at, updated_at)
SELECT
    X'00000000000000000000000000000001',
    'Dashboard Bug Reports',
    '__internal__/bugreports',
    datetime('now', 'subsec'),
    datetime('now', 'subsec')
WHERE NOT EXISTS (
    SELECT 1 FROM projects WHERE id = X'00000000000000000000000000000001'
);

-- Insert the Bug Reports board if it doesn't exist
INSERT INTO project_boards (id, project_id, name, slug, board_type, description, created_at, updated_at)
SELECT
    X'00000000000000000000000000000002',
    X'00000000000000000000000000000001',
    'Bug Reports',
    'bug-reports',
    'default',
    'Track bugs and issues reported by dashboard users',
    datetime('now', 'subsec'),
    datetime('now', 'subsec')
WHERE NOT EXISTS (
    SELECT 1 FROM project_boards WHERE id = X'00000000000000000000000000000002'
);
