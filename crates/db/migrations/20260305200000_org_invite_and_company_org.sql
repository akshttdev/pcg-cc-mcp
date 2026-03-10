-- Link a person to the org that represents their company (not the managing org)
ALTER TABLE persons ADD COLUMN company_org_id BLOB REFERENCES organizations(id);

-- Invite system on orgs (UNIQUE must be a separate index in SQLite ALTER TABLE)
ALTER TABLE organizations ADD COLUMN invite_token TEXT;
ALTER TABLE organizations ADD COLUMN pending_owner_email TEXT;
ALTER TABLE organizations ADD COLUMN created_by_org_id BLOB REFERENCES organizations(id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_organizations_invite_token
    ON organizations(invite_token) WHERE invite_token IS NOT NULL;
