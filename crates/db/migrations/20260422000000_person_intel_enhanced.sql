-- Enhanced person intelligence fields
-- Adds engagement stats to social profiles, description/url to company roles

ALTER TABLE person_social_profiles ADD COLUMN engagement_rate REAL;
ALTER TABLE person_social_profiles ADD COLUMN post_count INTEGER;
ALTER TABLE person_social_profiles ADD COLUMN content_themes TEXT; -- JSON array of strings
ALTER TABLE person_social_profiles ADD COLUMN avg_views REAL;

ALTER TABLE person_company_roles ADD COLUMN description TEXT;
ALTER TABLE person_company_roles ADD COLUMN company_url TEXT;
ALTER TABLE person_company_roles ADD COLUMN company_type TEXT;
