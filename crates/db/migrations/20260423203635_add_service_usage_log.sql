-- Service usage tracking for external APIs (music licensing, TTS, etc.)
-- Provides unified cost tracking and audit trail for all service calls

CREATE TABLE IF NOT EXISTS service_usage_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_type TEXT NOT NULL,      -- 'music', 'tts', 'stt', 'video_gen', 'search'
    provider TEXT NOT NULL,          -- 'epidemic', 'soundstripe', 'artlist', etc.
    operation TEXT NOT NULL,         -- 'search', 'get_track', 'download'
    input_units INTEGER,             -- For music: number of results requested
    output_units INTEGER,            -- For music: number of results returned
    duration_ms INTEGER,             -- API call duration
    success INTEGER NOT NULL,        -- 0 or 1
    error_message TEXT,
    metadata_json TEXT,              -- Additional context (search query, track ID, etc.)
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_service_usage_service_type ON service_usage_log(service_type);
CREATE INDEX IF NOT EXISTS idx_service_usage_provider ON service_usage_log(provider);
CREATE INDEX IF NOT EXISTS idx_service_usage_created_at ON service_usage_log(created_at);
