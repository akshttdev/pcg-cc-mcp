-- Create project_brand_profiles table for brand identity data
-- Separate table to avoid modifying all existing project queries

CREATE TABLE project_brand_profiles (
    id              BLOB PRIMARY KEY,
    project_id      BLOB NOT NULL UNIQUE REFERENCES projects(id) ON DELETE CASCADE,
    tagline         TEXT,
    industry        TEXT,
    primary_color   TEXT DEFAULT '#2563EB',
    secondary_color TEXT DEFAULT '#EC4899',
    brand_voice     TEXT CHECK (brand_voice IN ('formal', 'casual', 'playful', 'authoritative')),
    target_audience TEXT,
    logo_asset_id   BLOB REFERENCES project_assets(id) ON DELETE SET NULL,
    guidelines_asset_id BLOB REFERENCES project_assets(id) ON DELETE SET NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Index for querying by industry
CREATE INDEX IF NOT EXISTS idx_brand_profiles_industry ON project_brand_profiles(industry);
CREATE INDEX IF NOT EXISTS idx_brand_profiles_project ON project_brand_profiles(project_id);
