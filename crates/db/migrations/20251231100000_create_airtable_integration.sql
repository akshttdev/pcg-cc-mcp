-- Airtable base connections: links a PCG project to an Airtable base
CREATE TABLE IF NOT EXISTS airtable_bases (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    airtable_base_id TEXT NOT NULL,
    airtable_base_name TEXT,
    sync_enabled INTEGER NOT NULL DEFAULT 1,
    default_table_id TEXT,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, airtable_base_id)
);

-- Task-level Airtable record mappings
CREATE TABLE IF NOT EXISTS airtable_record_links (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL UNIQUE,
    airtable_record_id TEXT NOT NULL,
    airtable_base_id TEXT NOT NULL,
    airtable_table_id TEXT,
    origin TEXT NOT NULL DEFAULT 'airtable' CHECK(origin IN ('airtable', 'pcg')),
    sync_status TEXT NOT NULL DEFAULT 'synced' CHECK(sync_status IN ('synced', 'pending_push', 'pending_pull', 'error')),
    last_sync_error TEXT,
    airtable_record_url TEXT,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_airtable_bases_project ON airtable_bases(project_id);
CREATE INDEX IF NOT EXISTS idx_airtable_record_links_task ON airtable_record_links(task_id);
CREATE INDEX IF NOT EXISTS idx_airtable_record_links_record ON airtable_record_links(airtable_record_id);
CREATE INDEX IF NOT EXISTS idx_airtable_record_links_origin ON airtable_record_links(origin);
CREATE INDEX IF NOT EXISTS idx_airtable_record_links_sync_status ON airtable_record_links(sync_status);
