-- Sprint 4: Audience intelligence
-- Stores clustered topic/sentiment insights derived from social_mentions via
-- the MentionClusterer service. Powers FAQ content opportunities and crisis detection.

CREATE TABLE IF NOT EXISTS social_audience_insights (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    social_account_id TEXT REFERENCES social_accounts(id) ON DELETE CASCADE,

    insight_type TEXT NOT NULL,
    -- faq | objection | praise | feature_request | topic_interest | sentiment_shift

    topic           TEXT NOT NULL,
    mention_count   INTEGER NOT NULL DEFAULT 1,
    sample_mentions TEXT,   -- JSON array of anonymized example texts
    sentiment       TEXT,   -- positive | neutral | negative | mixed
    sentiment_score REAL,   -- -1.0 to 1.0

    suggested_content_angle TEXT,
    opportunity_id TEXT REFERENCES social_content_opportunities(id) ON DELETE SET NULL,

    period_start TEXT NOT NULL,
    period_end   TEXT NOT NULL,
    actioned     INTEGER NOT NULL DEFAULT 0,
    dismissed    INTEGER NOT NULL DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_audience_insights_org
    ON social_audience_insights(organization_id);
CREATE INDEX IF NOT EXISTS idx_audience_insights_account
    ON social_audience_insights(social_account_id);
CREATE INDEX IF NOT EXISTS idx_audience_insights_period
    ON social_audience_insights(organization_id, period_end DESC);
