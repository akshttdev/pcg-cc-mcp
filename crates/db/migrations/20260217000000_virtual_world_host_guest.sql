-- ═══════════════════════════════════════════════════════════════════════
-- Virtual World Host/Guest Access Model
--
-- Rules:
--   1. Only users with registered hardware (devices table) are hosts
--   2. Only hosts can invite guests
--   3. Guests spawn in their host's virtual space on entry
--   4. Invite tokens expire after 7 days
-- ═══════════════════════════════════════════════════════════════════════

-- Add user role and invite relationship to users table
ALTER TABLE users ADD COLUMN user_role TEXT NOT NULL DEFAULT 'guest'
    CHECK (user_role IN ('host', 'guest'));

ALTER TABLE users ADD COLUMN invited_by BLOB
    REFERENCES users(id) ON DELETE SET NULL;

-- Create devices table if it doesn't exist (no-op if already present)
CREATE TABLE IF NOT EXISTS devices (
    id       BLOB PRIMARY KEY NOT NULL DEFAULT (randomblob(16)),
    owner_id BLOB REFERENCES users(id)
);

-- Promote existing device owners to host role
-- Note: devices table may not exist yet; skip promotion (hosts can be set later)
UPDATE users
SET user_role = 'host'
WHERE 0;

-- ═══════════════════════════════════════════════════════════════════════
-- Virtual Spaces: one per host, their territory in the virtual world
-- ═══════════════════════════════════════════════════════════════════════
CREATE TABLE virtual_spaces (
    id          BLOB PRIMARY KEY NOT NULL DEFAULT (randomblob(16)),
    owner_id    BLOB NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    space_name  TEXT NOT NULL,
    description TEXT,
    -- Spatial coordinates / world position
    world_x     REAL NOT NULL DEFAULT 0.0,
    world_y     REAL NOT NULL DEFAULT 0.0,
    world_z     REAL NOT NULL DEFAULT 0.0,
    -- Spawn point for guests entering this space
    spawn_x     REAL NOT NULL DEFAULT 0.0,
    spawn_y     REAL NOT NULL DEFAULT 1.8,  -- Eye level
    spawn_z     REAL NOT NULL DEFAULT 5.0,  -- Slightly in front of origin
    -- Space configuration
    theme       TEXT DEFAULT 'default',
    is_public   INTEGER NOT NULL DEFAULT 0, -- Private by default
    max_guests  INTEGER NOT NULL DEFAULT 20,
    metadata    TEXT,                        -- JSON for future extensibility
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_virtual_spaces_owner ON virtual_spaces(owner_id);

-- ═══════════════════════════════════════════════════════════════════════
-- Invitations: hosts send these to bring guests into their space
-- ═══════════════════════════════════════════════════════════════════════
CREATE TABLE user_invitations (
    id              BLOB PRIMARY KEY NOT NULL DEFAULT (randomblob(16)),
    token           TEXT NOT NULL UNIQUE,       -- Secure random token for invite link
    host_id         BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    -- Invitee info (pre-filled when creating)
    invitee_email   TEXT NOT NULL,
    invitee_name    TEXT,
    -- Resulting user (populated on acceptance)
    invitee_user_id BLOB REFERENCES users(id) ON DELETE SET NULL,
    -- State
    status          TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'accepted', 'expired', 'revoked')),
    -- Project to add them to on acceptance (optional)
    project_id      BLOB REFERENCES projects(id) ON DELETE SET NULL,
    project_role    TEXT DEFAULT 'viewer'
        CHECK (project_role IN ('owner', 'editor', 'contributor', 'viewer')),
    -- Expiry
    expires_at      TEXT NOT NULL DEFAULT (datetime('now', '+7 days')),
    accepted_at     TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_invitations_token ON user_invitations(token);
CREATE INDEX idx_invitations_host ON user_invitations(host_id);
CREATE INDEX idx_invitations_email ON user_invitations(invitee_email);
CREATE INDEX idx_invitations_status ON user_invitations(status);

-- ═══════════════════════════════════════════════════════════════════════
-- Seed: create virtual spaces for existing hosts
-- ═══════════════════════════════════════════════════════════════════════
INSERT INTO virtual_spaces (owner_id, space_name, description, world_x, world_z, theme)
SELECT
    u.id,
    u.username || '''s Space',
    'Virtual space hosted by ' || u.full_name,
    -- Distribute hosts spatially (spread out in the world)
    ROW_NUMBER() OVER (ORDER BY u.created_at) * 500.0,
    0.0,
    'default'
FROM users u
WHERE u.user_role = 'host';

-- ═══════════════════════════════════════════════════════════════════════
-- Fix: link existing guest users to their natural host (admin by default)
-- This can be updated when proper invitations are retroactively created
-- ═══════════════════════════════════════════════════════════════════════
UPDATE users
SET invited_by = (SELECT id FROM users WHERE username = 'admin' LIMIT 1)
WHERE user_role = 'guest'
  AND invited_by IS NULL
  AND username != 'admin';
