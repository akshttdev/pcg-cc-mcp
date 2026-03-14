-- Sync Folders: org-scoped shared folders that map to local filesystem paths
-- Similar to Dropbox/OneDrive shared folders
CREATE TABLE IF NOT EXISTS sync_folders (
    id              BLOB PRIMARY KEY NOT NULL,       -- UUID
    organization_id BLOB NOT NULL,                   -- owning org
    parent_id       BLOB,                            -- parent folder (NULL = root)
    name            TEXT NOT NULL,                    -- folder display name
    path            TEXT NOT NULL,                    -- full logical path e.g. "/Clients/Acme/Assets"
    description     TEXT,
    is_shared       BOOLEAN NOT NULL DEFAULT 1,      -- visible to all org members
    auto_sync       BOOLEAN NOT NULL DEFAULT 1,      -- auto-sync to connected devices
    max_depth       INTEGER,                         -- max nesting depth to sync (NULL = unlimited)
    created_by      BLOB,                            -- user who created
    created_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),
    archived_at     DATETIME,

    FOREIGN KEY (organization_id) REFERENCES organizations(id),
    FOREIGN KEY (parent_id) REFERENCES sync_folders(id)
);

CREATE INDEX IF NOT EXISTS idx_sync_folders_org ON sync_folders(organization_id);
CREATE INDEX IF NOT EXISTS idx_sync_folders_parent ON sync_folders(parent_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_folders_org_path ON sync_folders(organization_id, path)
    WHERE archived_at IS NULL;

-- Sync Devices: registered devices per user for sync
CREATE TABLE IF NOT EXISTS sync_devices (
    id              BLOB PRIMARY KEY NOT NULL,       -- UUID
    user_id         BLOB NOT NULL,                   -- device owner
    organization_id BLOB NOT NULL,                   -- org context
    device_name     TEXT NOT NULL,                    -- e.g. "Jesse's MacBook Pro"
    device_type     TEXT NOT NULL DEFAULT 'desktop',  -- desktop, laptop, server
    platform        TEXT,                             -- linux, macos, windows
    sync_folder     TEXT NOT NULL,                    -- local root path e.g. "~/PCG Files"
    last_seen_at    DATETIME,
    last_sync_at    DATETIME,
    sync_status     TEXT NOT NULL DEFAULT 'idle',     -- idle, syncing, error, paused
    sync_error      TEXT,
    is_active       BOOLEAN NOT NULL DEFAULT 1,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),

    FOREIGN KEY (user_id) REFERENCES users(id),
    FOREIGN KEY (organization_id) REFERENCES organizations(id)
);

CREATE INDEX IF NOT EXISTS idx_sync_devices_user ON sync_devices(user_id);
CREATE INDEX IF NOT EXISTS idx_sync_devices_org ON sync_devices(organization_id);

-- Sync State: per-file tracking of sync status across devices
CREATE TABLE IF NOT EXISTS sync_state (
    id              BLOB PRIMARY KEY NOT NULL,       -- UUID
    device_id       BLOB NOT NULL,                   -- which device
    data_source_id  BLOB,                            -- linked data source (if any)
    sync_folder_id  BLOB,                            -- which sync folder
    file_path       TEXT NOT NULL,                    -- relative path within sync folder
    file_hash       TEXT,                             -- SHA-256 of local copy
    file_size       INTEGER,                         -- bytes
    local_modified  DATETIME,                        -- local file mtime
    remote_modified DATETIME,                        -- server file mtime
    sync_status     TEXT NOT NULL DEFAULT 'pending',  -- pending, synced, conflict, error, deleted
    sync_direction  TEXT,                             -- pull (server→local), push (local→server)
    conflict_type   TEXT,                             -- both_modified, deleted_remotely, deleted_locally
    resolution      TEXT,                             -- keep_local, keep_remote, keep_both, NULL
    error_message   TEXT,
    synced_at       DATETIME,                        -- last successful sync
    created_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),

    FOREIGN KEY (device_id) REFERENCES sync_devices(id),
    FOREIGN KEY (data_source_id) REFERENCES data_sources(id),
    FOREIGN KEY (sync_folder_id) REFERENCES sync_folders(id)
);

CREATE INDEX IF NOT EXISTS idx_sync_state_device ON sync_state(device_id);
CREATE INDEX IF NOT EXISTS idx_sync_state_folder ON sync_state(sync_folder_id);
CREATE INDEX IF NOT EXISTS idx_sync_state_datasource ON sync_state(data_source_id);
CREATE INDEX IF NOT EXISTS idx_sync_state_status ON sync_state(sync_status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sync_state_device_path ON sync_state(device_id, file_path);

-- Device-folder subscriptions: which folders a device syncs
CREATE TABLE IF NOT EXISTS sync_subscriptions (
    id              BLOB PRIMARY KEY NOT NULL,       -- UUID
    device_id       BLOB NOT NULL,
    sync_folder_id  BLOB NOT NULL,
    is_enabled      BOOLEAN NOT NULL DEFAULT 1,
    selective_paths TEXT,                             -- JSON array of sub-paths, NULL = all
    max_file_size   INTEGER,                         -- per-file size limit (bytes)
    created_at      DATETIME NOT NULL DEFAULT (datetime('now','subsec')),

    FOREIGN KEY (device_id) REFERENCES sync_devices(id),
    FOREIGN KEY (sync_folder_id) REFERENCES sync_folders(id),
    UNIQUE(device_id, sync_folder_id)
);

CREATE INDEX IF NOT EXISTS idx_sync_subs_device ON sync_subscriptions(device_id);
CREATE INDEX IF NOT EXISTS idx_sync_subs_folder ON sync_subscriptions(sync_folder_id);
