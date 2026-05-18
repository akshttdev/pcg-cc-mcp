-- Social account snapshots — periodic follower/engagement captures for trend analytics.
-- A background collector writes one row per account per day.

CREATE TABLE IF NOT EXISTS social_account_snapshots (
    id              TEXT PRIMARY KEY NOT NULL,
    account_id      TEXT NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
    project_id      TEXT NOT NULL,
    platform        TEXT NOT NULL,
    snapshot_date   TEXT NOT NULL,                    -- YYYY-MM-DD
    follower_count  INTEGER,
    following_count INTEGER,
    post_count      INTEGER,
    avg_engagement  REAL,                             -- average engagement rate this day
    impressions     INTEGER NOT NULL DEFAULT 0,
    reach           INTEGER NOT NULL DEFAULT 0,
    likes           INTEGER NOT NULL DEFAULT 0,
    comments        INTEGER NOT NULL DEFAULT 0,
    shares          INTEGER NOT NULL DEFAULT 0,
    saves           INTEGER NOT NULL DEFAULT 0,
    clicks          INTEGER NOT NULL DEFAULT 0,
    posts_published INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(account_id, snapshot_date)
);

CREATE INDEX IF NOT EXISTS idx_social_account_snapshots_account
    ON social_account_snapshots(account_id, snapshot_date DESC);
CREATE INDEX IF NOT EXISTS idx_social_account_snapshots_project
    ON social_account_snapshots(project_id, snapshot_date DESC);
