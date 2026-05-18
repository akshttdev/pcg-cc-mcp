-- Per-post time-series metrics snapshots for growth curve analytics.
-- The metrics sync worker inserts one row per sync cycle per post.
CREATE TABLE IF NOT EXISTS social_post_metrics_snapshots (
    id              BLOB PRIMARY KEY NOT NULL,
    post_id         BLOB NOT NULL REFERENCES social_posts(id) ON DELETE CASCADE,
    social_account_id BLOB,
    platform        TEXT NOT NULL,
    captured_at     TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    impressions     INTEGER NOT NULL DEFAULT 0,
    reach           INTEGER NOT NULL DEFAULT 0,
    likes           INTEGER NOT NULL DEFAULT 0,
    comments        INTEGER NOT NULL DEFAULT 0,
    shares          INTEGER NOT NULL DEFAULT 0,
    saves           INTEGER NOT NULL DEFAULT 0,
    clicks          INTEGER NOT NULL DEFAULT 0,
    video_views     INTEGER NOT NULL DEFAULT 0,
    engagement_rate REAL NOT NULL DEFAULT 0.0
);
CREATE INDEX IF NOT EXISTS idx_post_metrics_post_time
    ON social_post_metrics_snapshots(post_id, captured_at DESC);
