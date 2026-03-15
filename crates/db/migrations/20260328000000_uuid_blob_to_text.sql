-- Convert dashboard-critical tables from BLOB UUID storage to TEXT UUID storage.
-- Pattern: create _new table with TEXT columns, INSERT with hex conversion, DROP old, RENAME.
-- Columns referencing unconverted tables (users, task_attempts, agents, etc.) also become TEXT
-- in the converted table; cross-boundary lookups parse String→Uuid in Rust code.

PRAGMA foreign_keys = OFF;

-- Drop dependent views and triggers that reference tables being recreated.
-- They will be recreated at the end of this migration.
DROP VIEW IF EXISTS v_project_knowledge_completeness;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_artifact;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_context_injection;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_entity_appearance;
-- project_boards triggers are handled inline (dropped with table, recreated after)

-- Helper: BLOB→TEXT UUID conversion expression used throughout:
-- CASE WHEN col IS NULL THEN NULL
--      WHEN typeof(col) = 'blob' THEN lower(substr(hex(col),1,8)||'-'||substr(hex(col),9,4)||'-'||substr(hex(col),13,4)||'-'||substr(hex(col),17,4)||'-'||substr(hex(col),21,12))
--      ELSE col END

-----------------------------------------------------------------------
-- 1. organizations
-----------------------------------------------------------------------
CREATE TABLE organizations_new (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,
    description TEXT,
    avatar_url TEXT,
    owner_id TEXT NOT NULL,
    settings TEXT DEFAULT '{}',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT,
    deleted_by TEXT,
    company_id TEXT,
    invite_token TEXT,
    pending_owner_email TEXT,
    created_by_org_id TEXT,
    address TEXT
);

INSERT INTO organizations_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    name, slug, description, avatar_url,
    CASE WHEN typeof(owner_id) = 'blob' THEN lower(substr(hex(owner_id),1,8)||'-'||substr(hex(owner_id),9,4)||'-'||substr(hex(owner_id),13,4)||'-'||substr(hex(owner_id),17,4)||'-'||substr(hex(owner_id),21,12)) ELSE owner_id END,
    settings, is_active, created_at, updated_at, deleted_at,
    CASE WHEN deleted_by IS NULL THEN NULL WHEN typeof(deleted_by) = 'blob' THEN lower(substr(hex(deleted_by),1,8)||'-'||substr(hex(deleted_by),9,4)||'-'||substr(hex(deleted_by),13,4)||'-'||substr(hex(deleted_by),17,4)||'-'||substr(hex(deleted_by),21,12)) ELSE deleted_by END,
    CASE WHEN company_id IS NULL THEN NULL WHEN typeof(company_id) = 'blob' THEN lower(substr(hex(company_id),1,8)||'-'||substr(hex(company_id),9,4)||'-'||substr(hex(company_id),13,4)||'-'||substr(hex(company_id),17,4)||'-'||substr(hex(company_id),21,12)) ELSE company_id END,
    invite_token, pending_owner_email,
    CASE WHEN created_by_org_id IS NULL THEN NULL WHEN typeof(created_by_org_id) = 'blob' THEN lower(substr(hex(created_by_org_id),1,8)||'-'||substr(hex(created_by_org_id),9,4)||'-'||substr(hex(created_by_org_id),13,4)||'-'||substr(hex(created_by_org_id),17,4)||'-'||substr(hex(created_by_org_id),21,12)) ELSE created_by_org_id END,
    address
FROM organizations;

DROP TABLE organizations;
ALTER TABLE organizations_new RENAME TO organizations;

CREATE INDEX idx_organizations_slug ON organizations(slug);
CREATE INDEX idx_organizations_owner_id ON organizations(owner_id);
CREATE INDEX idx_organizations_deleted_at ON organizations(deleted_at);
CREATE UNIQUE INDEX idx_organizations_invite_token ON organizations(invite_token) WHERE invite_token IS NOT NULL;

-----------------------------------------------------------------------
-- 2. clients
-----------------------------------------------------------------------
CREATE TABLE clients_new (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    description TEXT,
    logo_url TEXT,
    website TEXT,
    crm_contact_id TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at TEXT,
    deleted_by TEXT,
    crm_enabled INTEGER NOT NULL DEFAULT 0,
    UNIQUE(organization_id, slug)
);

