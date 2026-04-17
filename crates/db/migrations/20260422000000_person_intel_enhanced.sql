-- Enhanced contact intelligence fields
-- Adds engagement stats to social profiles, description/url to company roles.
--
-- NOTE: Original upstream migration referenced person_social_profiles and
-- person_company_roles, which were dropped during the contacts unification
-- (20260418000001). Targets updated to the contact_* replacements.

ALTER TABLE contact_social_profiles ADD COLUMN engagement_rate REAL;
ALTER TABLE contact_social_profiles ADD COLUMN post_count INTEGER;
ALTER TABLE contact_social_profiles ADD COLUMN content_themes TEXT; -- JSON array of strings
ALTER TABLE contact_social_profiles ADD COLUMN avg_views REAL;

ALTER TABLE contact_company_roles ADD COLUMN description TEXT;
ALTER TABLE contact_company_roles ADD COLUMN company_url TEXT;
ALTER TABLE contact_company_roles ADD COLUMN company_type TEXT;
