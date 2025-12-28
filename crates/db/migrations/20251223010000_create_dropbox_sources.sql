-- Dropbox sources map webhook accounts to ingest instructions
CREATE TABLE IF NOT EXISTS dropbox_sources (
    id TEXT PRIMARY KEY NOT NULL,
    account_id TEXT NOT NULL,
    label TEXT NOT NULL,
    source_url TEXT,
    project_id TEXT,
    storage_tier TEXT NOT NULL CHECK(storage_tier IN ('hot', 'warm', 'cold')),
    checksum_required BOOLEAN NOT NULL DEFAULT 1,
    reference_name_template TEXT,
    ingest_strategy TEXT NOT NULL DEFAULT 'shared_link',
    access_token TEXT,
    cursor TEXT,
    last_processed_at TEXT,
    auto_ingest BOOLEAN NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_dropbox_sources_account ON dropbox_sources(account_id);
