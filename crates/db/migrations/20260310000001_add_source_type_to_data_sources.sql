-- Add source_type and content columns to data_sources
-- source_type: "file", "text", or "integration" — how data enters the system
-- content: raw text content (for text sources, or extracted text from files)

-- SQLite doesn't error on duplicate ADD COLUMN if we guard with a no-op select.
-- But sqlx runs migrations only once, so this is safe.
ALTER TABLE data_sources ADD COLUMN source_type TEXT NOT NULL DEFAULT 'file';
ALTER TABLE data_sources ADD COLUMN content TEXT;

CREATE INDEX IF NOT EXISTS idx_data_sources_source_type ON data_sources(source_type);
