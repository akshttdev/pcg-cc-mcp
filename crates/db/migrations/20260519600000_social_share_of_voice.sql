-- Sprint 5: Share of voice + KOL→CRM bridge
-- Weekly SOV snapshots: own mention count vs. niche total, per platform.
-- Competitor summary stored as JSON for flexible rendering.

CREATE TABLE IF NOT EXISTS social_share_of_voice (
    id              TEXT PRIMARY KEY,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    period_start    TEXT NOT NULL,
    period_end      TEXT NOT NULL,
    keyword_set     TEXT NOT NULL,  -- JSON array of tracked keywords/hashtags
    platform        TEXT,           -- NULL = cross-platform aggregate

    own_mention_count         INTEGER NOT NULL DEFAULT 0,
    total_niche_mention_count INTEGER NOT NULL DEFAULT 0,
    share_of_voice_pct        REAL,

    -- Sentiment breakdown
    own_positive_pct  REAL,
    own_neutral_pct   REAL,
    own_negative_pct  REAL,
    niche_positive_pct REAL,
    niche_neutral_pct  REAL,
    niche_negative_pct REAL,

    -- Top signals
    top_keywords      TEXT,  -- JSON [{keyword, count, velocity}]
    competitor_summary TEXT, -- JSON [{entity_id, name, mention_count, share_pct}]

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_sov_org_period
    ON social_share_of_voice(organization_id, period_end DESC);
CREATE INDEX IF NOT EXISTS idx_sov_platform
    ON social_share_of_voice(organization_id, platform);
