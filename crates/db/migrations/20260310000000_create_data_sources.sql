-- Data Sources: uploadable knowledge sources with flexible metadata
CREATE TABLE IF NOT EXISTS data_sources (
    id              BLOB PRIMARY KEY NOT NULL,          -- UUID
    organization_id BLOB,                               -- optional org scope
    project_id      BLOB,                               -- optional project scope
    created_by      BLOB,                               -- user who uploaded

    -- Core fields
    title           TEXT NOT NULL,
    description     TEXT,
    data_type       TEXT NOT NULL DEFAULT 'conversation', -- conversation, document, transcript, report, dataset, media, other
    file_type       TEXT,                                -- mime-like: text/plain, application/pdf, text/csv, audio/mp3, etc.

    -- File storage
    file_name       TEXT,                                -- original upload filename
    file_path       TEXT,                                -- relative path on disk
    file_size_bytes INTEGER,                             -- size in bytes
    file_hash       TEXT,                                -- SHA-256 for dedup

    -- Flexible metadata (JSON blob — schema varies by data_type + file_type)
    metadata        TEXT NOT NULL DEFAULT '{}',           -- JSON object

    -- Processing status
    status          TEXT NOT NULL DEFAULT 'pending',      -- pending, processing, ready, error
    processing_error TEXT,

    -- Timestamps
    created_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    archived_at     DATETIME,

    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_data_sources_org ON data_sources(organization_id) WHERE organization_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_data_sources_project ON data_sources(project_id) WHERE project_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_data_sources_data_type ON data_sources(data_type);
CREATE INDEX IF NOT EXISTS idx_data_sources_status ON data_sources(status);
CREATE INDEX IF NOT EXISTS idx_data_sources_created_at ON data_sources(created_at DESC);
