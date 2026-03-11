-- Link a person to the org that represents their company (not the managing org)
-- NOTE: All columns below already exist in live DB. ALTERs skipped to avoid duplicate column errors.
-- ALTER TABLE persons ADD COLUMN company_org_id BLOB REFERENCES organizations(id);
-- ALTER TABLE organizations ADD COLUMN invite_token TEXT;
-- ALTER TABLE organizations ADD COLUMN pending_owner_email TEXT;
-- ALTER TABLE organizations ADD COLUMN created_by_org_id BLOB REFERENCES organizations(id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_organizations_invite_token
    ON organizations(invite_token) WHERE invite_token IS NOT NULL;