INSERT INTO clients_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    name, slug, description, logo_url, website,
    CASE WHEN crm_contact_id IS NULL THEN NULL WHEN typeof(crm_contact_id) = 'blob' THEN lower(substr(hex(crm_contact_id),1,8)||'-'||substr(hex(crm_contact_id),9,4)||'-'||substr(hex(crm_contact_id),13,4)||'-'||substr(hex(crm_contact_id),17,4)||'-'||substr(hex(crm_contact_id),21,12)) ELSE crm_contact_id END,
    is_active, created_at, updated_at, deleted_at,
    CASE WHEN deleted_by IS NULL THEN NULL WHEN typeof(deleted_by) = 'blob' THEN lower(substr(hex(deleted_by),1,8)||'-'||substr(hex(deleted_by),9,4)||'-'||substr(hex(deleted_by),13,4)||'-'||substr(hex(deleted_by),17,4)||'-'||substr(hex(deleted_by),21,12)) ELSE deleted_by END,
    crm_enabled
FROM clients;

DROP TABLE clients;
ALTER TABLE clients_new RENAME TO clients;

CREATE INDEX idx_clients_organization_id ON clients(organization_id);
CREATE INDEX idx_clients_slug ON clients(slug);
CREATE INDEX idx_clients_crm_contact_id ON clients(crm_contact_id);
CREATE INDEX idx_clients_is_active ON clients(is_active);

-----------------------------------------------------------------------
-- 3. projects
-----------------------------------------------------------------------
CREATE TABLE projects_new (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    git_repo_path TEXT NOT NULL DEFAULT '' UNIQUE,
    setup_script  TEXT DEFAULT '',
    created_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    dev_script TEXT DEFAULT '',
    cleanup_script TEXT,
    copy_files TEXT,
    max_concurrent_agents INTEGER DEFAULT 3,
    max_concurrent_browser_agents INTEGER DEFAULT 1,
    vibe_budget_limit INTEGER,
    vibe_spent_amount INTEGER NOT NULL DEFAULT 0,
    owner_id TEXT,
    deleted_at TEXT,
    deleted_by TEXT,
    aptos_address TEXT,
    aptos_private_key_encrypted TEXT,
    aptos_funded INTEGER NOT NULL DEFAULT 0,
    total_vibe_deposited INTEGER NOT NULL DEFAULT 0,
    total_vibe_withdrawn INTEGER NOT NULL DEFAULT 0,
    organization_id TEXT,
    client_id TEXT,
    folder_id TEXT,
    parent_project_id TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    primary_device_id TEXT,
    is_shared BOOLEAN DEFAULT FALSE,
    collaboration_enabled BOOLEAN DEFAULT FALSE
);

INSERT INTO projects_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    name, git_repo_path, setup_script, created_at, updated_at,
    dev_script, cleanup_script, copy_files, max_concurrent_agents, max_concurrent_browser_agents,
    vibe_budget_limit, vibe_spent_amount,
    CASE WHEN owner_id IS NULL THEN NULL WHEN typeof(owner_id) = 'blob' THEN lower(substr(hex(owner_id),1,8)||'-'||substr(hex(owner_id),9,4)||'-'||substr(hex(owner_id),13,4)||'-'||substr(hex(owner_id),17,4)||'-'||substr(hex(owner_id),21,12)) ELSE owner_id END,
    deleted_at,
    CASE WHEN deleted_by IS NULL THEN NULL WHEN typeof(deleted_by) = 'blob' THEN lower(substr(hex(deleted_by),1,8)||'-'||substr(hex(deleted_by),9,4)||'-'||substr(hex(deleted_by),13,4)||'-'||substr(hex(deleted_by),17,4)||'-'||substr(hex(deleted_by),21,12)) ELSE deleted_by END,
    aptos_address, aptos_private_key_encrypted, aptos_funded,
    total_vibe_deposited, total_vibe_withdrawn,
    CASE WHEN organization_id IS NULL THEN NULL WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN client_id IS NULL THEN NULL WHEN typeof(client_id) = 'blob' THEN lower(substr(hex(client_id),1,8)||'-'||substr(hex(client_id),9,4)||'-'||substr(hex(client_id),13,4)||'-'||substr(hex(client_id),17,4)||'-'||substr(hex(client_id),21,12)) ELSE client_id END,
    CASE WHEN folder_id IS NULL THEN NULL WHEN typeof(folder_id) = 'blob' THEN lower(substr(hex(folder_id),1,8)||'-'||substr(hex(folder_id),9,4)||'-'||substr(hex(folder_id),13,4)||'-'||substr(hex(folder_id),17,4)||'-'||substr(hex(folder_id),21,12)) ELSE folder_id END,
    CASE WHEN parent_project_id IS NULL THEN NULL WHEN typeof(parent_project_id) = 'blob' THEN lower(substr(hex(parent_project_id),1,8)||'-'||substr(hex(parent_project_id),9,4)||'-'||substr(hex(parent_project_id),13,4)||'-'||substr(hex(parent_project_id),17,4)||'-'||substr(hex(parent_project_id),21,12)) ELSE parent_project_id END,
    sort_order, primary_device_id, is_shared, collaboration_enabled
