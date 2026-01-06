-- Migration: Add Email Integration & CRM Foundation Tables
-- Implements Gmail and Zoho Mail connections, plus CRM contact management

-- ============================================================================
-- PART 1: Email Accounts (Gmail & Zoho Mail Connections)
-- ============================================================================

CREATE TABLE IF NOT EXISTS email_accounts (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Provider info
    provider TEXT NOT NULL CHECK (provider IN (
        'gmail', 'zoho', 'outlook', 'imap_custom'
    )),
    account_type TEXT NOT NULL DEFAULT 'primary' CHECK (account_type IN (
        'primary', 'team', 'notifications', 'marketing', 'support'
    )),

    -- Account identifiers
    email_address TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,

    -- OAuth tokens (encrypted in production)
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TEXT,

    -- IMAP/SMTP settings for custom providers
    imap_host TEXT,
    imap_port INTEGER,
    smtp_host TEXT,
    smtp_port INTEGER,
    use_ssl INTEGER DEFAULT 1,

    -- Scopes & permissions
    granted_scopes TEXT, -- JSON: array of OAuth scopes granted

    -- Account metadata
    storage_used_bytes INTEGER,
    storage_total_bytes INTEGER,
    unread_count INTEGER DEFAULT 0,
    metadata TEXT, -- JSON for provider-specific data

    -- Status
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN (
        'active', 'inactive', 'expired', 'error', 'pending_auth', 'revoked'
    )),
    last_sync_at TEXT,
    last_error TEXT,

    -- Settings
    sync_enabled INTEGER DEFAULT 1,
    sync_frequency_minutes INTEGER DEFAULT 15,
    auto_reply_enabled INTEGER DEFAULT 0,
    signature TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, provider, email_address)
);

CREATE INDEX IF NOT EXISTS idx_email_accounts_project ON email_accounts(project_id);
CREATE INDEX IF NOT EXISTS idx_email_accounts_provider ON email_accounts(provider);
CREATE INDEX IF NOT EXISTS idx_email_accounts_status ON email_accounts(status);
CREATE INDEX IF NOT EXISTS idx_email_accounts_email ON email_accounts(email_address);

-- ============================================================================
-- PART 2: Email Messages (Synced Emails)
-- ============================================================================

CREATE TABLE IF NOT EXISTS email_messages (
    id BLOB PRIMARY KEY,
    email_account_id BLOB NOT NULL REFERENCES email_accounts(id) ON DELETE CASCADE,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Message identifiers
    provider_message_id TEXT NOT NULL,
    thread_id TEXT,

    -- Envelope
    from_address TEXT NOT NULL,
    from_name TEXT,
    to_addresses TEXT NOT NULL, -- JSON: array of recipients
    cc_addresses TEXT, -- JSON
    bcc_addresses TEXT, -- JSON
    reply_to TEXT,

    -- Content
    subject TEXT,
    body_text TEXT,
    body_html TEXT,
    snippet TEXT, -- Preview text

    -- Attachments
    has_attachments INTEGER DEFAULT 0,
    attachments TEXT, -- JSON: [{name, size, content_type, url}]

    -- Labels & status
    labels TEXT, -- JSON: provider-specific labels/folders
    is_read INTEGER DEFAULT 0,
    is_starred INTEGER DEFAULT 0,
    is_draft INTEGER DEFAULT 0,
    is_sent INTEGER DEFAULT 0,
    is_archived INTEGER DEFAULT 0,
    is_spam INTEGER DEFAULT 0,
    is_trash INTEGER DEFAULT 0,

    -- Threading & conversation
    in_reply_to TEXT,
    "references" TEXT, -- JSON: message-id references

    -- CRM linkage
    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE SET NULL,
    crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL,

    -- Agent handling
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    sentiment TEXT CHECK (sentiment IN ('positive', 'neutral', 'negative', 'unknown')),
    priority TEXT DEFAULT 'normal' CHECK (priority IN ('low', 'normal', 'high', 'urgent')),

    -- Response tracking
    needs_response INTEGER DEFAULT 0,
    response_due_at TEXT,
    responded_at TEXT,

    received_at TEXT NOT NULL,
    sent_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(email_account_id, provider_message_id)
);

