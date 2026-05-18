-- Sprint 2: Tracked entities + source_category intent layer
-- social_tracked_entities: named entities (competitors, KOLs, press, thought leaders)
--   that feed the intelligence signal processors.
-- source_category: added to pulse_sources as the intent layer on top of source_type
--   (transport). A KOL tracked via Twitter, YouTube, and RSS all get source_category
--   = 'kol_influencer' regardless of transport.

ALTER TABLE pulse_sources ADD COLUMN source_category TEXT NOT NULL DEFAULT 'keyword_feed';
-- Backfill: existing rows keep 'keyword_feed' as default (general monitoring)
-- values: competitor | kol_influencer | thought_leader | trade_press | keyword_feed | owned_mentions

CREATE TABLE IF NOT EXISTS social_tracked_entities (
    id              TEXT PRIMARY KEY,
    organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE,
    project_id      TEXT REFERENCES projects(id) ON DELETE SET NULL,
    entity_type     TEXT NOT NULL,
    -- competitor | kol_influencer | thought_leader | trade_press
    name            TEXT NOT NULL,
    description     TEXT,
    instagram_handle TEXT,
    twitter_handle   TEXT,
    linkedin_url     TEXT,
    tiktok_handle    TEXT,
    youtube_channel  TEXT,
    website_url      TEXT,
    crm_contact_id  TEXT REFERENCES crm_contacts(id) ON DELETE SET NULL,
    follower_estimate INTEGER,
    niche_relevance   REAL NOT NULL DEFAULT 0.5,
    is_active         INTEGER NOT NULL DEFAULT 1,
    notes             TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_tracked_entities_org
    ON social_tracked_entities(organization_id);
CREATE INDEX IF NOT EXISTS idx_tracked_entities_type
    ON social_tracked_entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_tracked_entities_active
    ON social_tracked_entities(organization_id, is_active);
