-- Org Cloud: virtual file index, contribution audit trail, and per-org settings

CREATE TABLE IF NOT EXISTS cloud_files (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    project_id TEXT,
    task_id TEXT,
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
    contributed_by TEXT,
    contributor_wallet TEXT,
    contributor_device TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_cloud_files_org ON cloud_files(organization_id);
CREATE INDEX IF NOT EXISTS idx_cloud_files_project ON cloud_files(project_id);
CREATE INDEX IF NOT EXISTS idx_cloud_files_task ON cloud_files(task_id);
CREATE INDEX IF NOT EXISTS idx_cloud_files_hash ON cloud_files(content_hash);
CREATE INDEX IF NOT EXISTS idx_cloud_files_visibility ON cloud_files(visibility);
CREATE INDEX IF NOT EXISTS idx_cloud_files_volume ON cloud_files(storage_volume);
CREATE INDEX IF NOT EXISTS idx_cloud_files_source ON cloud_files(source_id, source_table);

CREATE TABLE IF NOT EXISTS cloud_contributions (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    cloud_file_id TEXT NOT NULL REFERENCES cloud_files(id) ON DELETE CASCADE,
    user_id TEXT,
    wallet_address TEXT,
    device_id TEXT,
    contribution_type TEXT NOT NULL DEFAULT 'upload' CHECK (contribution_type IN ('upload', 'nats_publish', 'agent_output', 'sync')),
    channel TEXT NOT NULL DEFAULT 'http' CHECK (channel IN ('http', 'nats', 'pcg_sync', 'agent')),
    project_id TEXT,
    task_id TEXT,
    description TEXT,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_cloud_contribs_org ON cloud_contributions(organization_id);
CREATE INDEX IF NOT EXISTS idx_cloud_contribs_file ON cloud_contributions(cloud_file_id);
CREATE INDEX IF NOT EXISTS idx_cloud_contribs_user ON cloud_contributions(user_id);

CREATE TABLE IF NOT EXISTS org_cloud_settings (
    organization_id TEXT PRIMARY KEY NOT NULL,
    storage_quota_bytes INTEGER NOT NULL DEFAULT 10737418240,
    viewer_can_download INTEGER NOT NULL DEFAULT 1,
    member_can_upload INTEGER NOT NULL DEFAULT 1,
    auto_index_data_sources INTEGER NOT NULL DEFAULT 1,
    auto_index_artifacts INTEGER NOT NULL DEFAULT 1,
    auto_index_media INTEGER NOT NULL DEFAULT 1,
    nats_contribution_enabled INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
