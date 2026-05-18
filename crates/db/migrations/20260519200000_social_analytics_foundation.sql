-- Sprint 1: Analytics foundation
-- Adds learned optimal posting time benchmarks per account.
-- SnapshotAnalyzer service populates this from social_account_snapshots
-- and social_post_metrics_snapshots. Scheduler reads it when confidence > 0.6.

CREATE TABLE IF NOT EXISTS social_performance_benchmarks (
    id                  TEXT PRIMARY KEY,
    social_account_id   TEXT NOT NULL REFERENCES social_accounts(id) ON DELETE CASCADE,
    day_of_week         INTEGER NOT NULL,  -- 0=Sunday, 6=Saturday
    hour_of_day         INTEGER NOT NULL,  -- 0-23 UTC
    avg_engagement_rate REAL NOT NULL DEFAULT 0.0,
    avg_reach           REAL NOT NULL DEFAULT 0.0,
    avg_saves           REAL NOT NULL DEFAULT 0.0,
    avg_impressions     REAL NOT NULL DEFAULT 0.0,
    sample_count        INTEGER NOT NULL DEFAULT 0,
    -- confidence rises with sample_count; scheduler uses learned times when > 0.6
    confidence          REAL NOT NULL DEFAULT 0.0,
    updated_at          TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(social_account_id, day_of_week, hour_of_day)
);

CREATE INDEX IF NOT EXISTS idx_benchmarks_account
    ON social_performance_benchmarks(social_account_id);
CREATE INDEX IF NOT EXISTS idx_benchmarks_account_confidence
    ON social_performance_benchmarks(social_account_id, confidence);
