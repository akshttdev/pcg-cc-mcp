-- Make email_accounts polymorphic: owned by agent, user, organization, or project.
-- owner_type: 'agent' | 'user' | 'organization' | 'project'
-- owner_id: UUID hex of the owning entity

ALTER TABLE email_accounts ADD COLUMN owner_type TEXT NOT NULL DEFAULT 'project';
ALTER TABLE email_accounts ADD COLUMN owner_id TEXT NOT NULL DEFAULT '';

-- Back-fill owner_id from existing project_id (stored as BLOB, convert to hex string)
UPDATE email_accounts SET owner_id = lower(hex(project_id)) WHERE owner_id = '';

-- Re-tag Nora's own Zoho account as agent-owned.
-- owner_id will be stamped at runtime once we have the agent UUID,
-- but tag owner_type now so it's excluded from project lookups.
UPDATE email_accounts
SET owner_type = 'agent'
WHERE email_address = 'nora@powerclubglobal.com';

-- Tag the PCG organisation press email as organization-owned.
UPDATE email_accounts
SET owner_type = 'organization'
WHERE email_address = 'press@powerclubglobal.com';

CREATE INDEX IF NOT EXISTS idx_email_accounts_owner ON email_accounts(owner_type, owner_id);
