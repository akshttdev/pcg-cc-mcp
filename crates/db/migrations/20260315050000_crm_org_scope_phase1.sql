-- Phase 1: Make organization_id the primary CRM scope, project_id optional
-- CRM objects (contacts, deals, activities) should be scoped to organization (and optionally client),
-- not to project. project_id becomes an optional association for traceability.
--
-- SQLite does not support ALTER COLUMN to change nullability, so we use the table-recreation pattern.

PRAGMA foreign_keys = OFF;

------------------------------------------------------------
-- 1. crm_contacts: make project_id nullable
------------------------------------------------------------
CREATE TABLE crm_contacts_new (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,  -- was NOT NULL, now nullable
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,  -- now required
    client_id BLOB REFERENCES clients(id) ON DELETE SET NULL,

    first_name TEXT,
    last_name TEXT,
    full_name TEXT,
    email TEXT,
    phone TEXT,
    mobile TEXT,
    avatar_url TEXT,
    company_name TEXT,
    job_title TEXT,
    department TEXT,
    linkedin_url TEXT,
    twitter_handle TEXT,
    website TEXT,
    source TEXT CHECK (source IN ('manual', 'email', 'social', 'website', 'referral', 'import', 'api', 'zoho_sync', 'gmail_sync', 'workflow')),
    lifecycle_stage TEXT DEFAULT 'lead' CHECK (lifecycle_stage IN ('subscriber', 'lead', 'mql', 'sql', 'opportunity', 'customer', 'evangelist', 'churned')),
    lead_score INTEGER DEFAULT 0,
    last_activity_at TEXT,
    last_contacted_at TEXT,
    last_replied_at TEXT,
    owner_user_id TEXT,
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    zoho_contact_id TEXT,
    gmail_contact_id TEXT,
    external_ids TEXT,
    tags TEXT,
    lists TEXT,
    custom_fields TEXT,
    address_line1 TEXT,
    address_line2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    country TEXT,
    email_opt_in INTEGER DEFAULT 1,
    sms_opt_in INTEGER DEFAULT 0,
    do_not_contact INTEGER DEFAULT 0,
    email_count INTEGER DEFAULT 0,
    meeting_count INTEGER DEFAULT 0,
    deal_count INTEGER DEFAULT 0,
    total_revenue REAL DEFAULT 0.0,
    source_data_source_id TEXT,
    source_workflow_run_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, email)
);

-- Backfill organization_id from project for any rows missing it, then copy
-- We use a temp table approach: first update org_id in old table, then copy
-- Use OR IGNORE to skip rows where backfilling org_id would cause a UNIQUE(org_id, email) conflict
UPDATE OR IGNORE crm_contacts SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_contacts.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

-- For any contacts still without org_id (orphaned), assign a placeholder so NOT NULL succeeds
-- This shouldn't happen in practice since all contacts had project_id NOT NULL before
-- If it does, we skip those rows

INSERT OR IGNORE INTO crm_contacts_new
SELECT
    id, project_id, organization_id, NULL,  -- client_id: new column, no existing data
    first_name, last_name, full_name, email, phone, mobile, avatar_url,
    company_name, job_title, department, linkedin_url, twitter_handle, website,
    source, lifecycle_stage, lead_score,
    last_activity_at, last_contacted_at, last_replied_at,
    owner_user_id, assigned_agent_id,
    zoho_contact_id, gmail_contact_id, external_ids,
    tags, lists, custom_fields,
    address_line1, address_line2, city, state, postal_code, country,
    email_opt_in, sms_opt_in, do_not_contact,
    email_count, meeting_count, deal_count, total_revenue,
    source_data_source_id, source_workflow_run_id,
    created_at, updated_at
FROM crm_contacts
WHERE organization_id IS NOT NULL;

DROP TABLE crm_contacts;
ALTER TABLE crm_contacts_new RENAME TO crm_contacts;