FROM projects;

DROP TABLE projects;
ALTER TABLE projects_new RENAME TO projects;

CREATE INDEX idx_projects_owner_id ON projects(owner_id);
CREATE INDEX idx_projects_deleted_at ON projects(deleted_at);
CREATE INDEX idx_projects_aptos_address ON projects(aptos_address);
CREATE INDEX idx_projects_organization_id ON projects(organization_id);
CREATE INDEX idx_projects_client_id ON projects(client_id);
CREATE INDEX idx_projects_folder_id ON projects(folder_id);
CREATE INDEX idx_projects_parent ON projects(parent_project_id);

-----------------------------------------------------------------------
-- 4. project_boards
-----------------------------------------------------------------------
CREATE TABLE project_boards_new (
    id            TEXT PRIMARY KEY,
    project_id    TEXT NOT NULL,
    name          TEXT NOT NULL,
    slug          TEXT NOT NULL,
    board_type    TEXT NOT NULL CHECK (board_type IN (
        'executive_assets', 'brand_assets', 'dev_assets', 'social_assets', 'custom'
    )),
    description   TEXT,
    metadata      TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    CONSTRAINT project_boards_unique_slug UNIQUE (project_id, slug)
);

INSERT INTO project_boards_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    name, slug, board_type, description, metadata, created_at, updated_at
FROM project_boards;

DROP TABLE project_boards;
ALTER TABLE project_boards_new RENAME TO project_boards;

CREATE INDEX idx_project_boards_project ON project_boards(project_id);

-- Recreate board type validation triggers
CREATE TRIGGER validate_board_type_extended
BEFORE INSERT ON project_boards
FOR EACH ROW
WHEN NEW.board_type NOT IN (
    'executive_assets', 'brand_assets', 'dev_assets', 'social_assets', 'custom',
    'agent_flows', 'artifact_gallery', 'approval_queue', 'research_hub'
)
BEGIN
    SELECT RAISE(ABORT, 'Invalid board type');
END;

CREATE TRIGGER validate_board_type_update_extended
BEFORE UPDATE OF board_type ON project_boards
FOR EACH ROW
WHEN NEW.board_type NOT IN (
    'executive_assets', 'brand_assets', 'dev_assets', 'social_assets', 'custom',
    'agent_flows', 'artifact_gallery', 'approval_queue', 'research_hub'
)
BEGIN
    SELECT RAISE(ABORT, 'Invalid board type');
END;

-----------------------------------------------------------------------
-- 5. board_shares
-----------------------------------------------------------------------
CREATE TABLE board_shares_new (
    id                     TEXT PRIMARY KEY NOT NULL,
    board_id               TEXT NOT NULL,
    source_organization_id TEXT NOT NULL,
    target_organization_id TEXT NOT NULL,
    permission             TEXT NOT NULL DEFAULT 'editor'
                           CHECK(permission IN ('viewer', 'editor', 'admin')),
    share_type             TEXT NOT NULL DEFAULT 'collaboration'
                           CHECK(share_type IN ('joint_venture', 'collaboration', 'review')),
    shared_by              TEXT NOT NULL,
    is_active              INTEGER NOT NULL DEFAULT 1,
    created_at             TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at             TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(board_id, target_organization_id)
);

