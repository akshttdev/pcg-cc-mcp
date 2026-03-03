-- Meeting sessions and segments for Topsi meeting mode
-- Enables real-time meeting transcription with speaker identification,
-- on-demand command execution, and structured notes generation.

CREATE TABLE IF NOT EXISTS meeting_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT 'Untitled Meeting',
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','paused','ended','cancelled')),
    started_by TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    ended_at TEXT,
    duration_seconds INTEGER,
    participant_count INTEGER DEFAULT 0,
    participants TEXT DEFAULT '[]',
    transcript TEXT DEFAULT '[]',
    notes TEXT,
    metadata TEXT DEFAULT '{}',
    topology_node_id TEXT,
    access_level TEXT DEFAULT 'admin',
    shared_with TEXT DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE TABLE IF NOT EXISTS meeting_segments (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_session_id TEXT NOT NULL REFERENCES meeting_sessions(id) ON DELETE CASCADE,
    segment_index INTEGER NOT NULL,
    speaker_label TEXT,
    text TEXT NOT NULL,
    confidence REAL DEFAULT 0.0,
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER NOT NULL,
    is_topsi_addressed BOOLEAN DEFAULT 0,
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_meeting_sessions_project ON meeting_sessions(project_id);
CREATE INDEX IF NOT EXISTS idx_meeting_sessions_status ON meeting_sessions(status);
CREATE INDEX IF NOT EXISTS idx_meeting_segments_session ON meeting_segments(meeting_session_id);
CREATE INDEX IF NOT EXISTS idx_meeting_segments_index ON meeting_segments(meeting_session_id, segment_index);
