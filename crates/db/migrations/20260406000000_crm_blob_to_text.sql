-- Convert CRM and persons tables from BLOB UUID storage to TEXT UUID storage.
-- Also drops the legacy stage CHECK constraint on crm_deals (incompatible with 8-stage pipeline).
-- Also makes tasks.project_id nullable (required for CRM-only review tasks).
--
-- BLOB→TEXT conversion expression (reused throughout):
--   CASE WHEN col IS NULL THEN NULL
--        WHEN typeof(col) = 'blob' THEN lower(substr(hex(col),1,8)||'-'||substr(hex(col),9,4)||'-'||substr(hex(col),13,4)||'-'||substr(hex(col),17,4)||'-'||substr(hex(col),21,12))
--        ELSE col END

PRAGMA foreign_keys = OFF;

-- Drop triggers that reference tables being recreated (will recreate at end)
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_artifact;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_context_injection;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_entity_appearance;
DROP TRIGGER IF EXISTS trg_knowledge_update_entity_appearance;

-----------------------------------------------------------------------
-- 1. crm_pipelines
-----------------------------------------------------------------------
CREATE TABLE crm_pipelines_new (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    client_id TEXT REFERENCES clients(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    description TEXT,
    pipeline_type TEXT NOT NULL CHECK (pipeline_type IN ('conferences', 'clients', 'sales', 'delivery', 'custom')),
    is_active INTEGER DEFAULT 1,
    is_default INTEGER DEFAULT 0,
    icon TEXT,
    color TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(organization_id, name)
);

-- Deduplicate on (org_id_norm, name) keeping the row with the latest updated_at.
-- Some orgs have duplicate pipeline names from seed migrations running multiple times.
INSERT INTO crm_pipelines_new
SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN project_id IS NULL THEN NULL WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN client_id IS NULL THEN NULL WHEN typeof(client_id) = 'blob' THEN lower(substr(hex(client_id),1,8)||'-'||substr(hex(client_id),9,4)||'-'||substr(hex(client_id),13,4)||'-'||substr(hex(client_id),17,4)||'-'||substr(hex(client_id),21,12)) ELSE client_id END,
    name, description, pipeline_type, is_active, is_default, icon, color, created_at, updated_at
FROM crm_pipelines
WHERE rowid IN (
    -- Keep MIN(rowid) = the original BLOB pipelines (which have deals attached),
    -- discarding higher-rowid TEXT re-seeds for the same (org, name).
    SELECT MIN(rowid) FROM crm_pipelines
    GROUP BY
        CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
        name
);

DROP TABLE crm_pipelines;
ALTER TABLE crm_pipelines_new RENAME TO crm_pipelines;

CREATE INDEX idx_crm_pipelines_project ON crm_pipelines(project_id);
CREATE INDEX idx_crm_pipelines_organization ON crm_pipelines(organization_id);
CREATE INDEX idx_crm_pipelines_client ON crm_pipelines(client_id);
CREATE INDEX idx_crm_pipelines_type ON crm_pipelines(pipeline_type);

-----------------------------------------------------------------------
-- 2. crm_pipeline_stages
-----------------------------------------------------------------------
CREATE TABLE crm_pipeline_stages_new (
    id TEXT PRIMARY KEY NOT NULL,
    pipeline_id TEXT NOT NULL REFERENCES crm_pipelines(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    color TEXT NOT NULL DEFAULT '#6B7280',
    position INTEGER NOT NULL DEFAULT 0,
    is_closed INTEGER DEFAULT 0,
    is_won INTEGER DEFAULT 0,
    probability INTEGER DEFAULT 0 CHECK (probability >= 0 AND probability <= 100),
    auto_move_after_days INTEGER,
    notify_on_enter INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(pipeline_id, name),
    UNIQUE(pipeline_id, position)
);

INSERT INTO crm_pipeline_stages_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(pipeline_id) = 'blob' THEN lower(substr(hex(pipeline_id),1,8)||'-'||substr(hex(pipeline_id),9,4)||'-'||substr(hex(pipeline_id),13,4)||'-'||substr(hex(pipeline_id),17,4)||'-'||substr(hex(pipeline_id),21,12)) ELSE pipeline_id END,
    name, description, color, position, is_closed, is_won, probability,
    auto_move_after_days, notify_on_enter, created_at, updated_at
FROM crm_pipeline_stages;

DROP TABLE crm_pipeline_stages;
ALTER TABLE crm_pipeline_stages_new RENAME TO crm_pipeline_stages;

CREATE INDEX idx_crm_pipeline_stages_pipeline ON crm_pipeline_stages(pipeline_id);
CREATE INDEX idx_crm_pipeline_stages_position ON crm_pipeline_stages(pipeline_id, position);

-----------------------------------------------------------------------
-- 3. crm_contacts
-----------------------------------------------------------------------
CREATE TABLE crm_contacts_new (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    client_id TEXT REFERENCES clients(id) ON DELETE SET NULL,
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
    assigned_agent_id TEXT,
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

INSERT INTO crm_contacts_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN project_id IS NULL THEN NULL WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN client_id IS NULL THEN NULL WHEN typeof(client_id) = 'blob' THEN lower(substr(hex(client_id),1,8)||'-'||substr(hex(client_id),9,4)||'-'||substr(hex(client_id),13,4)||'-'||substr(hex(client_id),17,4)||'-'||substr(hex(client_id),21,12)) ELSE client_id END,
    first_name, last_name, full_name, email, phone, mobile, avatar_url,
    company_name, job_title, department, linkedin_url, twitter_handle, website,
    source, lifecycle_stage, lead_score, last_activity_at, last_contacted_at, last_replied_at,
    owner_user_id,
    CASE WHEN assigned_agent_id IS NULL THEN NULL WHEN typeof(assigned_agent_id) = 'blob' THEN lower(substr(hex(assigned_agent_id),1,8)||'-'||substr(hex(assigned_agent_id),9,4)||'-'||substr(hex(assigned_agent_id),13,4)||'-'||substr(hex(assigned_agent_id),17,4)||'-'||substr(hex(assigned_agent_id),21,12)) ELSE assigned_agent_id END,
    zoho_contact_id, gmail_contact_id, external_ids, tags, lists, custom_fields,
    address_line1, address_line2, city, state, postal_code, country,
    email_opt_in, sms_opt_in, do_not_contact, email_count, meeting_count, deal_count,
    total_revenue, source_data_source_id, source_workflow_run_id, created_at, updated_at
FROM crm_contacts;

DROP TABLE crm_contacts;
ALTER TABLE crm_contacts_new RENAME TO crm_contacts;

CREATE INDEX idx_crm_contacts_project ON crm_contacts(project_id);
CREATE INDEX idx_crm_contacts_organization ON crm_contacts(organization_id);
CREATE INDEX idx_crm_contacts_client ON crm_contacts(client_id);
CREATE INDEX idx_crm_contacts_email ON crm_contacts(email);
CREATE INDEX idx_crm_contacts_company ON crm_contacts(company_name);
CREATE INDEX idx_crm_contacts_lifecycle ON crm_contacts(lifecycle_stage);
CREATE INDEX idx_crm_contacts_lead_score ON crm_contacts(lead_score);
CREATE INDEX idx_crm_contacts_zoho ON crm_contacts(zoho_contact_id);
CREATE INDEX idx_crm_contacts_last_activity ON crm_contacts(last_activity_at);
CREATE INDEX idx_crm_contacts_org_email ON crm_contacts(organization_id, email);

-----------------------------------------------------------------------
-- 4. crm_deals  (also drops the legacy stage CHECK constraint)
-----------------------------------------------------------------------
CREATE TABLE crm_deals_new (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    client_id TEXT REFERENCES clients(id) ON DELETE SET NULL,
    crm_contact_id TEXT REFERENCES crm_contacts(id) ON DELETE SET NULL,
    crm_pipeline_id TEXT REFERENCES crm_pipelines(id) ON DELETE SET NULL,
    crm_stage_id TEXT REFERENCES crm_pipeline_stages(id) ON DELETE SET NULL,
    position INTEGER DEFAULT 0,
    name TEXT NOT NULL,
    description TEXT,
    amount REAL,
    currency TEXT DEFAULT 'USD',
    pipeline TEXT DEFAULT 'default',
    stage TEXT NOT NULL DEFAULT 'lead',   -- no CHECK constraint; free-form stage name
    probability INTEGER DEFAULT 0,
    expected_close_date TEXT,
    actual_close_date TEXT,
    last_activity_at TEXT,
    owner_user_id TEXT,
    assigned_agent_id TEXT,
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

INSERT INTO crm_deals_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN project_id IS NULL THEN NULL WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN client_id IS NULL THEN NULL WHEN typeof(client_id) = 'blob' THEN lower(substr(hex(client_id),1,8)||'-'||substr(hex(client_id),9,4)||'-'||substr(hex(client_id),13,4)||'-'||substr(hex(client_id),17,4)||'-'||substr(hex(client_id),21,12)) ELSE client_id END,
    CASE WHEN crm_contact_id IS NULL THEN NULL WHEN typeof(crm_contact_id) = 'blob' THEN lower(substr(hex(crm_contact_id),1,8)||'-'||substr(hex(crm_contact_id),9,4)||'-'||substr(hex(crm_contact_id),13,4)||'-'||substr(hex(crm_contact_id),17,4)||'-'||substr(hex(crm_contact_id),21,12)) ELSE crm_contact_id END,
    CASE WHEN crm_pipeline_id IS NULL THEN NULL WHEN typeof(crm_pipeline_id) = 'blob' THEN lower(substr(hex(crm_pipeline_id),1,8)||'-'||substr(hex(crm_pipeline_id),9,4)||'-'||substr(hex(crm_pipeline_id),13,4)||'-'||substr(hex(crm_pipeline_id),17,4)||'-'||substr(hex(crm_pipeline_id),21,12)) ELSE crm_pipeline_id END,
    CASE WHEN crm_stage_id IS NULL THEN NULL WHEN typeof(crm_stage_id) = 'blob' THEN lower(substr(hex(crm_stage_id),1,8)||'-'||substr(hex(crm_stage_id),9,4)||'-'||substr(hex(crm_stage_id),13,4)||'-'||substr(hex(crm_stage_id),17,4)||'-'||substr(hex(crm_stage_id),21,12)) ELSE crm_stage_id END,
    position, name, description, amount, currency, pipeline,
    COALESCE(stage, 'lead'),   -- ensure no NULL in stage column
    probability, expected_close_date, actual_close_date, last_activity_at, owner_user_id,
    CASE WHEN assigned_agent_id IS NULL THEN NULL WHEN typeof(assigned_agent_id) = 'blob' THEN lower(substr(hex(assigned_agent_id),1,8)||'-'||substr(hex(assigned_agent_id),9,4)||'-'||substr(hex(assigned_agent_id),13,4)||'-'||substr(hex(assigned_agent_id),17,4)||'-'||substr(hex(assigned_agent_id),21,12)) ELSE assigned_agent_id END,
    zoho_deal_id, external_ids, tags, custom_fields, lost_reason, win_reason,
    source_data_source_id, source_workflow_run_id, created_at, updated_at
FROM crm_deals;

DROP TABLE crm_deals;
ALTER TABLE crm_deals_new RENAME TO crm_deals;

CREATE INDEX idx_crm_deals_project ON crm_deals(project_id);
CREATE INDEX idx_crm_deals_organization ON crm_deals(organization_id);
CREATE INDEX idx_crm_deals_client ON crm_deals(client_id);
CREATE INDEX idx_crm_deals_contact ON crm_deals(crm_contact_id);
CREATE INDEX idx_crm_deals_pipeline_id ON crm_deals(crm_pipeline_id);
CREATE INDEX idx_crm_deals_stage_id ON crm_deals(crm_stage_id);

-----------------------------------------------------------------------
-- 5. crm_activities
-----------------------------------------------------------------------
CREATE TABLE crm_activities_new (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    client_id TEXT REFERENCES clients(id) ON DELETE SET NULL,
    crm_contact_id TEXT REFERENCES crm_contacts(id) ON DELETE CASCADE,
    crm_deal_id TEXT REFERENCES crm_deals(id) ON DELETE SET NULL,
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
    email_message_id TEXT,
    social_mention_id TEXT,
    task_id TEXT,
    performed_by_user TEXT,
    performed_by_agent_id TEXT,
    metadata TEXT,
    duration_minutes INTEGER,
    activity_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

INSERT INTO crm_activities_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN project_id IS NULL THEN NULL WHEN typeof(project_id) = 'blob' THEN lower(substr(hex(project_id),1,8)||'-'||substr(hex(project_id),9,4)||'-'||substr(hex(project_id),13,4)||'-'||substr(hex(project_id),17,4)||'-'||substr(hex(project_id),21,12)) ELSE project_id END,
    CASE WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    CASE WHEN client_id IS NULL THEN NULL WHEN typeof(client_id) = 'blob' THEN lower(substr(hex(client_id),1,8)||'-'||substr(hex(client_id),9,4)||'-'||substr(hex(client_id),13,4)||'-'||substr(hex(client_id),17,4)||'-'||substr(hex(client_id),21,12)) ELSE client_id END,
    CASE WHEN crm_contact_id IS NULL THEN NULL WHEN typeof(crm_contact_id) = 'blob' THEN lower(substr(hex(crm_contact_id),1,8)||'-'||substr(hex(crm_contact_id),9,4)||'-'||substr(hex(crm_contact_id),13,4)||'-'||substr(hex(crm_contact_id),17,4)||'-'||substr(hex(crm_contact_id),21,12)) ELSE crm_contact_id END,
    CASE WHEN crm_deal_id IS NULL THEN NULL WHEN typeof(crm_deal_id) = 'blob' THEN lower(substr(hex(crm_deal_id),1,8)||'-'||substr(hex(crm_deal_id),9,4)||'-'||substr(hex(crm_deal_id),13,4)||'-'||substr(hex(crm_deal_id),17,4)||'-'||substr(hex(crm_deal_id),21,12)) ELSE crm_deal_id END,
    activity_type, subject, description, outcome,
    CASE WHEN email_message_id IS NULL THEN NULL WHEN typeof(email_message_id) = 'blob' THEN lower(substr(hex(email_message_id),1,8)||'-'||substr(hex(email_message_id),9,4)||'-'||substr(hex(email_message_id),13,4)||'-'||substr(hex(email_message_id),17,4)||'-'||substr(hex(email_message_id),21,12)) ELSE email_message_id END,
    CASE WHEN social_mention_id IS NULL THEN NULL WHEN typeof(social_mention_id) = 'blob' THEN lower(substr(hex(social_mention_id),1,8)||'-'||substr(hex(social_mention_id),9,4)||'-'||substr(hex(social_mention_id),13,4)||'-'||substr(hex(social_mention_id),17,4)||'-'||substr(hex(social_mention_id),21,12)) ELSE social_mention_id END,
    CASE WHEN task_id IS NULL THEN NULL WHEN typeof(task_id) = 'blob' THEN lower(substr(hex(task_id),1,8)||'-'||substr(hex(task_id),9,4)||'-'||substr(hex(task_id),13,4)||'-'||substr(hex(task_id),17,4)||'-'||substr(hex(task_id),21,12)) ELSE task_id END,
    performed_by_user,
    CASE WHEN performed_by_agent_id IS NULL THEN NULL WHEN typeof(performed_by_agent_id) = 'blob' THEN lower(substr(hex(performed_by_agent_id),1,8)||'-'||substr(hex(performed_by_agent_id),9,4)||'-'||substr(hex(performed_by_agent_id),13,4)||'-'||substr(hex(performed_by_agent_id),17,4)||'-'||substr(hex(performed_by_agent_id),21,12)) ELSE performed_by_agent_id END,
    metadata, duration_minutes, activity_at, created_at
FROM crm_activities;

DROP TABLE crm_activities;
ALTER TABLE crm_activities_new RENAME TO crm_activities;

CREATE INDEX idx_crm_activities_project ON crm_activities(project_id);
CREATE INDEX idx_crm_activities_organization ON crm_activities(organization_id);
CREATE INDEX idx_crm_activities_client ON crm_activities(client_id);
CREATE INDEX idx_crm_activities_contact ON crm_activities(crm_contact_id);
CREATE INDEX idx_crm_activities_deal ON crm_activities(crm_deal_id);
CREATE INDEX idx_crm_activities_type ON crm_activities(activity_type);
CREATE INDEX idx_crm_activities_date ON crm_activities(activity_at);

-----------------------------------------------------------------------
-- 6. persons
-----------------------------------------------------------------------
CREATE TABLE persons_new (
    id TEXT PRIMARY KEY NOT NULL,
    full_name TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    avatar_url TEXT,
    person_type TEXT NOT NULL DEFAULT 'contact',
    financial_role TEXT NOT NULL DEFAULT 'neutral',
    client_profile TEXT,
    business_stage TEXT,
    lifecycle_stage TEXT NOT NULL DEFAULT 'lead',
    lead_score INTEGER NOT NULL DEFAULT 0,
    company_name TEXT,
    job_title TEXT,
    website TEXT,
    user_id TEXT,
    crm_contact_id TEXT,
    organization_id TEXT,
    intelligence_summary TEXT,
    intelligence_raw TEXT,
    intelligence_last_run_at TEXT,
    intelligence_confidence REAL NOT NULL DEFAULT 0.0,
    notes TEXT,
    tags TEXT NOT NULL DEFAULT '[]',
    custom_fields TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    intelligence_status TEXT NOT NULL DEFAULT 'idle' CHECK(intelligence_status IN ('idle','queued','running','done','failed')),
    intelligence_agent TEXT,
    emails TEXT NOT NULL DEFAULT '[]',
    phones TEXT NOT NULL DEFAULT '[]',
    assigned_to TEXT,
    company_org_id TEXT,
    company_id TEXT,
    onboarding_channel TEXT CHECK(onboarding_channel IN ('email','instagram','whatsapp','linkedin','twitter','sms','phone','in_person')),
    preferred_contact TEXT CHECK(preferred_contact IN ('email','instagram','whatsapp','linkedin','twitter','sms','phone','in_person')),
    research_pass_count INTEGER NOT NULL DEFAULT 0,
    research_depth TEXT NOT NULL DEFAULT 'shallow'
);

INSERT INTO persons_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    full_name, email, phone, avatar_url, person_type, financial_role,
    client_profile, business_stage, lifecycle_stage, lead_score, company_name, job_title, website,
    CASE WHEN user_id IS NULL THEN NULL WHEN typeof(user_id) = 'blob' THEN lower(substr(hex(user_id),1,8)||'-'||substr(hex(user_id),9,4)||'-'||substr(hex(user_id),13,4)||'-'||substr(hex(user_id),17,4)||'-'||substr(hex(user_id),21,12)) ELSE user_id END,
    CASE WHEN crm_contact_id IS NULL THEN NULL WHEN typeof(crm_contact_id) = 'blob' THEN lower(substr(hex(crm_contact_id),1,8)||'-'||substr(hex(crm_contact_id),9,4)||'-'||substr(hex(crm_contact_id),13,4)||'-'||substr(hex(crm_contact_id),17,4)||'-'||substr(hex(crm_contact_id),21,12)) ELSE crm_contact_id END,
    CASE WHEN organization_id IS NULL THEN NULL WHEN typeof(organization_id) = 'blob' THEN lower(substr(hex(organization_id),1,8)||'-'||substr(hex(organization_id),9,4)||'-'||substr(hex(organization_id),13,4)||'-'||substr(hex(organization_id),17,4)||'-'||substr(hex(organization_id),21,12)) ELSE organization_id END,
    intelligence_summary, intelligence_raw, intelligence_last_run_at, intelligence_confidence,
    notes, tags, custom_fields, created_at, updated_at,
    intelligence_status, intelligence_agent, emails, phones,
    CASE WHEN assigned_to IS NULL THEN NULL WHEN typeof(assigned_to) = 'blob' THEN lower(substr(hex(assigned_to),1,8)||'-'||substr(hex(assigned_to),9,4)||'-'||substr(hex(assigned_to),13,4)||'-'||substr(hex(assigned_to),17,4)||'-'||substr(hex(assigned_to),21,12)) ELSE assigned_to END,
    CASE WHEN company_org_id IS NULL THEN NULL WHEN typeof(company_org_id) = 'blob' THEN lower(substr(hex(company_org_id),1,8)||'-'||substr(hex(company_org_id),9,4)||'-'||substr(hex(company_org_id),13,4)||'-'||substr(hex(company_org_id),17,4)||'-'||substr(hex(company_org_id),21,12)) ELSE company_org_id END,
    CASE WHEN company_id IS NULL THEN NULL WHEN typeof(company_id) = 'blob' THEN lower(substr(hex(company_id),1,8)||'-'||substr(hex(company_id),9,4)||'-'||substr(hex(company_id),13,4)||'-'||substr(hex(company_id),17,4)||'-'||substr(hex(company_id),21,12)) ELSE company_id END,
    onboarding_channel, preferred_contact, research_pass_count, research_depth
FROM persons;

DROP TABLE persons;
ALTER TABLE persons_new RENAME TO persons;

CREATE INDEX idx_persons_email          ON persons(email);
CREATE INDEX idx_persons_person_type    ON persons(person_type);
CREATE INDEX idx_persons_financial_role ON persons(financial_role);
CREATE INDEX idx_persons_user_id        ON persons(user_id);
CREATE INDEX idx_persons_crm_contact_id ON persons(crm_contact_id);
CREATE INDEX idx_persons_org            ON persons(organization_id);
CREATE INDEX idx_persons_company        ON persons(company_id);
CREATE INDEX idx_persons_lifecycle      ON persons(lifecycle_stage);
CREATE INDEX idx_persons_name           ON persons(full_name);

-----------------------------------------------------------------------
-- 7. person_social_profiles
-----------------------------------------------------------------------
CREATE TABLE person_social_profiles_new (
    id TEXT PRIMARY KEY NOT NULL,
    person_id TEXT NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    handle TEXT,
    profile_url TEXT,
    follower_count INTEGER,
    following_count INTEGER,
    bio TEXT,
    verified INTEGER NOT NULL DEFAULT 0,
    raw_data TEXT,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (person_id, platform)
);

INSERT INTO person_social_profiles_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(person_id) = 'blob' THEN lower(substr(hex(person_id),1,8)||'-'||substr(hex(person_id),9,4)||'-'||substr(hex(person_id),13,4)||'-'||substr(hex(person_id),17,4)||'-'||substr(hex(person_id),21,12)) ELSE person_id END,
    platform, handle, profile_url, follower_count, following_count, bio, verified,
    raw_data, last_synced_at, created_at, updated_at
FROM person_social_profiles;

DROP TABLE person_social_profiles;
ALTER TABLE person_social_profiles_new RENAME TO person_social_profiles;

CREATE INDEX idx_psp_person_id ON person_social_profiles(person_id);
CREATE INDEX idx_psp_platform  ON person_social_profiles(platform);

-----------------------------------------------------------------------
-- 8. person_notes
-----------------------------------------------------------------------
CREATE TABLE person_notes_new (
    id TEXT PRIMARY KEY NOT NULL,
    person_id TEXT NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    author_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    text TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'follow_up', 'resolved', 'pinned')),
    attachments TEXT NOT NULL DEFAULT '[]',
    proposal_id TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

INSERT INTO person_notes_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(person_id) = 'blob' THEN lower(substr(hex(person_id),1,8)||'-'||substr(hex(person_id),9,4)||'-'||substr(hex(person_id),13,4)||'-'||substr(hex(person_id),17,4)||'-'||substr(hex(person_id),21,12)) ELSE person_id END,
    CASE WHEN author_id IS NULL THEN NULL WHEN typeof(author_id) = 'blob' THEN lower(substr(hex(author_id),1,8)||'-'||substr(hex(author_id),9,4)||'-'||substr(hex(author_id),13,4)||'-'||substr(hex(author_id),17,4)||'-'||substr(hex(author_id),21,12)) ELSE author_id END,
    text, status, attachments,
    CASE WHEN proposal_id IS NULL THEN NULL WHEN typeof(proposal_id) = 'blob' THEN lower(substr(hex(proposal_id),1,8)||'-'||substr(hex(proposal_id),9,4)||'-'||substr(hex(proposal_id),13,4)||'-'||substr(hex(proposal_id),17,4)||'-'||substr(hex(proposal_id),21,12)) ELSE proposal_id END,
    created_at, updated_at
FROM person_notes;

DROP TABLE person_notes;
ALTER TABLE person_notes_new RENAME TO person_notes;

CREATE INDEX idx_person_notes_person   ON person_notes(person_id);
CREATE INDEX idx_person_notes_author   ON person_notes(author_id);
CREATE INDEX idx_person_notes_status   ON person_notes(status);

-----------------------------------------------------------------------
-- 9. person_research_passes
-----------------------------------------------------------------------
CREATE TABLE person_research_passes_new (
    id TEXT NOT NULL PRIMARY KEY,
    person_id TEXT NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    pass_number INTEGER NOT NULL DEFAULT 1,
    research_focus TEXT NOT NULL,
    focus_prompt TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    summary TEXT,
    raw_results TEXT,
    key_findings TEXT DEFAULT '[]',
    search_queries TEXT DEFAULT '[]',
    confidence_delta REAL DEFAULT 0.0,
    agent_used TEXT,
    tokens_used INTEGER,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    completed_at TEXT
);

INSERT INTO person_research_passes_new SELECT
    CASE WHEN typeof(id) = 'blob' THEN lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12)) ELSE id END,
    CASE WHEN typeof(person_id) = 'blob' THEN lower(substr(hex(person_id),1,8)||'-'||substr(hex(person_id),9,4)||'-'||substr(hex(person_id),13,4)||'-'||substr(hex(person_id),17,4)||'-'||substr(hex(person_id),21,12)) ELSE person_id END,
    pass_number, research_focus, focus_prompt, status, summary, raw_results,
    key_findings, search_queries, confidence_delta, agent_used, tokens_used, error,
    created_at, completed_at
FROM person_research_passes;

DROP TABLE person_research_passes;
ALTER TABLE person_research_passes_new RENAME TO person_research_passes;

CREATE INDEX idx_research_passes_person ON person_research_passes(person_id, pass_number);

-----------------------------------------------------------------------
-- 10. call_intake_items — convert BLOB ids in-place via UPDATE
-----------------------------------------------------------------------
UPDATE call_intake_items SET
    id = lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12))