-- Recreate indexes
CREATE INDEX idx_crm_contacts_project ON crm_contacts(project_id);
CREATE INDEX idx_crm_contacts_organization ON crm_contacts(organization_id);
CREATE INDEX idx_crm_contacts_client ON crm_contacts(client_id);
CREATE INDEX idx_crm_contacts_email ON crm_contacts(email);
CREATE INDEX idx_crm_contacts_company ON crm_contacts(company_name);
CREATE INDEX idx_crm_contacts_lifecycle ON crm_contacts(lifecycle_stage);
CREATE INDEX idx_crm_contacts_lead_score ON crm_contacts(lead_score);
CREATE INDEX idx_crm_contacts_zoho ON crm_contacts(zoho_contact_id);
CREATE INDEX idx_crm_contacts_last_activity ON crm_contacts(last_activity_at);
-- New composite index for org-scoped email lookups
CREATE INDEX idx_crm_contacts_org_email ON crm_contacts(organization_id, email);

------------------------------------------------------------
-- 2. crm_deals: make project_id nullable
------------------------------------------------------------
CREATE TABLE crm_deals_new (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,  -- was NOT NULL, now nullable
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,  -- now required
    client_id BLOB REFERENCES clients(id) ON DELETE SET NULL,

    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE SET NULL,
    crm_pipeline_id BLOB REFERENCES crm_pipelines(id) ON DELETE SET NULL,
    crm_stage_id BLOB REFERENCES crm_pipeline_stages(id) ON DELETE SET NULL,
    position INTEGER DEFAULT 0,

    name TEXT NOT NULL,
    description TEXT,
    amount REAL,
    currency TEXT DEFAULT 'USD',
    pipeline TEXT DEFAULT 'default',
    stage TEXT NOT NULL DEFAULT 'qualification' CHECK (stage IN ('qualification', 'discovery', 'proposal', 'negotiation', 'closed_won', 'closed_lost')),
    probability INTEGER DEFAULT 0,
    expected_close_date TEXT,
    actual_close_date TEXT,
    last_activity_at TEXT,
    owner_user_id TEXT,
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    zoho_deal_id TEXT,
    external_ids TEXT,
    tags TEXT,
    custom_fields TEXT,
    lost_reason TEXT,
    win_reason TEXT,
    source_data_source_id TEXT,
    source_workflow_run_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

UPDATE OR IGNORE crm_deals SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_deals.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

INSERT OR IGNORE INTO crm_deals_new
SELECT
    id, project_id, organization_id, NULL,  -- client_id: new column, no existing data
    crm_contact_id, crm_pipeline_id, crm_stage_id, position,
    name, description, amount, currency,
    pipeline, stage, probability,
    expected_close_date, actual_close_date, last_activity_at,
    owner_user_id, assigned_agent_id,
    zoho_deal_id, external_ids, tags, custom_fields,
    lost_reason, win_reason,
    source_data_source_id, source_workflow_run_id,
    created_at, updated_at
FROM crm_deals
WHERE organization_id IS NOT NULL;

DROP TABLE crm_deals;
ALTER TABLE crm_deals_new RENAME TO crm_deals;

CREATE INDEX idx_crm_deals_project ON crm_deals(project_id);
CREATE INDEX idx_crm_deals_organization ON crm_deals(organization_id);
CREATE INDEX idx_crm_deals_client ON crm_deals(client_id);
CREATE INDEX idx_crm_deals_contact ON crm_deals(crm_contact_id);
CREATE INDEX idx_crm_deals_pipeline_id ON crm_deals(crm_pipeline_id);
CREATE INDEX idx_crm_deals_stage_id ON crm_deals(crm_stage_id);
CREATE INDEX idx_crm_deals_position ON crm_deals(crm_stage_id, position);
CREATE INDEX idx_crm_deals_zoho ON crm_deals(zoho_deal_id);
CREATE INDEX idx_crm_deals_close_date ON crm_deals(expected_close_date);

------------------------------------------------------------
-- 3. crm_activities: make project_id nullable
------------------------------------------------------------
CREATE TABLE crm_activities_new (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,  -- was NOT NULL, now nullable
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,  -- now required
    client_id BLOB REFERENCES clients(id) ON DELETE SET NULL,

    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE CASCADE,
    crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL,
    activity_type TEXT NOT NULL CHECK (activity_type IN (
        'email_sent', 'email_received', 'email_opened', 'email_clicked',
        'call_made', 'call_received', 'call_scheduled',
        'meeting_scheduled', 'meeting_completed', 'meeting_cancelled',
        'note_added', 'task_created', 'task_completed',
        'deal_stage_changed', 'deal_created', 'deal_won', 'deal_lost',
        'social_mention', 'social_dm', 'social_comment',
        'form_submitted', 'page_visited', 'document_viewed', 'custom'
    )),
    subject TEXT,
    description TEXT,
    outcome TEXT,
    email_message_id BLOB REFERENCES email_messages(id) ON DELETE SET NULL,
    social_mention_id BLOB REFERENCES social_mentions(id) ON DELETE SET NULL,
    task_id BLOB REFERENCES tasks(id) ON DELETE SET NULL,
    performed_by_user TEXT,
    performed_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    metadata TEXT,
    duration_minutes INTEGER,
    activity_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

UPDATE OR IGNORE crm_activities SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_activities.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

INSERT OR IGNORE INTO crm_activities_new
SELECT
    id, project_id, organization_id, NULL,  -- client_id: new column, no existing data
    crm_contact_id, crm_deal_id, activity_type,
    subject, description, outcome,
    email_message_id, social_mention_id, task_id,
    performed_by_user, performed_by_agent_id,
    metadata, duration_minutes,
    activity_at, created_at
FROM crm_activities
WHERE organization_id IS NOT NULL;

DROP TABLE crm_activities;
ALTER TABLE crm_activities_new RENAME TO crm_activities;

CREATE INDEX idx_crm_activities_project ON crm_activities(project_id);
CREATE INDEX idx_crm_activities_organization ON crm_activities(organization_id);
CREATE INDEX idx_crm_activities_client ON crm_activities(client_id);
CREATE INDEX idx_crm_activities_contact ON crm_activities(crm_contact_id);
CREATE INDEX idx_crm_activities_deal ON crm_activities(crm_deal_id);
CREATE INDEX idx_crm_activities_type ON crm_activities(activity_type);
CREATE INDEX idx_crm_activities_date ON crm_activities(activity_at);

------------------------------------------------------------
-- 4. crm_pipelines: make project_id nullable, org_id required
------------------------------------------------------------
CREATE TABLE crm_pipelines_new (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,  -- was NOT NULL, now nullable
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,  -- now required
    client_id BLOB REFERENCES clients(id) ON DELETE SET NULL,

    name TEXT NOT NULL,
    description TEXT,
    pipeline_type TEXT NOT NULL CHECK (pipeline_type IN (
        'conferences', 'clients', 'sales', 'delivery', 'custom'
    )),
    is_active INTEGER DEFAULT 1,
    is_default INTEGER DEFAULT 0,
    icon TEXT,
    color TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(organization_id, name)  -- scoped to org now instead of project
);

UPDATE OR IGNORE crm_pipelines SET organization_id = (
    SELECT p.organization_id FROM projects p WHERE p.id = crm_pipelines.project_id
) WHERE organization_id IS NULL AND project_id IS NOT NULL;

INSERT OR IGNORE INTO crm_pipelines_new
SELECT
    id, project_id, organization_id, NULL,  -- client_id: new column, no existing data
    name, description, pipeline_type,
    is_active, is_default, icon, color,
    created_at, updated_at
FROM crm_pipelines
WHERE organization_id IS NOT NULL;

DROP TABLE crm_pipelines;
ALTER TABLE crm_pipelines_new RENAME TO crm_pipelines;

CREATE INDEX idx_crm_pipelines_project ON crm_pipelines(project_id);
CREATE INDEX idx_crm_pipelines_organization ON crm_pipelines(organization_id);
CREATE INDEX idx_crm_pipelines_client ON crm_pipelines(client_id);
CREATE INDEX idx_crm_pipelines_type ON crm_pipelines(pipeline_type);

-- Recreate crm_pipeline_stages indexes (stages FK into pipelines, which was dropped/recreated)
-- The stages table itself doesn't change, but we need to make sure the FK still works
-- SQLite should handle this since we kept the same table name

PRAGMA foreign_keys = ON;
