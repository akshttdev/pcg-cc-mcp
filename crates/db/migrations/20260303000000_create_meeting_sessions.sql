-- Create meeting sessions and segments tables
-- These tables were referenced by later migrations (20260304000000+) but never created.

CREATE TABLE IF NOT EXISTS meeting_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    title TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'active',
    started_by TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    ended_at TEXT,
    duration_seconds INTEGER,
    participant_count INTEGER,
    participants TEXT,
    transcript TEXT,
    notes TEXT,
    metadata TEXT,
    topology_node_id TEXT,
    access_level TEXT,
    shared_with TEXT,
    linked_person_ids TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS meeting_segments (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_session_id TEXT NOT NULL REFERENCES meeting_sessions(id) ON DELETE CASCADE,
    segment_index INTEGER NOT NULL,
    speaker_label TEXT,
    text TEXT NOT NULL,
    confidence REAL,
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER NOT NULL,
    is_topsi_addressed BOOLEAN NOT NULL DEFAULT 0,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_meeting_sessions_project ON meeting_sessions(project_id);
-- idx_meeting_sessions_status created in 20260310200000
CREATE INDEX IF NOT EXISTS idx_meeting_segments_session ON meeting_segments(meeting_session_id);
