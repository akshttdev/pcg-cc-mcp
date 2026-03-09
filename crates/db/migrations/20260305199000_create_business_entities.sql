-- ============================================================
-- Migration: Create Business Entity Tables
-- 2026-03-05
-- ============================================================
-- Creates the core business entity tables required by later
-- migrations (drift_fix, proposals_company_id, entity_graph,
-- person_associations, etc.).
--
-- Tables created:
--   companies, persons, person_social_profiles,
--   proposals, deliverables, invoices, operator_rates
--
-- Also adds company_id column to organizations and persons.

PRAGMA foreign_keys = ON;

-- ============================================================
-- 1. companies
-- ============================================================

CREATE TABLE IF NOT EXISTS companies (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    name                    TEXT NOT NULL,
    slug                    TEXT UNIQUE,
    website                 TEXT,
    industry                TEXT,
    description             TEXT,
    logo_url                TEXT,
    headquarters            TEXT,

    -- Intelligence / enrichment
    intelligence_summary    TEXT,
    intelligence_raw        TEXT,
    intelligence_status     TEXT NOT NULL DEFAULT 'pending',
    intelligence_last_run_at TEXT,
    intelligence_confidence REAL,
    intelligence_agent      TEXT,

    -- Platform links
    organization_id         BLOB REFERENCES organizations(id) ON DELETE SET NULL,
    created_by_org_id       BLOB REFERENCES organizations(id) ON DELETE SET NULL,

    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_companies_slug           ON companies(slug);
CREATE INDEX IF NOT EXISTS idx_companies_org_id         ON companies(organization_id);
CREATE INDEX IF NOT EXISTS idx_companies_created_by_org ON companies(created_by_org_id);
CREATE INDEX IF NOT EXISTS idx_companies_name           ON companies(name);

-- ============================================================
-- 2. organizations.company_id  (ALTER existing table)
-- ============================================================
-- Links a platform organization to its public company record.

ALTER TABLE organizations ADD COLUMN company_id BLOB REFERENCES companies(id) ON DELETE SET NULL;

-- ============================================================
-- 3. persons
-- ============================================================

CREATE TABLE IF NOT EXISTS persons (
    id                      BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    full_name               TEXT NOT NULL,
    email                   TEXT,
    phone                   TEXT,
    avatar_url              TEXT,

    -- Classification
    person_type             TEXT NOT NULL DEFAULT 'contact',
    financial_role          TEXT NOT NULL DEFAULT 'neutral',
    client_profile          TEXT,
    business_stage          TEXT,
    lifecycle_stage         TEXT NOT NULL DEFAULT 'lead',
    lead_score              INTEGER NOT NULL DEFAULT 0,

    -- Employer / role info
    company_name            TEXT,
    job_title               TEXT,
    website                 TEXT,

    -- Bridge FKs
    user_id                 BLOB REFERENCES users(id) ON DELETE SET NULL,
    crm_contact_id          BLOB,
    organization_id         BLOB REFERENCES organizations(id) ON DELETE SET NULL,
    company_id              BLOB REFERENCES companies(id) ON DELETE SET NULL,

    -- AI intelligence
    intelligence_summary    TEXT,
    intelligence_raw        TEXT,
    intelligence_last_run_at TEXT,
    intelligence_confidence REAL NOT NULL DEFAULT 0.0,

    notes                   TEXT,
    tags                    TEXT NOT NULL DEFAULT '[]',
    custom_fields           TEXT NOT NULL DEFAULT '{}',

    created_at              TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_persons_email     ON persons(email);
CREATE INDEX IF NOT EXISTS idx_persons_type      ON persons(person_type);
CREATE INDEX IF NOT EXISTS idx_persons_org       ON persons(organization_id);
CREATE INDEX IF NOT EXISTS idx_persons_company   ON persons(company_id);
CREATE INDEX IF NOT EXISTS idx_persons_user      ON persons(user_id);
CREATE INDEX IF NOT EXISTS idx_persons_lifecycle ON persons(lifecycle_stage);
CREATE INDEX IF NOT EXISTS idx_persons_name      ON persons(full_name);

-- ============================================================
-- 4. person_social_profiles
-- ============================================================

CREATE TABLE IF NOT EXISTS person_social_profiles (
    id                BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    person_id         BLOB NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    platform          TEXT NOT NULL,
    handle            TEXT,
    profile_url       TEXT,
    follower_count    INTEGER,
    following_count   INTEGER,
    bio               TEXT,
    verified          INTEGER NOT NULL DEFAULT 0,
    raw_data          TEXT,
    last_synced_at    TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(person_id, platform)
);

CREATE INDEX IF NOT EXISTS idx_psp_person   ON person_social_profiles(person_id);
CREATE INDEX IF NOT EXISTS idx_psp_platform ON person_social_profiles(platform);

-- ============================================================
-- 5. proposals
-- ============================================================

CREATE TABLE IF NOT EXISTS proposals (
    id                   BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    lead_id              BLOB REFERENCES persons(id) ON DELETE SET NULL,
    organization_id      BLOB REFERENCES organizations(id) ON DELETE SET NULL,
    owner_id             BLOB REFERENCES users(id) ON DELETE SET NULL,
    project_id           BLOB REFERENCES projects(id) ON DELETE SET NULL,

    status               TEXT NOT NULL DEFAULT 'drafted',
    title                TEXT NOT NULL,
    description          TEXT NOT NULL DEFAULT '',
    quote_amount_vibe    INTEGER NOT NULL DEFAULT 0,
    deal_type            TEXT NOT NULL DEFAULT 'one-off',

    -- JSON array of contact person UUIDs
    contact_ids          TEXT NOT NULL DEFAULT '[]',

    sent_at              TEXT,
    seen_at              TEXT,
    verbal_at            TEXT,
    signed_at            TEXT,
    declined_at          TEXT,
    created_at           TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at           TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_proposals_status  ON proposals(status);
CREATE INDEX IF NOT EXISTS idx_proposals_lead    ON proposals(lead_id);
CREATE INDEX IF NOT EXISTS idx_proposals_org     ON proposals(organization_id);
CREATE INDEX IF NOT EXISTS idx_proposals_owner   ON proposals(owner_id);
CREATE INDEX IF NOT EXISTS idx_proposals_project ON proposals(project_id);

-- ============================================================
-- 6. deliverables
-- ============================================================

CREATE TABLE IF NOT EXISTS deliverables (
    id                       BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    project_id               BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    proposal_id              BLOB REFERENCES proposals(id) ON DELETE SET NULL,

    deliverable_type         TEXT NOT NULL DEFAULT 'other',
    title                    TEXT NOT NULL,
    description              TEXT NOT NULL DEFAULT '',

    status                   TEXT NOT NULL DEFAULT 'working',
    revision_rounds_allowed  INTEGER NOT NULL DEFAULT 2,
    revision_rounds_used     INTEGER NOT NULL DEFAULT 0,

    working_file_url         TEXT,
    final_link               TEXT,
    due_date                 TEXT,
    delivered_at             TEXT,
    created_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_deliverables_project  ON deliverables(project_id);
CREATE INDEX IF NOT EXISTS idx_deliverables_proposal ON deliverables(proposal_id);
CREATE INDEX IF NOT EXISTS idx_deliverables_status   ON deliverables(status);

-- ============================================================
-- 7. invoices
-- ============================================================

CREATE TABLE IF NOT EXISTS invoices (
    id                 BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    invoice_number     TEXT NOT NULL UNIQUE,

    person_id          BLOB REFERENCES persons(id) ON DELETE SET NULL,
    organization_id    BLOB REFERENCES organizations(id) ON DELETE SET NULL,
    project_id         BLOB REFERENCES projects(id) ON DELETE SET NULL,

    invoice_type       TEXT NOT NULL DEFAULT 'ar',
    status             TEXT NOT NULL DEFAULT 'draft',

    amount_usd         REAL NOT NULL DEFAULT 0.0,
    amount_vibe        INTEGER NOT NULL DEFAULT 0,
    currency           TEXT NOT NULL DEFAULT 'USD',

    title              TEXT,
    description        TEXT,
    line_items         TEXT NOT NULL DEFAULT '[]',

    issue_date         TEXT,
    due_date           TEXT,
    paid_at            TEXT,

    payment_method     TEXT,
    payment_reference  TEXT,

    notes              TEXT,
    metadata           TEXT NOT NULL DEFAULT '{}',

    created_by         BLOB REFERENCES users(id) ON DELETE SET NULL,
    created_at         TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at         TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_invoices_person  ON invoices(person_id);
CREATE INDEX IF NOT EXISTS idx_invoices_org     ON invoices(organization_id);
CREATE INDEX IF NOT EXISTS idx_invoices_project ON invoices(project_id);
CREATE INDEX IF NOT EXISTS idx_invoices_type    ON invoices(invoice_type);
CREATE INDEX IF NOT EXISTS idx_invoices_status  ON invoices(status);
CREATE INDEX IF NOT EXISTS idx_invoices_number  ON invoices(invoice_number);

-- ============================================================
-- 8. operator_rates
-- ============================================================

CREATE TABLE IF NOT EXISTS operator_rates (
    id               BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id  BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    operator_id      BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    duration         TEXT NOT NULL,
    rate_vibe        INTEGER NOT NULL DEFAULT 0,
    rate_usd         REAL NOT NULL DEFAULT 0.0,
    notes            TEXT,
    created_at       TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at       TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id, operator_id, duration)
);

CREATE INDEX IF NOT EXISTS idx_oprates_org      ON operator_rates(organization_id);
CREATE INDEX IF NOT EXISTS idx_oprates_operator ON operator_rates(operator_id);
