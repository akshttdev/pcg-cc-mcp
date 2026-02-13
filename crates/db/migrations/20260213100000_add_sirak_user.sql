-- Add Sirak user account and projects
-- Password: Sirak123 (bcrypt cost 12)
-- Safe migration: only inserts data, no FK-dependent operations that could fail

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
