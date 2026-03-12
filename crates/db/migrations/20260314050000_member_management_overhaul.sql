-- Phase 1: Member Management Schema Overhaul
-- Adds: polymorphic task assignee, org-scoped tags, org invitations

-- ═══════════════════════════════════════════════════════════════════
-- 1. Polymorphic task assignee: add assignee_type column
-- ═══════════════════════════════════════════════════════════════════
ALTER TABLE tasks ADD COLUMN assignee_type TEXT
    CHECK (assignee_type IN ('user', 'agent', 'team'));

-- Backfill existing records (safe to run even if already backfilled)
UPDATE tasks SET assignee_type = 'user'
    WHERE assignee_id IS NOT NULL AND assignee_type IS NULL;
UPDATE tasks SET assignee_type = 'agent'
    WHERE agent_id IS NOT NULL AND assignee_id IS NULL AND assignee_type IS NULL;

-- ═══════════════════════════════════════════════════════════════════
-- 2. Org-scoped tags: recreate tags table with nullable project_id
--    and new organization_id column
-- ═══════════════════════════════════════════════════════════════════

-- Must disable FK checks during table swap
PRAGMA foreign_keys = OFF;

-- Recreate with nullable project_id + new organization_id
CREATE TABLE tags_new (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE CASCADE,
    organization_id BLOB REFERENCES organizations(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT DEFAULT '#gray',
    content TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Copy existing data
INSERT INTO tags_new (id, project_id, organization_id, name, color, content, created_at, updated_at)
    SELECT t.id, t.project_id,
           (SELECT p.organization_id FROM projects p WHERE p.id = t.project_id),
           t.name, t.color, t.content, t.created_at, t.updated_at
    FROM tags t;

-- Swap tables
DROP TABLE tags;
ALTER TABLE tags_new RENAME TO tags;

-- Rebuild indexes
CREATE INDEX idx_tags_project_id ON tags(project_id);
CREATE INDEX idx_tags_organization_id ON tags(organization_id);
CREATE UNIQUE INDEX idx_tags_org_name ON tags(
    COALESCE(project_id, organization_id), name
);

PRAGMA foreign_keys = ON;

-- ═══════════════════════════════════════════════════════════════════
-- 3. Org-level invitations (copy-link flow)
-- ═══════════════════════════════════════════════════════════════════
CREATE TABLE IF NOT EXISTS org_invitations (
    id BLOB PRIMARY KEY NOT NULL,
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    invite_code TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL DEFAULT 'member'
        CHECK (role IN ('admin', 'member', 'viewer')),
    created_by BLOB NOT NULL REFERENCES users(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    expires_at TEXT NOT NULL,
    accepted_by BLOB REFERENCES users(id),
    accepted_at TEXT,
    max_uses INTEGER NOT NULL DEFAULT 1,
    use_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_org_invitations_code ON org_invitations(invite_code);
CREATE INDEX idx_org_invitations_org ON org_invitations(organization_id);
CREATE INDEX idx_org_invitations_expires ON org_invitations(expires_at);
