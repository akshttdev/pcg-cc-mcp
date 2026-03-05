-- Universal Person Entity
--
-- Introduces a single canonical `persons` table that bridges the currently
-- fragmented identity records across `users` (platform accounts) and
-- `crm_contacts` (external contacts).  New CRM capabilities (social profiles,
-- invoices, AI intelligence) all point at `persons`.
--
-- Bridge pattern: existing tables get a `person_id` FK column.  A seed
-- section auto-creates person records from current data so no data is lost.

-- ============================================================
-- 1. persons — the universal entity
-- ============================================================
CREATE TABLE IF NOT EXISTS persons (
    id BLOB PRIMARY KEY,

    -- Core identity
    full_name TEXT NOT NULL,
    email     TEXT,
    phone     TEXT,
    avatar_url TEXT,

    -- Classification
    -- person_type: 'team' | 'client' | 'contractor' | 'lead' | 'partner' | 'contact'
    person_type TEXT NOT NULL DEFAULT 'contact',

    -- Financial role (Taker = client who spends VIBE, Giver = contributor who earns VIBE)
    -- financial_role: 'taker' | 'giver' | 'both' | 'neutral'
    financial_role TEXT NOT NULL DEFAULT 'neutral',

    -- Client profiles
    -- client_profile: 'startup' | 'retainer' | NULL
    client_profile TEXT,

    -- Startup business lifecycle (for startup-profile clients)
    -- business_stage: 'idea' | 'funding' | 'manufacturing' | 'distribution' | 'customer_acquisition'
    business_stage TEXT,

    -- CRM lifecycle (mirrors crm_contacts.lifecycle_stage)
    lifecycle_stage TEXT NOT NULL DEFAULT 'lead',
    lead_score      INTEGER NOT NULL DEFAULT 0,

    -- Professional details
    company_name TEXT,
    job_title    TEXT,
    website      TEXT,

    -- Bridge FKs to existing tables (NULL = not linked yet)
    user_id         BLOB,   -- links to users.id
    crm_contact_id  BLOB,   -- links to crm_contacts.id
    organization_id BLOB,   -- links to organizations.id

    -- AI-gathered online presence intelligence
    intelligence_summary    TEXT,   -- Short AI-written profile paragraph
    intelligence_raw        TEXT,   -- Full JSON from research run
    intelligence_last_run_at TEXT,
    intelligence_confidence REAL NOT NULL DEFAULT 0.0,

    -- Free-form
    notes         TEXT,
    tags          TEXT NOT NULL DEFAULT '[]',
    custom_fields TEXT NOT NULL DEFAULT '{}',

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- ============================================================
-- 2. person_social_profiles — social media rail on contact card
-- ============================================================
CREATE TABLE IF NOT EXISTS person_social_profiles (
    id         BLOB PRIMARY KEY,
    person_id  BLOB NOT NULL,   -- FK → persons.id

    -- platform: 'linkedin' | 'twitter' | 'instagram' | 'youtube' | 'tiktok' | 'github' | 'facebook' | 'website'
    platform TEXT NOT NULL,

    handle         TEXT,
    profile_url    TEXT,
    follower_count INTEGER,
    following_count INTEGER,
    bio            TEXT,
    verified       INTEGER NOT NULL DEFAULT 0,
    raw_data       TEXT,   -- JSON from platform API / Nora research
    last_synced_at TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE (person_id, platform)
);

-- ============================================================
-- 3. invoices — AR (client owes PCG) and AP (PCG owes contractor)
-- ============================================================
CREATE TABLE IF NOT EXISTS invoices (
    id             BLOB PRIMARY KEY,
    invoice_number TEXT NOT NULL UNIQUE,

    -- Who the invoice is with
    person_id      BLOB,   -- FK → persons.id
    organization_id BLOB,  -- optional org link

    -- What project/scope it covers
    project_id BLOB,

    -- AR = Accounts Receivable (client owes PCG), AP = Accounts Payable (PCG owes contributor)
    invoice_type TEXT NOT NULL DEFAULT 'ar',

    -- status: 'draft' | 'sent' | 'viewed' | 'paid' | 'partial' | 'overdue' | 'void' | 'cancelled'
    status TEXT NOT NULL DEFAULT 'draft',

    -- Amounts
    amount_usd  REAL    NOT NULL DEFAULT 0.0,
    amount_vibe INTEGER NOT NULL DEFAULT 0,
    currency    TEXT    NOT NULL DEFAULT 'USD',

    -- Content
    title       TEXT,
    description TEXT,
    line_items  TEXT NOT NULL DEFAULT '[]',  -- JSON [{description, qty, rate, amount}]

    -- Dates
    issue_date TEXT,
    due_date   TEXT,
    paid_at    TEXT,

    -- Payment tracking
    payment_method    TEXT,
    payment_reference TEXT,

    notes    TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',

    created_by BLOB,   -- FK → users.id
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- ============================================================
-- 4. Bridge columns on existing tables
-- ============================================================
ALTER TABLE users ADD COLUMN person_id BLOB;
ALTER TABLE crm_contacts ADD COLUMN person_id BLOB;

-- ============================================================
-- 5. Seed: auto-create Person records from existing users
-- ============================================================
INSERT OR IGNORE INTO persons
    (id, full_name, email, avatar_url, person_type, financial_role,
     lifecycle_stage, user_id, created_at, updated_at)
SELECT
    u.id,
    COALESCE(NULLIF(TRIM(u.full_name), ''), NULLIF(u.email, ''), 'Unknown'),
    u.email,
    u.avatar_url,
    CASE
        WHEN u.is_admin = 1     THEN 'team'
        WHEN u.user_role = 'host' THEN 'team'
        ELSE 'client'
    END,
    CASE
        WHEN u.is_admin = 1     THEN 'giver'
        WHEN u.user_role = 'host' THEN 'giver'
        ELSE 'both'
    END,
    'customer',
    u.id,
    u.created_at,
    u.updated_at
FROM users u
WHERE u.deleted_at IS NULL;

UPDATE users SET person_id = id WHERE deleted_at IS NULL;

-- ============================================================
-- 6. Seed: auto-create Person records from existing crm_contacts
-- ============================================================
INSERT OR IGNORE INTO persons
    (id, full_name, email, phone, avatar_url, company_name, job_title, website,
     person_type, financial_role, lifecycle_stage, lead_score,
     crm_contact_id, organization_id, created_at, updated_at)
SELECT
    c.id,
    COALESCE(
        NULLIF(TRIM(COALESCE(c.full_name, '')), ''),
        NULLIF(TRIM(COALESCE(c.first_name, '') || ' ' || COALESCE(c.last_name, '')), ''),
        NULLIF(c.email, ''),
        'Unknown'
    ),
    c.email,
    COALESCE(c.phone, c.mobile),
    c.avatar_url,
    c.company_name,
    c.job_title,
    c.website,
    'lead',
    'taker',
    COALESCE(c.lifecycle_stage, 'lead'),
    COALESCE(c.lead_score, 0),
    c.id,
    NULL,  -- organization_id not yet on crm_contacts
    c.created_at,
    c.updated_at
FROM crm_contacts c
WHERE NOT EXISTS (SELECT 1 FROM persons WHERE id = c.id);

UPDATE crm_contacts SET person_id = id WHERE person_id IS NULL;

-- ============================================================
-- 7. Indexes
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_persons_email          ON persons(email);
CREATE INDEX IF NOT EXISTS idx_persons_person_type    ON persons(person_type);
CREATE INDEX IF NOT EXISTS idx_persons_financial_role ON persons(financial_role);
CREATE INDEX IF NOT EXISTS idx_persons_user_id        ON persons(user_id);
CREATE INDEX IF NOT EXISTS idx_persons_crm_contact_id ON persons(crm_contact_id);
CREATE INDEX IF NOT EXISTS idx_persons_org            ON persons(organization_id);

CREATE INDEX IF NOT EXISTS idx_psp_person_id ON person_social_profiles(person_id);
CREATE INDEX IF NOT EXISTS idx_psp_platform  ON person_social_profiles(platform);

CREATE INDEX IF NOT EXISTS idx_invoices_person_id    ON invoices(person_id);
CREATE INDEX IF NOT EXISTS idx_invoices_project_id   ON invoices(project_id);
CREATE INDEX IF NOT EXISTS idx_invoices_invoice_type ON invoices(invoice_type);
CREATE INDEX IF NOT EXISTS idx_invoices_status       ON invoices(status);