WHERE typeof(id) = 'blob';

UPDATE call_intake_items SET
    person_id = lower(substr(hex(person_id),1,8)||'-'||substr(hex(person_id),9,4)||'-'||substr(hex(person_id),13,4)||'-'||substr(hex(person_id),17,4)||'-'||substr(hex(person_id),21,12))
WHERE person_id IS NOT NULL AND typeof(person_id) = 'blob';

UPDATE call_intake_items SET
    company_id = lower(substr(hex(company_id),1,8)||'-'||substr(hex(company_id),9,4)||'-'||substr(hex(company_id),13,4)||'-'||substr(hex(company_id),17,4)||'-'||substr(hex(company_id),21,12))
WHERE company_id IS NOT NULL AND typeof(company_id) = 'blob';

UPDATE call_intake_items SET
    crm_deal_id = lower(substr(hex(crm_deal_id),1,8)||'-'||substr(hex(crm_deal_id),9,4)||'-'||substr(hex(crm_deal_id),13,4)||'-'||substr(hex(crm_deal_id),17,4)||'-'||substr(hex(crm_deal_id),21,12))
WHERE crm_deal_id IS NOT NULL AND typeof(crm_deal_id) = 'blob';

UPDATE call_intake_items SET
    report_id = lower(substr(hex(report_id),1,8)||'-'||substr(hex(report_id),9,4)||'-'||substr(hex(report_id),13,4)||'-'||substr(hex(report_id),17,4)||'-'||substr(hex(report_id),21,12))
