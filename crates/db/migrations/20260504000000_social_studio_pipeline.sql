-- Add delivery pipeline fields to social_posts
ALTER TABLE social_posts ADD COLUMN deliverable_id TEXT REFERENCES deliverables(id) ON DELETE SET NULL;
ALTER TABLE social_posts ADD COLUMN review_token TEXT;
ALTER TABLE social_posts ADD COLUMN review_note TEXT;
ALTER TABLE social_posts ADD COLUMN reviewed_by TEXT;
ALTER TABLE social_posts ADD COLUMN publish_attempt INTEGER NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_social_posts_deliverable ON social_posts(deliverable_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_social_posts_review_token ON social_posts(review_token) WHERE review_token IS NOT NULL;

-- Social media assets table (distinct from video DAM media_assets)
CREATE TABLE IF NOT EXISTS social_media_assets (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    deliverable_id TEXT REFERENCES deliverables(id) ON DELETE SET NULL,
    social_post_id TEXT REFERENCES social_posts(id) ON DELETE SET NULL,
    filename TEXT NOT NULL,
    original_filename TEXT,
    storage_path TEXT NOT NULL,
    public_url TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    file_size_bytes INTEGER,
    width_px INTEGER,
    height_px INTEGER,
    uploaded_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_social_media_assets_project ON social_media_assets(project_id);
CREATE INDEX IF NOT EXISTS idx_social_media_assets_deliverable ON social_media_assets(deliverable_id);
CREATE INDEX IF NOT EXISTS idx_social_media_assets_post ON social_media_assets(social_post_id);

-- Social approval events audit trail
CREATE TABLE IF NOT EXISTS social_approval_events (
    id TEXT PRIMARY KEY,
    post_id TEXT NOT NULL REFERENCES social_posts(id) ON DELETE CASCADE,
    actor TEXT NOT NULL,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    note TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_social_approval_events_post ON social_approval_events(post_id);
