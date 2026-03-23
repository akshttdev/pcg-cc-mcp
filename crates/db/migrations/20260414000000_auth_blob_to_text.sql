-- Convert remaining auth-related tables from BLOB UUID storage to TEXT UUID storage.
-- Tables: users, organization_members, sessions, user_platform_roles
-- Pattern: create _new table with TEXT columns, INSERT with hex conversion, DROP old, RENAME.

PRAGMA foreign_keys = OFF;

-- Helper expression used throughout for BLOB→TEXT UUID conversion:
-- CASE WHEN col IS NULL THEN NULL
--      WHEN typeof(col) = 'blob' THEN lower(substr(hex(col),1,8)||'-'||substr(hex(col),9,4)||'-'||substr(hex(col),13,4)||'-'||substr(hex(col),17,4)||'-'||substr(hex(col),21,12))
--      ELSE col END

-----------------------------------------------------------------------
-- 1. users
-----------------------------------------------------------------------
CREATE TABLE users_new (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT NOT NULL,
    avatar_url TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    is_admin INTEGER NOT NULL DEFAULT 0,
    last_login_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT,
    external_provider TEXT,
    external_id TEXT,
    deleted_at TEXT,
    deleted_by TEXT REFERENCES users_new(id) ON DELETE SET NULL,
    wallet_address TEXT,
    home_project_id TEXT,
    user_role TEXT NOT NULL DEFAULT 'guest' CHECK (user_role IN ('host', 'guest')),
    invited_by TEXT REFERENCES users_new(id) ON DELETE SET NULL,
    home_organization_id TEXT REFERENCES organizations(id)
);

INSERT INTO users_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    username, email, password_hash, full_name, avatar_url, is_active, is_admin, last_login_at, created_at, updated_at,
    CASE WHEN created_by IS NULL THEN NULL WHEN typeof(created_by) = 'blob' THEN lower(substr(hex(created_by),1,8)||'-'||substr(hex(created_by),9,4)||'-'||substr(hex(created_by),13,4)||'-'||substr(hex(created_by),17,4)||'-'||substr(hex(created_by),21,12)) ELSE created_by END,
    external_provider, external_id, deleted_at,
    CASE WHEN deleted_by IS NULL THEN NULL WHEN typeof(deleted_by) = 'blob' THEN lower(substr(hex(deleted_by),1,8)||'-'||substr(hex(deleted_by),9,4)||'-'||substr(hex(deleted_by),13,4)||'-'||substr(hex(deleted_by),17,4)||'-'||substr(hex(deleted_by),21,12)) ELSE deleted_by END,
    wallet_address,
    CASE WHEN home_project_id IS NULL THEN NULL WHEN typeof(home_project_id) = 'blob' THEN lower(substr(hex(home_project_id),1,8)||'-'||substr(hex(home_project_id),9,4)||'-'||substr(hex(home_project_id),13,4)||'-'||substr(hex(home_project_id),17,4)||'-'||substr(hex(home_project_id),21,12)) ELSE home_project_id END,
    user_role,
    CASE WHEN invited_by IS NULL THEN NULL WHEN typeof(invited_by) = 'blob' THEN lower(substr(hex(invited_by),1,8)||'-'||substr(hex(invited_by),9,4)||'-'||substr(hex(invited_by),13,4)||'-'||substr(hex(invited_by),17,4)||'-'||substr(hex(invited_by),21,12)) ELSE invited_by END,
    CASE WHEN home_organization_id IS NULL THEN NULL WHEN typeof(home_organization_id) = 'blob' THEN lower(substr(hex(home_organization_id),1,8)||'-'||substr(hex(home_organization_id),9,4)||'-'||substr(hex(home_organization_id),13,4)||'-'||substr(hex(home_organization_id),17,4)||'-'||substr(hex(home_organization_id),21,12)) ELSE home_organization_id END
FROM users;

DROP TABLE users;
ALTER TABLE users_new RENAME TO users;

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_is_admin ON users(is_admin);
CREATE UNIQUE INDEX idx_users_external ON users(external_provider, external_id)
    WHERE external_provider IS NOT NULL AND external_id IS NOT NULL;
CREATE INDEX idx_users_deleted_at ON users(deleted_at);
CREATE UNIQUE INDEX idx_users_wallet_address ON users(wallet_address) WHERE wallet_address IS NOT NULL;

-----------------------------------------------------------------------
-- 2. organization_members
-----------------------------------------------------------------------
CREATE TABLE organization_members_new (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(organization_id, user_id)
);

INSERT INTO organization_members_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    organization_id,
    CASE WHEN typeof(user_id) = 'blob' THEN lower(substr(hex(user_id),1,8)||'-'||substr(hex(user_id),9,4)||'-'||substr(hex(user_id),13,4)||'-'||substr(hex(user_id),17,4)||'-'||substr(hex(user_id),21,12)) ELSE user_id END,
    role, joined_at
FROM organization_members;

DROP TABLE organization_members;
ALTER TABLE organization_members_new RENAME TO organization_members;

CREATE INDEX idx_org_members_org_id ON organization_members(organization_id);
CREATE INDEX idx_org_members_user_id ON organization_members(user_id);
CREATE INDEX idx_org_members_org_role ON organization_members(organization_id, role);

-----------------------------------------------------------------------
-- 3. sessions
-----------------------------------------------------------------------
CREATE TABLE sessions_new (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    ip_address TEXT,
    user_agent TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

INSERT INTO sessions_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(user_id) = 'blob' THEN lower(substr(hex(user_id),1,8)||'-'||substr(hex(user_id),9,4)||'-'||substr(hex(user_id),13,4)||'-'||substr(hex(user_id),17,4)||'-'||substr(hex(user_id),21,12)) ELSE user_id END,
    token_hash, ip_address, user_agent, expires_at, created_at, last_used_at
FROM sessions;

DROP TABLE sessions;
ALTER TABLE sessions_new RENAME TO sessions;

CREATE INDEX idx_sessions_user_id ON sessions(user_id);
CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);

-----------------------------------------------------------------------
-- 4. user_platform_roles
-----------------------------------------------------------------------
CREATE TABLE user_platform_roles_new (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('platform_admin', 'operator', 'client_user')),
    granted_by TEXT,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(user_id, role)
);

INSERT INTO user_platform_roles_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(user_id) = 'blob' THEN lower(substr(hex(user_id),1,8)||'-'||substr(hex(user_id),9,4)||'-'||substr(hex(user_id),13,4)||'-'||substr(hex(user_id),17,4)||'-'||substr(hex(user_id),21,12)) ELSE user_id END,
    role,
    CASE WHEN granted_by IS NULL THEN NULL WHEN typeof(granted_by) = 'blob' THEN lower(substr(hex(granted_by),1,8)||'-'||substr(hex(granted_by),9,4)||'-'||substr(hex(granted_by),13,4)||'-'||substr(hex(granted_by),17,4)||'-'||substr(hex(granted_by),21,12)) ELSE granted_by END,
    granted_at
FROM user_platform_roles;

DROP TABLE user_platform_roles;
ALTER TABLE user_platform_roles_new RENAME TO user_platform_roles;

CREATE INDEX idx_user_platform_roles_user_id ON user_platform_roles(user_id);
CREATE INDEX idx_user_platform_roles_role ON user_platform_roles(role);

PRAGMA foreign_keys = ON;
