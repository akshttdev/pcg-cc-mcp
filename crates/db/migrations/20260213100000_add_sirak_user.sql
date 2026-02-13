-- Add Sirak user account, projects, and fix admin project memberships
-- Password: Sirak123 (bcrypt cost 12)

-- 1. Insert Sirak user
INSERT OR IGNORE INTO users (
    id,
    username,
    email,
    full_name,
    password_hash,
    is_admin,
    is_active
) VALUES (
    X'a0a1a2a3a4a5a6a7a8a9aaabacadaeaf',
    'sirak',
    'sirak@powerclubglobal.com',
    'Sirak',
    '$2b$12$r.nAooN5ov2dBokKgnlh3ewOsVSB6Q5z4UxQJk16l2wcHSEQFMxQO',
    0,
    1
);

-- 2. Register Sirak's projects
-- Note: explicit timestamps needed for SQLite < 3.42.0 (subsec not supported)
INSERT OR IGNORE INTO projects (id, name, git_repo_path, created_at, updated_at)
VALUES (
    X'b0b1b2b3b4b5b6b7b8b9babbbcbdbebf',
    'sirak-studios',
    '/home/spaceterminal/topos/sirak-studios',
    datetime('now'),
    datetime('now')
);

INSERT OR IGNORE INTO projects (id, name, git_repo_path, created_at, updated_at)
VALUES (
    X'c0c1c2c3c4c5c6c7c8c9cacbcccdcecf',
    'Prime',
    '/home/spaceterminal/topos/Prime',
    datetime('now'),
    datetime('now')
);

-- 3. Add Sirak as owner of both projects (project_id stored as UUID text)
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role)
VALUES (
    randomblob(16),
    'b0b1b2b3-b4b5-b6b7-b8b9-babbbcbdbebf',
    X'a0a1a2a3a4a5a6a7a8a9aaabacadaeaf',
    'owner'
);

INSERT OR IGNORE INTO project_members (id, project_id, user_id, role)
VALUES (
    randomblob(16),
    'c0c1c2c3-c4c5-c6c7-c8c9-cacbcccdcecf',
    X'a0a1a2a3a4a5a6a7a8a9aaabacadaeaf',
    'owner'
);

-- 4. Add Admin as owner of ALL existing projects (fixes empty dashboard for admin)
-- Admin sees all projects via is_admin flag, but project_members entries
-- ensure data consistency. Uses UUID text format for project_id column.
INSERT OR IGNORE INTO project_members (id, project_id, user_id, role)
SELECT
    randomblob(16),
    lower(
        substr(hex(p.id), 1, 8) || '-' ||
        substr(hex(p.id), 9, 4) || '-' ||
        substr(hex(p.id), 13, 4) || '-' ||
        substr(hex(p.id), 17, 4) || '-' ||
        substr(hex(p.id), 21, 12)
    ),
    u.id,
    'owner'
FROM projects p
CROSS JOIN users u
WHERE u.username = 'admin';
