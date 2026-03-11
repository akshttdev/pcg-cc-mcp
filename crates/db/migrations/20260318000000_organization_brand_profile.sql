-- Organization Brand Profile
-- Comprehensive brand identity table scoped to an organization.
-- Covers everything needed to inform website, social, and operational development.

CREATE TABLE IF NOT EXISTS organization_brand_profiles (
    id                    BLOB PRIMARY KEY NOT NULL,
    organization_id       BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,

    -- Visual Identity
    tagline               TEXT,
    primary_color         TEXT NOT NULL DEFAULT '#2563EB',
    secondary_color       TEXT NOT NULL DEFAULT '#EC4899',
    accent_color          TEXT,
    typography_heading    TEXT,
    typography_body       TEXT,
    logo_url              TEXT,

    -- Positioning
    industry              TEXT,
    market_position       TEXT,   -- luxury | premium | mid-market | budget
    unique_value_proposition TEXT,
    mission_statement     TEXT,
    vision_statement      TEXT,
    brand_values          TEXT,   -- JSON array of strings
    brand_voice           TEXT,   -- formal | casual | playful | authoritative | bold | sophisticated
    brand_archetype       TEXT,   -- Hero | Creator | Sage | Outlaw | Explorer | Ruler | Caregiver | Innocent | Jester | Lover | Magician | Regular Guy

    -- Audience
    target_audience       TEXT,   -- free-form description
    icp_description       TEXT,   -- ideal customer profile
    icp_company_size      TEXT,   -- solo | startup | smb | mid-market | enterprise
    icp_industries        TEXT,   -- JSON array

    -- Competitive
    competitor_brands     TEXT,   -- JSON array of {name, url} objects
    differentiators       TEXT,   -- JSON array of strings

    -- Content & Social
    content_pillars       TEXT,   -- JSON array of strings
    content_tone          TEXT,   -- free-form notes on tone
    website_url           TEXT,
    social_instagram      TEXT,
    social_twitter        TEXT,
    social_linkedin       TEXT,
    social_facebook       TEXT,
    social_youtube        TEXT,
    social_tiktok         TEXT,

    created_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at            TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(organization_id)
);