CREATE INDEX IF NOT EXISTS idx_email_messages_account ON email_messages(email_account_id);
CREATE INDEX IF NOT EXISTS idx_email_messages_project ON email_messages(project_id);
CREATE INDEX IF NOT EXISTS idx_email_messages_thread ON email_messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_email_messages_from ON email_messages(from_address);
CREATE INDEX IF NOT EXISTS idx_email_messages_unread ON email_messages(is_read) WHERE is_read = 0;
CREATE INDEX IF NOT EXISTS idx_email_messages_starred ON email_messages(is_starred) WHERE is_starred = 1;
CREATE INDEX IF NOT EXISTS idx_email_messages_needs_response ON email_messages(needs_response) WHERE needs_response = 1;
CREATE INDEX IF NOT EXISTS idx_email_messages_crm_contact ON email_messages(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_email_messages_received ON email_messages(received_at);

-- ============================================================================
-- PART 3: CRM Contacts
-- ============================================================================

CREATE TABLE IF NOT EXISTS crm_contacts (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Basic info
    first_name TEXT,
    last_name TEXT,
    full_name TEXT,
    email TEXT,
    phone TEXT,
    mobile TEXT,
    avatar_url TEXT,

    -- Organization
    company_name TEXT,
    job_title TEXT,
    department TEXT,

    -- Social profiles
    linkedin_url TEXT,
    twitter_handle TEXT,
    website TEXT,

    -- Contact source & lifecycle
    source TEXT CHECK (source IN (
        'manual', 'email', 'social', 'website', 'referral',
        'import', 'api', 'zoho_sync', 'gmail_sync'
    )),
    lifecycle_stage TEXT DEFAULT 'lead' CHECK (lifecycle_stage IN (
        'subscriber', 'lead', 'mql', 'sql', 'opportunity',
        'customer', 'evangelist', 'churned'
    )),

    -- Lead scoring
    lead_score INTEGER DEFAULT 0,
    last_activity_at TEXT,
    last_contacted_at TEXT,
    last_replied_at TEXT,

    -- Owner & assignment
    owner_user_id TEXT,
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,

    -- External IDs (for syncing)
    zoho_contact_id TEXT,
    gmail_contact_id TEXT,
    external_ids TEXT, -- JSON: {provider: id}

    -- Tags & segmentation
    tags TEXT, -- JSON array
    lists TEXT, -- JSON array of list IDs
    custom_fields TEXT, -- JSON object

    -- Address
    address_line1 TEXT,
    address_line2 TEXT,
    city TEXT,
    state TEXT,
    postal_code TEXT,
    country TEXT,

    -- Communication preferences
    email_opt_in INTEGER DEFAULT 1,
    sms_opt_in INTEGER DEFAULT 0,
    do_not_contact INTEGER DEFAULT 0,

    -- Stats
    email_count INTEGER DEFAULT 0,
    meeting_count INTEGER DEFAULT 0,
    deal_count INTEGER DEFAULT 0,
    total_revenue REAL DEFAULT 0.0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, email)
);

CREATE INDEX IF NOT EXISTS idx_crm_contacts_project ON crm_contacts(project_id);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_email ON crm_contacts(email);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_company ON crm_contacts(company_name);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_lifecycle ON crm_contacts(lifecycle_stage);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_lead_score ON crm_contacts(lead_score);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_zoho ON crm_contacts(zoho_contact_id);
CREATE INDEX IF NOT EXISTS idx_crm_contacts_last_activity ON crm_contacts(last_activity_at);

-- ============================================================================
-- PART 4: CRM Deals (Sales Pipeline)
-- ============================================================================

