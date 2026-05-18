-- Migration: Cloud Storage Accounts (OneDrive / Dropbox / Google Drive sync state)
--
-- Provider-specific *operational* state for cloud storage syncs. The OAuth
-- credentials live in `integration_connections` (the unified token store);
-- this table tracks the bits that don't belong there: cursor, root path,
-- sync interval, last_sync_at, file counts.
--
-- One row per (organization, provider, account_email). Multiple drives per
-- provider per org are supported (e.g. two OneDrive accounts).

CREATE TABLE IF NOT EXISTS cloud_storage_accounts (
    id                        TEXT PRIMARY KEY NOT NULL,
    organization_id           TEXT NOT NULL,
    project_id                TEXT,                                   -- NULL = org-level account
    integration_connection_id TEXT REFERENCES integration_connections(id) ON DELETE SET NULL,

    provider                  TEXT NOT NULL
        CHECK (provider IN ('onedrive','dropbox','gdrive')),
    account_email             TEXT,                                   -- canonical user email (where available)
    display_name              TEXT,                                   -- human-friendly label

    sync_cursor               TEXT,                                   -- delta token / cursor / pageToken
    sync_root_path            TEXT,                                   -- NULL = drive root
    auto_sync                 INTEGER NOT NULL DEFAULT 1,
    sync_interval_secs        INTEGER NOT NULL DEFAULT 900,           -- 15 min default
    last_sync_at              TEXT,
    last_error                TEXT,
    status                    TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active','syncing','expired','error','disconnected')),

    total_files_synced        INTEGER NOT NULL DEFAULT 0,
    metadata                  TEXT NOT NULL DEFAULT '{}',             -- JSON: drive_id, team_id, etc.

    created_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),

    UNIQUE(organization_id, provider, account_email)
);

CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_org
    ON cloud_storage_accounts(organization_id);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_provider
    ON cloud_storage_accounts(provider);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_connection
    ON cloud_storage_accounts(integration_connection_id);
-- Drives the 15-min sync worker: pick rows whose interval has elapsed.
CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_due
    ON cloud_storage_accounts(last_sync_at)
    WHERE auto_sync = 1 AND status = 'active';

-- ============================================================================
-- cloud_storage_files: sidecar mapping a remote file (provider id + path) to
-- a cloud_files row. One row per (cloud_storage_account, remote_id). Lets the
-- sync worker do an O(1) lookup on incremental changes without overloading
-- cloud_files.source_id / source_table semantics.
-- ============================================================================

CREATE TABLE IF NOT EXISTS cloud_storage_files (
    id                        TEXT PRIMARY KEY NOT NULL,
    cloud_storage_account_id  TEXT NOT NULL REFERENCES cloud_storage_accounts(id) ON DELETE CASCADE,
    cloud_file_id             TEXT REFERENCES cloud_files(id) ON DELETE SET NULL,
    remote_id                 TEXT NOT NULL,                    -- provider's stable file id
    remote_path               TEXT NOT NULL DEFAULT '',          -- slash path from sync root
    remote_name               TEXT NOT NULL,
    mime_type                 TEXT,
    size_bytes                INTEGER NOT NULL DEFAULT 0,
    content_hash              TEXT,
    web_url                   TEXT,
    is_folder                 INTEGER NOT NULL DEFAULT 0,
    remote_modified_at        TEXT,
    last_seen_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at                TEXT,
    created_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(cloud_storage_account_id, remote_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_storage_files_account
    ON cloud_storage_files(cloud_storage_account_id);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_files_cloud_file
    ON cloud_storage_files(cloud_file_id);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_files_remote
    ON cloud_storage_files(cloud_storage_account_id, remote_id);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_files_path
    ON cloud_storage_files(cloud_storage_account_id, remote_path);
