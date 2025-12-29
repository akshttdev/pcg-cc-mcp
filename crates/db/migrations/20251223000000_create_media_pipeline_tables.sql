-- Media Pipeline Tables for Editron
-- Created: 2025-12-23

-- Media Batches: Main table for tracking media ingestion
CREATE TABLE IF NOT EXISTS media_batches (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT,
    reference_name TEXT,
    source_url TEXT NOT NULL,
    storage_tier TEXT NOT NULL CHECK(storage_tier IN ('hot', 'warm', 'cold')),
    checksum_required BOOLEAN NOT NULL DEFAULT 1,
    status TEXT NOT NULL CHECK(status IN ('queued', 'downloading', 'ready', 'analyzing', 'analyzed', 'failed')),
    file_count INTEGER NOT NULL DEFAULT 0,
    total_size_bytes INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_media_batches_project_id ON media_batches(project_id);
CREATE INDEX idx_media_batches_status ON media_batches(status);
CREATE INDEX idx_media_batches_created_at ON media_batches(created_at DESC);

-- Media Files: Individual files within a batch
CREATE TABLE IF NOT EXISTS media_files (
    id TEXT PRIMARY KEY NOT NULL,
    batch_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    checksum_sha256 TEXT,
    duration_seconds REAL,
    resolution TEXT,
    codec TEXT,
    fps REAL,
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    FOREIGN KEY (batch_id) REFERENCES media_batches(id) ON DELETE CASCADE
);

CREATE INDEX idx_media_files_batch_id ON media_files(batch_id);

-- Media Batch Analysis: Analysis results for video editing
CREATE TABLE IF NOT EXISTS media_batch_analyses (
    id TEXT PRIMARY KEY NOT NULL,
    batch_id TEXT NOT NULL,
    brief TEXT NOT NULL,
    summary TEXT NOT NULL,
    passes_completed INTEGER NOT NULL,
    deliverable_targets TEXT NOT NULL DEFAULT '[]',
    hero_moments TEXT NOT NULL DEFAULT '[]',
    insights TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    FOREIGN KEY (batch_id) REFERENCES media_batches(id) ON DELETE CASCADE
);

CREATE INDEX idx_media_batch_analyses_batch_id ON media_batch_analyses(batch_id);

-- Edit Sessions: Video editing sessions
CREATE TABLE IF NOT EXISTS edit_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    batch_id TEXT NOT NULL,
    deliverable_type TEXT NOT NULL,
    aspect_ratios TEXT NOT NULL DEFAULT '[]',
    reference_style TEXT,
    include_captions BOOLEAN NOT NULL DEFAULT 0,
    imovie_project TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('assembling', 'needsreview', 'approved', 'rendering', 'complete', 'failed')),
    timelines TEXT NOT NULL DEFAULT '[]',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (batch_id) REFERENCES media_batches(id) ON DELETE CASCADE
);

CREATE INDEX idx_edit_sessions_batch_id ON edit_sessions(batch_id);
CREATE INDEX idx_edit_sessions_status ON edit_sessions(status);

-- Render Jobs: Export jobs for video deliverables
CREATE TABLE IF NOT EXISTS render_jobs (
    id TEXT PRIMARY KEY NOT NULL,
    edit_session_id TEXT NOT NULL,
    destinations TEXT NOT NULL DEFAULT '[]',
    formats TEXT NOT NULL DEFAULT '[]',
    priority TEXT NOT NULL CHECK(priority IN ('low', 'standard', 'rush')),
    status TEXT NOT NULL CHECK(status IN ('queued', 'rendering', 'complete', 'failed')),
    progress_percent REAL,
    last_error TEXT,
    output_urls TEXT NOT NULL DEFAULT '[]',
    metadata TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (edit_session_id) REFERENCES edit_sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_render_jobs_edit_session_id ON render_jobs(edit_session_id);
CREATE INDEX idx_render_jobs_status ON render_jobs(status);
CREATE INDEX idx_render_jobs_priority ON render_jobs(priority);
