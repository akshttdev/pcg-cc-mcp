-- Research depth tracking and extended brand intelligence fields
ALTER TABLE organization_brand_profiles ADD COLUMN research_iterations INTEGER NOT NULL DEFAULT 0;
ALTER TABLE organization_brand_profiles ADD COLUMN research_depth INTEGER NOT NULL DEFAULT 0;
ALTER TABLE organization_brand_profiles ADD COLUMN founder_name TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN founding_year TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN key_clients TEXT; -- JSON array
ALTER TABLE organization_brand_profiles ADD COLUMN estimated_team_size TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN tech_stack TEXT; -- JSON array
ALTER TABLE organization_brand_profiles ADD COLUMN geographic_focus TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN funding_stage TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN content_strategy_notes TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN awards_and_recognition TEXT; -- JSON array
ALTER TABLE organization_brand_profiles ADD COLUMN brand_gap_notes TEXT; -- what's still unknown