WHERE report_id IS NOT NULL AND typeof(report_id) = 'blob';

-----------------------------------------------------------------------
-- 11. business_reports — convert BLOB ids in-place via UPDATE
-----------------------------------------------------------------------
UPDATE business_reports SET
    id = lower(substr(hex(id),1,8)||'-'||substr(hex(id),9,4)||'-'||substr(hex(id),13,4)||'-'||substr(hex(id),17,4)||'-'||substr(hex(id),21,12))
WHERE typeof(id) = 'blob';

UPDATE business_reports SET
    person_id = lower(substr(hex(person_id),1,8)||'-'||substr(hex(person_id),9,4)||'-'||substr(hex(person_id),13,4)||'-'||substr(hex(person_id),17,4)||'-'||substr(hex(person_id),21,12))
WHERE person_id IS NOT NULL AND typeof(person_id) = 'blob';

UPDATE business_reports SET
    company_id = lower(substr(hex(company_id),1,8)||'-'||substr(hex(company_id),9,4)||'-'||substr(hex(company_id),13,4)||'-'||substr(hex(company_id),17,4)||'-'||substr(hex(company_id),21,12))
WHERE company_id IS NOT NULL AND typeof(company_id) = 'blob';

UPDATE business_reports SET
    crm_deal_id = lower(substr(hex(crm_deal_id),1,8)||'-'||substr(hex(crm_deal_id),9,4)||'-'||substr(hex(crm_deal_id),13,4)||'-'||substr(hex(crm_deal_id),17,4)||'-'||substr(hex(crm_deal_id),21,12))
