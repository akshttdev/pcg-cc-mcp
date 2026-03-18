-- Add FK constraints to org_cloud tables.
--
-- The original 20260409000000_org_cloud.sql created cloud_files and
-- cloud_contributions without REFERENCES on organization_id, project_id,
-- task_id, user_id, and contributed_by columns.  cloud_contributions already
-- has a proper FK on cloud_file_id, and org_cloud_settings is a simple
-- key-value table that doesn't need FKs.
--
-- SQLite does not support ALTER TABLE … ADD CONSTRAINT, so we recreate the
-- table with the correct FK declarations, copy data, drop old, rename.
--
-- cloud_contributions already has FK on cloud_file_id — we also add FKs for
-- organization_id and user_id.

-- ── cloud_files ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS cloud_files_new (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    storage_volume TEXT NOT NULL CHECK (storage_volume IN ('data_sources', 'media_pipeline', 'sovereign', 'sovereign_personal', 'sovereign_org', 'artifacts', 'dropbox')),
    content_hash TEXT,
    file_size_bytes INTEGER NOT NULL DEFAULT 0,
    mime_type TEXT,
    source_type TEXT NOT NULL DEFAULT 'upload' CHECK (source_type IN ('upload', 'sync', 'agent', 'integration')),
    source_id TEXT,
    source_table TEXT,
    visibility TEXT NOT NULL DEFAULT 'org' CHECK (visibility IN ('org', 'project', 'task', 'private')),
    contributed_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    contributor_wallet TEXT,
    contributor_device TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

INSERT OR IGNORE INTO cloud_files_new SELECT * FROM cloud_files;
DROP TABLE IF EXISTS cloud_files;
ALTER TABLE cloud_files_new RENAME TO cloud_files;

CREATE INDEX IF NOT EXISTS idx_cloud_files_org ON cloud_files(organization_id);
CREATE INDEX IF NOT EXISTS idx_cloud_files_project ON cloud_files(project_id);
CREATE INDEX IF NOT EXISTS idx_cloud_files_task ON cloud_files(task_id);
CREATE INDEX IF NOT EXISTS idx_cloud_files_hash ON cloud_files(content_hash);
CREATE INDEX IF NOT EXISTS idx_cloud_files_visibility ON cloud_files(visibility);
CREATE INDEX IF NOT EXISTS idx_cloud_files_volume ON cloud_files(storage_volume);
CREATE INDEX IF NOT EXISTS idx_cloud_files_source ON cloud_files(source_id, source_table);

-- ── cloud_contributions ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS cloud_contributions_new (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    cloud_file_id TEXT NOT NULL REFERENCES cloud_files(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    wallet_address TEXT,
    device_id TEXT,
    contribution_type TEXT NOT NULL DEFAULT 'upload' CHECK (contribution_type IN ('upload', 'nats_publish', 'agent_output', 'sync')),
    channel TEXT NOT NULL DEFAULT 'http' CHECK (channel IN ('http', 'nats', 'pcg_sync', 'agent')),
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    description TEXT,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO cloud_contributions_new SELECT * FROM cloud_contributions;
DROP TABLE IF EXISTS cloud_contributions;
ALTER TABLE cloud_contributions_new RENAME TO cloud_contributions;

CREATE INDEX IF NOT EXISTS idx_cloud_contribs_org ON cloud_contributions(organization_id);
CREATE INDEX IF NOT EXISTS idx_cloud_contribs_file ON cloud_contributions(cloud_file_id);
CREATE INDEX IF NOT EXISTS idx_cloud_contribs_user ON cloud_contributions(user_id);

-- ── org_cloud_settings ─────────────────────────────────────────────────────
-- org_cloud_settings.organization_id should also reference organizations(id).

CREATE TABLE IF NOT EXISTS org_cloud_settings_new (
    organization_id TEXT PRIMARY KEY NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    storage_quota_bytes INTEGER NOT NULL DEFAULT 10737418240,
    viewer_can_download INTEGER NOT NULL DEFAULT 1,
    member_can_upload INTEGER NOT NULL DEFAULT 1,
    auto_index_data_sources INTEGER NOT NULL DEFAULT 1,
    auto_index_artifacts INTEGER NOT NULL DEFAULT 1,
    auto_index_media INTEGER NOT NULL DEFAULT 1,
    nats_contribution_enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT OR IGNORE INTO org_cloud_settings_new SELECT * FROM org_cloud_settings;
DROP TABLE IF EXISTS org_cloud_settings;
ALTER TABLE org_cloud_settings_new RENAME TO org_cloud_settings;
