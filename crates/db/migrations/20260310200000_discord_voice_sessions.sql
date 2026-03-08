-- Discord Voice Session support
-- Adds source_type to meeting_sessions so Discord sessions are distinguishable
-- Discord-specific metadata (guild_id, channel_id, channel_name, bot_user) stored in existing metadata JSON column

ALTER TABLE meeting_sessions ADD COLUMN source_type TEXT NOT NULL DEFAULT 'browser';

CREATE INDEX IF NOT EXISTS idx_meeting_sessions_source_type ON meeting_sessions(source_type);
CREATE INDEX IF NOT EXISTS idx_meeting_sessions_status ON meeting_sessions(status);
