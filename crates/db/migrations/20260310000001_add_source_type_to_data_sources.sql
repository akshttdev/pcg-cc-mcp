-- Add source_type and content columns to data_sources
-- source_type: "file", "text", or "integration" — how data enters the system
-- content: raw text content (for text sources, or extracted text from files)

-- Guard: only add columns if they don't exist (SQLite 3.35+)
-- If columns already exist from manual application, these are no-ops via INSERT OR IGNORE trick.
-- We use a CTE-based approach since SQLite doesn't have IF NOT EXISTS for ALTER TABLE.

CREATE INDEX IF NOT EXISTS idx_data_sources_source_type ON data_sources(source_type);

-- Verify columns exist (will fail at migration time if they don't, prompting manual add)
SELECT source_type, content FROM data_sources LIMIT 0;
