-- ─────────────────────────────────────────────────────────────────────────────
-- CRM Pipeline: Proposals, Deliverables, Operator Rates
-- 2026-03-03
-- ─────────────────────────────────────────────────────────────────────────────

-- ── Proposals ────────────────────────────────────────────────────────────────
-- 8-stage pipeline: drafted → pending_approval → approved → meeting_scheduled
--                   → sent → seen → verbal → contract_signed
-- Terminal: declined | deferred

CREATE TABLE IF NOT EXISTS proposals (
    id               BLOB    PRIMARY KEY,
    -- Relationships
    lead_id          BLOB    REFERENCES persons(id) ON DELETE SET NULL,
    organization_id  BLOB    REFERENCES organizations(id) ON DELETE SET NULL,
    owner_id         BLOB    REFERENCES users(id) ON DELETE SET NULL,
    project_id       BLOB    REFERENCES projects(id) ON DELETE SET NULL,
    -- Stage
    status           TEXT    NOT NULL DEFAULT 'drafted'
                     CHECK(status IN ('drafted','pending_approval','approved',
                                      'meeting_scheduled','sent','seen',
                                      'verbal','contract_signed',
                                      'declined','deferred')),
    -- Content
    title            TEXT    NOT NULL DEFAULT '',
    description      TEXT    NOT NULL DEFAULT '',
    quote_amount_vibe INTEGER NOT NULL DEFAULT 0,
    deal_type        TEXT    NOT NULL DEFAULT 'one-off'
                     CHECK(deal_type IN ('one-off','retainer','hybrid')),
    -- Timestamps
    sent_at          TEXT,
    seen_at          TEXT,
    verbal_at        TEXT,
    signed_at        TEXT,
    declined_at      TEXT,
    created_at       TEXT    NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at       TEXT    NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_proposals_lead       ON proposals(lead_id);
CREATE INDEX IF NOT EXISTS idx_proposals_project    ON proposals(project_id);
CREATE INDEX IF NOT EXISTS idx_proposals_owner      ON proposals(owner_id);
CREATE INDEX IF NOT EXISTS idx_proposals_status     ON proposals(status);

-- ── Deliverables ─────────────────────────────────────────────────────────────
-- 6-stage: working → internal_review → client_review → revision
--          → client_revision → done

CREATE TABLE IF NOT EXISTS deliverables (
    id                      BLOB    PRIMARY KEY,
    project_id              BLOB    NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    -- Optional link to a proposal
    proposal_id             BLOB    REFERENCES proposals(id) ON DELETE SET NULL,
    -- Classification
    deliverable_type        TEXT    NOT NULL DEFAULT 'other'
                            CHECK(deliverable_type IN ('video','audio','graphic','copy',
                                                       'code','document','other')),
    title                   TEXT    NOT NULL DEFAULT '',
    description             TEXT    NOT NULL DEFAULT '',
    -- Status
    status                  TEXT    NOT NULL DEFAULT 'working'
                            CHECK(status IN ('working','internal_review','client_review',
                                             'revision','client_revision','done')),
    -- Revisions
    revision_rounds_allowed INTEGER NOT NULL DEFAULT 2,
    revision_rounds_used    INTEGER NOT NULL DEFAULT 0,
    -- Files
    working_file_url        TEXT,
    final_link              TEXT,
    -- Scheduling
    due_date                TEXT,
    delivered_at            TEXT,
    -- Timestamps
    created_at              TEXT    NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at              TEXT    NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_deliverables_project  ON deliverables(project_id);
CREATE INDEX IF NOT EXISTS idx_deliverables_proposal ON deliverables(proposal_id);
CREATE INDEX IF NOT EXISTS idx_deliverables_status   ON deliverables(status);

-- ── Operator Rates ────────────────────────────────────────────────────────────
-- Private rates per operator per organization — admin/PM eyes only.
-- duration = 'hourly' | 'daily' | 'weekly' | 'monthly' | 'project'

CREATE TABLE IF NOT EXISTS operator_rates (
    id              BLOB    PRIMARY KEY,
    organization_id BLOB    NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    operator_id     BLOB    NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    duration        TEXT    NOT NULL DEFAULT 'hourly'
                    CHECK(duration IN ('hourly','daily','weekly','monthly','project')),
    rate_vibe       INTEGER NOT NULL DEFAULT 0,
    rate_usd        REAL    NOT NULL DEFAULT 0.0,
    notes           TEXT,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id, operator_id, duration)
);

CREATE INDEX IF NOT EXISTS idx_operator_rates_org  ON operator_rates(organization_id);
CREATE INDEX IF NOT EXISTS idx_operator_rates_user ON operator_rates(operator_id);

-- ── Extend projects ───────────────────────────────────────────────────────────
ALTER TABLE projects ADD COLUMN client_budget_vibe     INTEGER;
ALTER TABLE projects ADD COLUMN dropbox_folder_url     TEXT;
ALTER TABLE projects ADD COLUMN waiting_on_client      INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN project_status         TEXT NOT NULL DEFAULT 'active'
    CHECK(project_status IN ('active','paused','waiting_on_client','complete','cancelled'));
ALTER TABLE projects ADD COLUMN account_manager_id     BLOB REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN pm_ep_id               BLOB REFERENCES users(id) ON DELETE SET NULL;

-- ── Extend users (operators) ──────────────────────────────────────────────────
ALTER TABLE users ADD COLUMN skill_level           TEXT
    CHECK(skill_level IN ('junior','mid','senior','expert') OR skill_level IS NULL);
ALTER TABLE users ADD COLUMN compliance_status     TEXT NOT NULL DEFAULT 'good'
    CHECK(compliance_status IN ('good','pending','flagged'));
ALTER TABLE users ADD COLUMN primary_tools         TEXT; -- JSON array of tool names

-- ── Extend crm_contacts (leads pipeline) ─────────────────────────────────────
ALTER TABLE crm_contacts ADD COLUMN estimated_budget_range TEXT;
ALTER TABLE crm_contacts ADD COLUMN deal_type              TEXT DEFAULT 'one-off'
    CHECK(deal_type IN ('one-off','retainer','hybrid') OR deal_type IS NULL);
ALTER TABLE crm_contacts ADD COLUMN follow_up_attempts     INTEGER NOT NULL DEFAULT 0;
ALTER TABLE crm_contacts ADD COLUMN waiting_on_client      INTEGER NOT NULL DEFAULT 0;
ALTER TABLE crm_contacts ADD COLUMN discovery_scheduled_at TEXT;
ALTER TABLE crm_contacts ADD COLUMN proposal_meeting_at    TEXT;
