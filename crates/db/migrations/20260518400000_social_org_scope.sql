-- Phase A: Social calendar ownership expansion
-- Adds organization_id and user_id to social tables so accounts/posts can be
-- owned by an org (brand channels) or user (personal accounts) rather than
-- always requiring a project.  Existing rows keep their project_id unchanged.
-- Also adds client_visible gate to project_knowledge_sources.

ALTER TABLE social_accounts ADD COLUMN organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE;
ALTER TABLE social_accounts ADD COLUMN user_id TEXT REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE social_posts ADD COLUMN organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE;
ALTER TABLE social_posts ADD COLUMN user_id TEXT REFERENCES users(id) ON DELETE CASCADE;

ALTER TABLE project_knowledge_sources ADD COLUMN client_visible INTEGER NOT NULL DEFAULT 0;

ALTER TABLE social_account_snapshots ADD COLUMN organization_id TEXT REFERENCES organizations(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_social_accounts_org ON social_accounts(organization_id) WHERE organization_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_social_posts_org ON social_posts(organization_id) WHERE organization_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_kg_client_visible ON project_knowledge_sources(client_visible) WHERE client_visible = 1;
