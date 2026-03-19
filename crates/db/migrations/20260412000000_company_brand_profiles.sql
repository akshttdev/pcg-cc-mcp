-- Company Brand Profiles
-- Mirrors organization_brand_profiles but keyed on company_id.
-- Populated during discovery/research for client companies.

CREATE TABLE IF NOT EXISTS company_brand_profiles (
    id                      TEXT PRIMARY KEY NOT NULL,
    company_id              TEXT NOT NULL UNIQUE REFERENCES companies(id) ON DELETE CASCADE,

    -- Identity
    tagline                 TEXT,
    primary_color           TEXT NOT NULL DEFAULT '#2563EB',
    secondary_color         TEXT NOT NULL DEFAULT '#EC4899',
    accent_color            TEXT,
    typography_heading      TEXT,
    typography_body         TEXT,
    logo_url                TEXT,

    -- Positioning
    industry                TEXT,
    market_position         TEXT,
    unique_value_proposition TEXT,
    mission_statement       TEXT,
    vision_statement        TEXT,
    brand_values            TEXT,
    brand_voice             TEXT,
    brand_archetype         TEXT,

    -- Audience
    target_audience         TEXT,
    icp_description         TEXT,
    icp_company_size        TEXT,
    icp_industries          TEXT,

    -- Competitive
    competitor_brands       TEXT,
    differentiators         TEXT,

    -- Content
    content_pillars         TEXT,
    content_tone            TEXT,

    -- Social
    website_url             TEXT,
    social_instagram        TEXT,
    social_twitter          TEXT,
    social_linkedin         TEXT,
    social_facebook         TEXT,
    social_youtube          TEXT,
    social_tiktok           TEXT,

    -- Research
    research_status         TEXT NOT NULL DEFAULT 'idle',
    research_ran_at         TEXT,
    research_summary        TEXT,
    research_iterations     INTEGER NOT NULL DEFAULT 0,
    research_depth          INTEGER NOT NULL DEFAULT 0,

    -- Extra
    founder_name            TEXT,
    founding_year           TEXT,
    key_clients             TEXT,
    estimated_team_size     TEXT,
    tech_stack              TEXT,
    geographic_focus        TEXT,
    funding_stage           TEXT,
    content_strategy_notes  TEXT,
    awards_and_recognition  TEXT,
    brand_gap_notes         TEXT,
    mood_board_urls         TEXT,
    clearbit_logo_url       TEXT,
    brand_photography_notes TEXT,

    created_at              TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_company_brand_profiles_company
    ON company_brand_profiles(company_id);