INSERT INTO board_shares_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(board_id) = 'blob' THEN lower(substr(hex(board_id),1,8)||'-'||substr(hex(board_id),9,4)||'-'||substr(hex(board_id),13,4)||'-'||substr(hex(board_id),17,4)||'-'||substr(hex(board_id),21,12)) ELSE board_id END,
    CASE WHEN typeof(source_organization_id) = 'blob' THEN lower(substr(hex(source_organization_id),1,8)||'-'||substr(hex(source_organization_id),9,4)||'-'||substr(hex(source_organization_id),13,4)||'-'||substr(hex(source_organization_id),17,4)||'-'||substr(hex(source_organization_id),21,12)) ELSE source_organization_id END,
    CASE WHEN typeof(target_organization_id) = 'blob' THEN lower(substr(hex(target_organization_id),1,8)||'-'||substr(hex(target_organization_id),9,4)||'-'||substr(hex(target_organization_id),13,4)||'-'||substr(hex(target_organization_id),17,4)||'-'||substr(hex(target_organization_id),21,12)) ELSE target_organization_id END,
    permission, share_type,
    CASE WHEN typeof(shared_by) = 'blob' THEN lower(substr(hex(shared_by),1,8)||'-'||substr(hex(shared_by),9,4)||'-'||substr(hex(shared_by),13,4)||'-'||substr(hex(shared_by),17,4)||'-'||substr(hex(shared_by),21,12)) ELSE shared_by END,
    is_active, created_at, updated_at
FROM board_shares;

DROP TABLE board_shares;
ALTER TABLE board_shares_new RENAME TO board_shares;

CREATE INDEX idx_board_shares_target_org ON board_shares(target_organization_id);
CREATE INDEX idx_board_shares_board ON board_shares(board_id);
CREATE INDEX idx_board_shares_active ON board_shares(is_active);

-----------------------------------------------------------------------
-- 6. data_sources
-----------------------------------------------------------------------
CREATE TABLE data_sources_new (
    id              TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT,
    project_id      TEXT,
    created_by      TEXT,
    title           TEXT NOT NULL,
    description     TEXT,
    data_type       TEXT NOT NULL DEFAULT 'conversation',
    file_type       TEXT,
    file_name       TEXT,
    file_path       TEXT,
    file_size_bytes INTEGER,
    file_hash       TEXT,
    metadata        TEXT NOT NULL DEFAULT '{}',
    status          TEXT NOT NULL DEFAULT 'pending',
    processing_error TEXT,
    created_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    archived_at     DATETIME,
    source_type     TEXT NOT NULL DEFAULT 'file',
    content         TEXT,
    folder          TEXT NOT NULL DEFAULT 'Unfiled'
);

INSERT INTO data_sources_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN organization_id IS NULL THEN NULL WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN project_id IS NULL THEN NULL WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    CASE WHEN created_by IS NULL THEN NULL WHEN typeof(created_by) = 'blob' THEN lower(substr(hex(created_by),1,8)||'-'||substr(hex(created_by),9,4)||'-'||substr(hex(created_by),13,4)||'-'||substr(hex(created_by),17,4)||'-'||substr(hex(created_by),21,12)) ELSE created_by END,
    title, description, data_type, file_type, file_name, file_path,
    file_size_bytes, file_hash, metadata, status, processing_error,
    created_at, updated_at, archived_at, source_type, content, folder
FROM data_sources;

DROP TABLE data_sources;
ALTER TABLE data_sources_new RENAME TO data_sources;

CREATE INDEX idx_data_sources_org ON data_sources(organization_id) WHERE organization_id IS NOT NULL;
CREATE INDEX idx_data_sources_project ON data_sources(project_id) WHERE project_id IS NOT NULL;
CREATE INDEX idx_data_sources_data_type ON data_sources(data_type);
CREATE INDEX idx_data_sources_status ON data_sources(status);
CREATE INDEX idx_data_sources_created_at ON data_sources(created_at DESC);
CREATE INDEX idx_data_sources_source_type ON data_sources(source_type);

