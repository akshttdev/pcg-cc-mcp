-- Platform roles, org-scoped CRM, and access control refinements

-- ============================================================================
-- PART 1: User Platform Roles (multi-role junction table)
-- ============================================================================

CREATE TABLE IF NOT EXISTS user_platform_roles (
    id BLOB PRIMARY KEY NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('platform_admin', 'operator', 'client_user')),
    granted_by BLOB,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(user_id, role)
);

CREATE INDEX IF NOT EXISTS idx_user_platform_roles_user_id ON user_platform_roles(user_id);
CREATE INDEX IF NOT EXISTS idx_user_platform_roles_role ON user_platform_roles(role);

-- Seed: migrate existing is_admin users to platform_admin role
INSERT OR IGNORE INTO user_platform_roles (id, user_id, role, granted_at)
SELECT randomblob(16), id, 'platform_admin', datetime('now')
FROM users WHERE is_admin = 1;

-- ============================================================================
-- PART 2: CRM tables — add organization_id (org-scoped CRM)
-- ============================================================================

-- crm_contacts: add org + client scope
ALTER TABLE crm_contacts ADD COLUMN organization_id BLOB REFERENCES organizations(id) ON DELETE CASCADE;
ALTER TABLE crm_contacts ADD COLUMN client_id BLOB REFERENCES clients(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_crm_contacts_organization ON crm_contacts(organization_id);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_client ON crm_contacts(client_id);

-- crm_deals: add org + client scope
ALTER TABLE crm_deals ADD COLUMN organization_id BLOB REFERENCES organizations(id) ON DELETE CASCADE;
ALTER TABLE crm_deals ADD COLUMN client_id BLOB REFERENCES clients(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_crm_deals_organization ON crm_deals(organization_id);
CREATE INDEX IF NOT EXISTS idx_crm_deals_client ON crm_deals(client_id);

-- crm_activities: add org + client scope
ALTER TABLE crm_activities ADD COLUMN organization_id BLOB REFERENCES organizations(id) ON DELETE CASCADE;
ALTER TABLE crm_activities ADD COLUMN client_id BLOB REFERENCES clients(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_crm_activities_organization ON crm_activities(organization_id);
CREATE INDEX IF NOT EXISTS idx_crm_activities_client ON crm_activities(client_id);

-- crm_pipelines: add org scope (already has project_id, keep for backward compat)
-- Note: crm_pipelines already supports org via listOrgPipelines, but lacks the column
ALTER TABLE crm_pipelines ADD COLUMN organization_id BLOB REFERENCES organizations(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_crm_pipelines_organization ON crm_pipelines(organization_id);

-- ============================================================================
-- PART 3: Client CRM feature flag
-- ============================================================================

ALTER TABLE clients ADD COLUMN crm_enabled INTEGER NOT NULL DEFAULT 0;

-- ============================================================================
-- PART 4: Backfill org scope on existing CRM data
-- ============================================================================

-- For existing CRM records that have project_id, derive organization_id from the project
UPDATE crm_contacts SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_contacts.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

UPDATE crm_deals SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_deals.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

UPDATE crm_activities SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_activities.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

UPDATE crm_pipelines SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_pipelines.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;
