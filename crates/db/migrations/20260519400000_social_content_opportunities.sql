-- Sprint 3: Content opportunity pipeline
-- Links detected signals (trending topics, KOL content, news events, competitor gaps,
-- audience questions) to auto-drafted social posts for human review.

CREATE TABLE IF NOT EXISTS social_content_opportunities (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,

    -- Signal origin
    opportunity_type TEXT NOT NULL,
    -- trending_topic | kol_signal | news_event | audience_question
    -- competitor_gap | performance_pattern | thought_leader_angle | manual

    signal_source_type TEXT,
    -- pulse_content_item | social_mention_cluster | snapshot_analysis | manual
    signal_source_id   TEXT,
    tracked_entity_id  TEXT REFERENCES social_tracked_entities(id) ON DELETE SET NULL,

    -- Content brief (AI-generated)
    title               TEXT NOT NULL,
    brief               TEXT,
    suggested_format    TEXT,        -- post | carousel | reel | story | thread
    suggested_platforms TEXT,        -- JSON array
    suggested_slot      TEXT,        -- ISO datetime recommendation
    suggested_hashtags  TEXT,        -- JSON array

    -- Draft linkage
    draft_post_id TEXT REFERENCES social_posts(id) ON DELETE SET NULL,

    -- Lifecycle
    status          TEXT NOT NULL DEFAULT 'detected',
    -- detected → briefed → draft_created → approved → rejected → scheduled | expired
    relevance_score REAL NOT NULL DEFAULT 0.5,
    expires_at      TEXT,

    -- Review
    reviewed_by   TEXT REFERENCES users(id) ON DELETE SET NULL,
    reviewed_at   TEXT,
    review_notes  TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_opportunities_org
    ON social_content_opportunities(organization_id);
CREATE INDEX IF NOT EXISTS idx_opportunities_status
    ON social_content_opportunities(status);
CREATE INDEX IF NOT EXISTS idx_opportunities_type
    ON social_content_opportunities(opportunity_type);
CREATE INDEX IF NOT EXISTS idx_opportunities_expires
    ON social_content_opportunities(expires_at)
    WHERE expires_at IS NOT NULL;