-----------------------------------------------------------------------
-- 7. tasks
-----------------------------------------------------------------------
CREATE TABLE tasks_new (
    id          TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    title       TEXT NOT NULL,
    description TEXT,
    status      TEXT NOT NULL DEFAULT 'todo'
                   CHECK (status IN ('todo','inprogress','done','cancelled','inreview')),
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    parent_task_attempt TEXT,
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN ('critical', 'high', 'medium', 'low')),
    assignee_id TEXT,
    assigned_agent TEXT,
    assigned_mcps TEXT,
    created_by TEXT NOT NULL DEFAULT 'system',
    requires_approval INTEGER NOT NULL DEFAULT 0,
    approval_status TEXT CHECK(approval_status IS NULL OR approval_status IN ('pending', 'approved', 'rejected', 'changes_requested')),
    parent_task_id TEXT,
    tags TEXT,
    due_date TEXT,
    pod_id TEXT,
    custom_properties TEXT,
    scheduled_start TEXT,
    scheduled_end TEXT,
    board_id TEXT,
    collaborators TEXT,
    agent_id TEXT,
    autonomy_mode TEXT DEFAULT 'agent_assisted'
        CHECK (autonomy_mode IN ('agent_driven', 'agent_assisted', 'review_driven')),
    onboarding_segment_id TEXT,
    workflow_phase TEXT,
    approved_by TEXT,
    approved_at TEXT,
    approval_options TEXT,
    approval_selection TEXT,
    deleted_at TEXT,
    deleted_by TEXT,
    created_by_user_id TEXT,
    execution_config TEXT,
    screenshot TEXT DEFAULT NULL,
    crm_deal_id TEXT,
    source_data_source_id TEXT,
    source_workflow_run_id TEXT,
    assignee_type TEXT
        CHECK (assignee_type IN ('user', 'agent', 'team')),
    completion_criteria TEXT,
    output_format TEXT
);

INSERT INTO tasks_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    title, description, status, created_at, updated_at,
    CASE WHEN parent_task_attempt IS NULL THEN NULL WHEN typeof(parent_task_attempt) = 'blob' THEN lower(substr(hex(parent_task_attempt),1,8)||'-'||substr(hex(parent_task_attempt),9,4)||'-'||substr(hex(parent_task_attempt),13,4)||'-'||substr(hex(parent_task_attempt),17,4)||'-'||substr(hex(parent_task_attempt),21,12)) ELSE parent_task_attempt END,
    priority, assignee_id, assigned_agent, assigned_mcps, created_by,
    requires_approval, approval_status, parent_task_id, tags, due_date,
    CASE WHEN pod_id IS NULL THEN NULL WHEN typeof(pod_id) = 'blob' THEN lower(substr(hex(pod_id),1,8)||'-'||substr(hex(pod_id),9,4)||'-'||substr(hex(pod_id),13,4)||'-'||substr(hex(pod_id),17,4)||'-'||substr(hex(pod_id),21,12)) ELSE pod_id END,
    custom_properties, scheduled_start, scheduled_end,
    CASE WHEN board_id IS NULL THEN NULL WHEN typeof(board_id) = 'blob' THEN lower(substr(hex(board_id),1,8)||'-'||substr(hex(board_id),9,4)||'-'||substr(hex(board_id),13,4)||'-'||substr(hex(board_id),17,4)||'-'||substr(hex(board_id),21,12)) ELSE board_id END,
    collaborators, agent_id, autonomy_mode,
    CASE WHEN onboarding_segment_id IS NULL THEN NULL WHEN typeof(onboarding_segment_id) = 'blob' THEN lower(substr(hex(onboarding_segment_id),1,8)||'-'||substr(hex(onboarding_segment_id),9,4)||'-'||substr(hex(onboarding_segment_id),13,4)||'-'||substr(hex(onboarding_segment_id),17,4)||'-'||substr(hex(onboarding_segment_id),21,12)) ELSE onboarding_segment_id END,
    workflow_phase, approved_by, approved_at, approval_options, approval_selection,
    deleted_at,
    CASE WHEN deleted_by IS NULL THEN NULL WHEN typeof(deleted_by) = 'blob' THEN lower(substr(hex(deleted_by),1,8)||'-'||substr(hex(deleted_by),9,4)||'-'||substr(hex(deleted_by),13,4)||'-'||substr(hex(deleted_by),17,4)||'-'||substr(hex(deleted_by),21,12)) ELSE deleted_by END,
    CASE WHEN created_by_user_id IS NULL THEN NULL WHEN typeof(created_by_user_id) = 'blob' THEN lower(substr(hex(created_by_user_id),1,8)||'-'||substr(hex(created_by_user_id),9,4)||'-'||substr(hex(created_by_user_id),13,4)||'-'||substr(hex(created_by_user_id),17,4)||'-'||substr(hex(created_by_user_id),21,12)) ELSE created_by_user_id END,
    execution_config, screenshot,
    CASE WHEN crm_deal_id IS NULL THEN NULL WHEN typeof(crm_deal_id) = 'blob' THEN lower(substr(hex(crm_deal_id),1,8)||'-'||substr(hex(crm_deal_id),9,4)||'-'||substr(hex(crm_deal_id),13,4)||'-'||substr(hex(crm_deal_id),17,4)||'-'||substr(hex(crm_deal_id),21,12)) ELSE crm_deal_id END,
    source_data_source_id, source_workflow_run_id, assignee_type,
    completion_criteria, output_format
