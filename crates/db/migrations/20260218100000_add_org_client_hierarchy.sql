-- Add Organization → Client → Project hierarchy
-- Creates clients table, client_members table, and links projects to organizations and clients

-- ============================================================================
-- PART 1: Create clients table
-- ============================================================================

CREATE TABLE IF NOT EXISTS clients (
    id BLOB PRIMARY KEY NOT NULL,
    organization_id BLOB NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    logo_url TEXT,
    website TEXT,
    crm_contact_id BLOB,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT,
    deleted_by BLOB,
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (crm_contact_id) REFERENCES crm_contacts(id) ON DELETE SET NULL,
    FOREIGN KEY (deleted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(organization_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_clients_organization_id ON clients(organization_id);
CREATE INDEX IF NOT EXISTS idx_clients_slug ON clients(slug);
CREATE INDEX IF NOT EXISTS idx_clients_crm_contact_id ON clients(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_clients_is_active ON clients(is_active);

-- ============================================================================
-- PART 2: Create client_members table (external collaborator access)
-- ============================================================================

CREATE TABLE IF NOT EXISTS client_members (
    id BLOB PRIMARY KEY NOT NULL,
    client_id BLOB NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK(role IN ('admin', 'editor', 'viewer')),
    granted_by BLOB,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(client_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_client_members_client_id ON client_members(client_id);
CREATE INDEX IF NOT EXISTS idx_client_members_user_id ON client_members(user_id);

-- ============================================================================
-- PART 3: Add organization_id and client_id to projects
-- ============================================================================

ALTER TABLE projects ADD COLUMN organization_id BLOB REFERENCES organizations(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN client_id BLOB REFERENCES clients(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_projects_organization_id ON projects(organization_id);
CREATE INDEX IF NOT EXISTS idx_projects_client_id ON projects(client_id);