WHERE crm_deal_id IS NOT NULL AND typeof(crm_deal_id) = 'blob';

UPDATE business_reports SET
    created_by = lower(substr(hex(created_by),1,8)||'-'||substr(hex(created_by),9,4)||'-'||substr(hex(created_by),13,4)||'-'||substr(hex(created_by),17,4)||'-'||substr(hex(created_by),21,12))
WHERE created_by IS NOT NULL AND typeof(created_by) = 'blob';

UPDATE business_reports SET
    reviewed_by = lower(substr(hex(reviewed_by),1,8)||'-'||substr(hex(reviewed_by),9,4)||'-'||substr(hex(reviewed_by),13,4)||'-'||substr(hex(reviewed_by),17,4)||'-'||substr(hex(reviewed_by),21,12))
WHERE reviewed_by IS NOT NULL AND typeof(reviewed_by) = 'blob';

-----------------------------------------------------------------------
-- 12. tasks — make project_id nullable (required for CRM-only review tasks)
-----------------------------------------------------------------------
CREATE TABLE tasks_new (
    id          TEXT PRIMARY KEY,
    project_id  TEXT,             -- was NOT NULL, now nullable for CRM-only tasks
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
    assignee_type TEXT CHECK (assignee_type IN ('user', 'agent', 'team')),
    completion_criteria TEXT,
    output_format TEXT
);

