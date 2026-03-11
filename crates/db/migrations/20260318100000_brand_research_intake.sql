-- Brand research status fields on org_brand_profiles
ALTER TABLE organization_brand_profiles ADD COLUMN research_status TEXT NOT NULL DEFAULT 'idle';
ALTER TABLE organization_brand_profiles ADD COLUMN research_ran_at TEXT;
ALTER TABLE organization_brand_profiles ADD COLUMN research_summary TEXT;

-- Client intake tokens (shareable questionnaire links)
CREATE TABLE IF NOT EXISTS brand_intake_tokens (
    id              BLOB PRIMARY KEY,
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    token           TEXT NOT NULL UNIQUE,
    expires_at      TEXT NOT NULL,
    used_at         TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);