FROM tasks;

DROP TABLE tasks;
ALTER TABLE tasks_new RENAME TO tasks;

CREATE INDEX idx_tasks_parent_task_attempt ON tasks(parent_task_attempt);
CREATE INDEX idx_tasks_project_created_at ON tasks(project_id, created_at DESC);
CREATE INDEX idx_tasks_pod ON tasks(pod_id);
CREATE INDEX idx_tasks_autonomy_mode ON tasks(autonomy_mode);
CREATE INDEX idx_tasks_onboarding_segment ON tasks(onboarding_segment_id);
CREATE INDEX idx_tasks_workflow_phase ON tasks(workflow_phase);
CREATE INDEX idx_tasks_approval_status ON tasks(approval_status);
CREATE INDEX idx_tasks_deleted_at ON tasks(deleted_at);
CREATE INDEX idx_tasks_board_id ON tasks(board_id);
CREATE INDEX idx_tasks_pod_id ON tasks(pod_id);
CREATE INDEX idx_tasks_project_created ON tasks(project_id, created_at DESC);
CREATE INDEX idx_tasks_created_by_user_id ON tasks(created_by_user_id);
CREATE INDEX idx_tasks_crm_deal ON tasks(crm_deal_id);

-----------------------------------------------------------------------
-- 8. organization_members (org_id → TEXT, user_id stays BLOB)
-----------------------------------------------------------------------
CREATE TABLE organization_members_new (
    id TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL DEFAULT 'member',
    joined_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    UNIQUE(organization_id, user_id)
);

INSERT INTO organization_members_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    user_id,
    role, joined_at
FROM organization_members;

DROP TABLE organization_members;
ALTER TABLE organization_members_new RENAME TO organization_members;

CREATE INDEX idx_org_members_org_id ON organization_members(organization_id);
CREATE INDEX idx_org_members_user_id ON organization_members(user_id);
CREATE INDEX idx_org_members_org_role ON organization_members(organization_id, role);

-----------------------------------------------------------------------
-- 9. project_members (project_id → TEXT, user_id/granted_by stay BLOB)
-----------------------------------------------------------------------
CREATE TABLE project_members_new (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK(role IN ('owner', 'admin', 'editor', 'viewer')),
    permissions TEXT NOT NULL DEFAULT '{}',
    granted_by BLOB,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(project_id, user_id)
);

INSERT INTO project_members_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    user_id, role, permissions, granted_by, granted_at
FROM project_members;

DROP TABLE project_members;
ALTER TABLE project_members_new RENAME TO project_members;

CREATE INDEX idx_project_members_project_id ON project_members(project_id);
CREATE INDEX idx_project_members_user_id ON project_members(user_id);

-----------------------------------------------------------------------
-- 10. client_members (client_id → TEXT, user_id/granted_by stay BLOB)
-----------------------------------------------------------------------
CREATE TABLE client_members_new (
    id TEXT PRIMARY KEY NOT NULL,
    client_id TEXT NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK(role IN ('admin', 'editor', 'viewer')),
    granted_by BLOB,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(client_id, user_id)
);

INSERT INTO client_members_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(client_id) = 'blob' THEN lower(substr(hex(client_id),1,8)||'-'||substr(hex(client_id),9,4)||'-'||substr(hex(client_id),13,4)||'-'||substr(hex(client_id),17,4)||'-'||substr(hex(client_id),21,12)) ELSE client_id END,
    user_id, role, granted_by, granted_at
FROM client_members;

DROP TABLE client_members;
ALTER TABLE client_members_new RENAME TO client_members;