INSERT INTO tasks_new SELECT
    id, project_id, title, description, status, created_at, updated_at,
    parent_task_attempt, priority, assignee_id, assigned_agent, assigned_mcps,
    created_by, requires_approval, approval_status, parent_task_id, tags,
    due_date, pod_id, custom_properties, scheduled_start, scheduled_end,
    board_id, collaborators, agent_id, autonomy_mode, onboarding_segment_id,
    workflow_phase, approved_by, approved_at, approval_options, approval_selection,
    deleted_at, deleted_by, created_by_user_id, execution_config, screenshot,
    crm_deal_id, source_data_source_id, source_workflow_run_id,
    assignee_type, completion_criteria, output_format
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

PRAGMA foreign_keys = ON;

-- Recreate dropped triggers
CREATE TRIGGER trg_knowledge_update_entity_appearance
AFTER UPDATE OF status ON entity_appearances
WHEN NEW.status IN ('researched', 'verified', 'content_created')
BEGIN
    UPDATE project_knowledge_sources SET
        coverage_score = CASE NEW.status WHEN 'researched' THEN MAX(coverage_score, 0.7) WHEN 'verified' THEN MAX(coverage_score, 0.85) WHEN 'content_created' THEN MAX(coverage_score, 1.0) ELSE coverage_score END,
        is_stale = 0,
        last_refreshed_at = datetime('now', 'subsec'),
        updated_at = datetime('now', 'subsec')
    WHERE source_type = 'entity' AND source_id = hex(NEW.id);
END;

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
