-- Migration: extend cloud_storage_accounts.provider CHECK to include 'local'.
--
-- The original 20260430000000 migration locked provider to the three cloud
-- vendors. Master nodes that already host a cloned tree on local disk (e.g.
-- E:/topos/Sirak Studios Dropbox/) don't need an OAuth round-trip; the
-- LocalFolderConnector walks the path directly. SQLite can't ALTER a CHECK
-- constraint, so we rebuild the table.

PRAGMA foreign_keys = OFF;

CREATE TABLE cloud_storage_accounts_new (
    id                        TEXT PRIMARY KEY NOT NULL,
    organization_id           TEXT NOT NULL,
    project_id                TEXT,
    integration_connection_id TEXT REFERENCES integration_connections(id) ON DELETE SET NULL,

    provider                  TEXT NOT NULL
        CHECK (provider IN ('onedrive','dropbox','gdrive','local')),
    account_email             TEXT,
    display_name              TEXT,

    sync_cursor               TEXT,
    sync_root_path            TEXT,
    auto_sync                 INTEGER NOT NULL DEFAULT 1,
    sync_interval_secs        INTEGER NOT NULL DEFAULT 900,
    last_sync_at              TEXT,
    last_error                TEXT,
    status                    TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active','syncing','expired','error','disconnected')),

    total_files_synced        INTEGER NOT NULL DEFAULT 0,
    metadata                  TEXT NOT NULL DEFAULT '{}',

    created_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),

    UNIQUE(organization_id, provider, account_email)
);

INSERT INTO cloud_storage_accounts_new (
    id, organization_id, project_id, integration_connection_id,
    provider, account_email, display_name,
    sync_cursor, sync_root_path, auto_sync, sync_interval_secs,
    last_sync_at, last_error, status,
    total_files_synced, metadata, created_at, updated_at
)
SELECT
    id, organization_id, project_id, integration_connection_id,
    provider, account_email, display_name,
    sync_cursor, sync_root_path, auto_sync, sync_interval_secs,
    last_sync_at, last_error, status,
    total_files_synced, metadata, created_at, updated_at
FROM cloud_storage_accounts;

DROP TABLE cloud_storage_accounts;
ALTER TABLE cloud_storage_accounts_new RENAME TO cloud_storage_accounts;

CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_org
    ON cloud_storage_accounts(organization_id);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_provider
    ON cloud_storage_accounts(provider);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_connection
    ON cloud_storage_accounts(integration_connection_id);
CREATE INDEX IF NOT EXISTS idx_cloud_storage_accounts_due
    ON cloud_storage_accounts(last_sync_at)
    WHERE auto_sync = 1 AND status = 'active';

PRAGMA foreign_keys = ON;