CREATE TABLE IF NOT EXISTS crm_deals (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE SET NULL,

    -- Deal info
    name TEXT NOT NULL,
    description TEXT,
    amount REAL,
    currency TEXT DEFAULT 'USD',

    -- Pipeline & stage
    pipeline TEXT DEFAULT 'default',
    stage TEXT NOT NULL DEFAULT 'qualification' CHECK (stage IN (
        'qualification', 'discovery', 'proposal', 'negotiation',
        'closed_won', 'closed_lost'
    )),
    probability INTEGER DEFAULT 0, -- 0-100%

    -- Dates
    expected_close_date TEXT,
    actual_close_date TEXT,
    last_activity_at TEXT,

    -- Owner & assignment
    owner_user_id TEXT,
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,

    -- External IDs
    zoho_deal_id TEXT,
    external_ids TEXT, -- JSON

    -- Metadata
    tags TEXT, -- JSON array
    custom_fields TEXT, -- JSON object
    lost_reason TEXT,
    win_reason TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_crm_deals_project ON crm_deals(project_id);
CREATE INDEX IF NOT EXISTS idx_crm_deals_contact ON crm_deals(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_crm_deals_stage ON crm_deals(stage);
CREATE INDEX IF NOT EXISTS idx_crm_deals_pipeline ON crm_deals(pipeline, stage);
CREATE INDEX IF NOT EXISTS idx_crm_deals_zoho ON crm_deals(zoho_deal_id);
CREATE INDEX IF NOT EXISTS idx_crm_deals_close_date ON crm_deals(expected_close_date);

-- ============================================================================
-- PART 5: CRM Activities (Unified Activity Log)
-- ============================================================================

CREATE TABLE IF NOT EXISTS crm_activities (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    crm_contact_id BLOB REFERENCES crm_contacts(id) ON DELETE CASCADE,
    crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL,

    -- Activity type
    activity_type TEXT NOT NULL CHECK (activity_type IN (
        'email_sent', 'email_received', 'email_opened', 'email_clicked',
        'call_made', 'call_received', 'call_scheduled',
        'meeting_scheduled', 'meeting_completed', 'meeting_cancelled',
        'note_added', 'task_created', 'task_completed',
        'deal_stage_changed', 'deal_created', 'deal_won', 'deal_lost',
        'social_mention', 'social_dm', 'social_comment',
        'form_submitted', 'page_visited', 'document_viewed',
        'custom'
    )),

    -- Activity details
    subject TEXT,
    description TEXT,
    outcome TEXT,

    -- Related records
    email_message_id BLOB REFERENCES email_messages(id) ON DELETE SET NULL,
    social_mention_id BLOB REFERENCES social_mentions(id) ON DELETE SET NULL,
    task_id BLOB REFERENCES tasks(id) ON DELETE SET NULL,

    -- Performer
    performed_by_user TEXT,
    performed_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,

    -- Metadata
    metadata TEXT, -- JSON for activity-specific data
    duration_minutes INTEGER,

    activity_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_crm_activities_project ON crm_activities(project_id);
CREATE INDEX IF NOT EXISTS idx_crm_activities_contact ON crm_activities(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_crm_activities_deal ON crm_activities(crm_deal_id);
CREATE INDEX IF NOT EXISTS idx_crm_activities_type ON crm_activities(activity_type);
CREATE INDEX IF NOT EXISTS idx_crm_activities_date ON crm_activities(activity_at);

-- ============================================================================
-- PART 6: Zoho Integration Settings
-- ============================================================================

CREATE TABLE IF NOT EXISTS zoho_integrations (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Zoho app connection
    zoho_org_id TEXT,
    zoho_domain TEXT, -- com, eu, in, com.au, jp

    -- OAuth tokens
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TEXT,
    granted_scopes TEXT, -- JSON array

    -- Sync settings
    sync_contacts INTEGER DEFAULT 1,
    sync_deals INTEGER DEFAULT 1,
    sync_activities INTEGER DEFAULT 1,
    sync_direction TEXT DEFAULT 'bidirectional' CHECK (sync_direction IN (
        'to_zoho', 'from_zoho', 'bidirectional'
    )),

    -- Field mappings
    contact_field_mapping TEXT, -- JSON: {local_field: zoho_field}
    deal_field_mapping TEXT, -- JSON

    -- Last sync timestamps
    last_contact_sync_at TEXT,
    last_deal_sync_at TEXT,
    last_activity_sync_at TEXT,

    -- Status
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN (
        'active', 'inactive', 'error', 'pending_auth', 'revoked'
    )),
    last_error TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id)
);

CREATE INDEX IF NOT EXISTS idx_zoho_integrations_project ON zoho_integrations(project_id);
CREATE INDEX IF NOT EXISTS idx_zoho_integrations_status ON zoho_integrations(status);

-- ============================================================================
-- PART 7: Email Templates
-- ============================================================================

CREATE TABLE IF NOT EXISTS email_templates (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Template info
    name TEXT NOT NULL,
    description TEXT,
    category TEXT, -- sales, support, marketing, onboarding, etc.

    -- Content
    subject TEXT NOT NULL,
    body_html TEXT NOT NULL,
    body_text TEXT,

    -- Variables & personalization
    variables TEXT, -- JSON: [{name, type, default_value}]

    -- Usage tracking
    times_used INTEGER DEFAULT 0,
    last_used_at TEXT,

    -- Performance metrics
    sent_count INTEGER DEFAULT 0,
    open_count INTEGER DEFAULT 0,
    click_count INTEGER DEFAULT 0,
    reply_count INTEGER DEFAULT 0,

    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_email_templates_project ON email_templates(project_id);
CREATE INDEX IF NOT EXISTS idx_email_templates_category ON email_templates(category);