CREATE INDEX idx_client_members_client_id ON client_members(client_id);
CREATE INDEX idx_client_members_user_id ON client_members(user_id);

-----------------------------------------------------------------------
-- Recreate dependent views and triggers
-----------------------------------------------------------------------

CREATE VIEW v_project_knowledge_completeness AS
SELECT
    p.id AS project_id,
    COALESCE(src.total_sources, 0)   AS total_sources,
    COALESCE(src.fresh_sources, 0)   AS fresh_sources,
    COALESCE(src.avg_coverage, 0.0)  AS avg_coverage,
    COALESCE(src.type_count, 0)      AS type_count,
    ROUND(
        0.4 * (COALESCE(src.type_count, 0) / 6.0)
        + 0.6 * COALESCE(src.avg_coverage, 0.0),
        4
    ) AS knowledge_completeness
FROM projects p
LEFT JOIN (
    SELECT
        project_id,
        COUNT(*)                                              AS total_sources,
        SUM(CASE WHEN is_stale = 0 THEN 1 ELSE 0 END)       AS fresh_sources,
        AVG(coverage_score)                                   AS avg_coverage,
        COUNT(DISTINCT source_type)                           AS type_count
    FROM project_knowledge_sources
    WHERE is_active = 1 AND project_id IS NOT NULL
    GROUP BY project_id
) src ON src.project_id = p.id;

CREATE TRIGGER trg_knowledge_auto_register_artifact
AFTER INSERT ON execution_artifacts
WHEN NEW.artifact_type IN ('research_report','strategy_document','content_calendar','competitor_analysis','content_draft','visual_brief','verification_report','aggregated_research')
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    SELECT randomblob(16), t.project_id, 'artifact', hex(NEW.id), COALESCE(NEW.title, 'Artifact'), SUBSTR(COALESCE(NEW.content, ''), 1, 500),
        CASE NEW.artifact_type WHEN 'research_report' THEN 0.6 WHEN 'strategy_document' THEN 0.7 WHEN 'competitor_analysis' THEN 0.6 WHEN 'aggregated_research' THEN 0.7 WHEN 'content_draft' THEN 0.4 WHEN 'visual_brief' THEN 0.4 WHEN 'content_calendar' THEN 0.5 WHEN 'verification_report' THEN 0.5 ELSE 0.3 END, 1
    FROM execution_processes ep JOIN task_attempts ta ON ta.id = ep.task_attempt_id JOIN tasks t ON t.id = ta.task_id
    WHERE ep.id = NEW.execution_process_id AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_auto_register_context_injection
AFTER INSERT ON context_injections
WHEN NEW.injection_type IN ('note', 'correction', 'directive', 'answer')
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    SELECT randomblob(16), t.project_id, 'context_injection', hex(NEW.id),
        COALESCE(NEW.injector_name || ': ' || NEW.injection_type, 'Context: ' || NEW.injection_type),
        SUBSTR(NEW.content, 1, 500),
        CASE NEW.injection_type WHEN 'correction' THEN 0.8 WHEN 'directive' THEN 0.7 WHEN 'answer' THEN 0.6 WHEN 'note' THEN 0.5 ELSE 0.4 END, 1
    FROM execution_processes ep JOIN task_attempts ta ON ta.id = ep.task_attempt_id JOIN tasks t ON t.id = ta.task_id
    WHERE ep.id = NEW.execution_process_id AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER trg_knowledge_auto_register_entity_appearance
AFTER INSERT ON entity_appearances
BEGIN
    INSERT INTO project_knowledge_sources (id, project_id, source_type, source_id, source_title, source_summary, coverage_score, auto_registered)
    SELECT randomblob(16), pb.project_id, 'entity', hex(NEW.id),
        COALESCE(e.canonical_name || ' (' || NEW.appearance_type || ')', 'Entity Appearance'),
        COALESCE(NEW.talk_title, e.bio),
        CASE NEW.appearance_type WHEN 'keynote' THEN 0.7 WHEN 'speaker' THEN 0.6 WHEN 'panelist' THEN 0.5 WHEN 'sponsor' THEN 0.5 WHEN 'workshop_leader' THEN 0.6 ELSE 0.4 END,
        1
    FROM project_boards pb JOIN entities e ON e.id = NEW.entity_id
    WHERE pb.id = NEW.conference_board_id AND pb.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

PRAGMA foreign_keys = ON;
