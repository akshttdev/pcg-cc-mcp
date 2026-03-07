-- ============================================================
-- Migration: DB Drift Fixes + User Knowledge Graph
-- 2026-03-06
-- ============================================================
-- NOTE: companies table, organizations.company_id, persons.company_id,
-- and project_knowledge_sources.owner_type/owner_id were already
-- applied in migration 20260305200000. This migration adds:
--   1. Drift fixes (empty string → NULL) for runtime safety
--   2. user_knowledge_sources table (user-personal knowledge graph)

-- ── Part 1: Fix empty string drift ───────────────────────────
-- These "" values cause sqlx Option<DateTime> deserialization
-- failures at runtime. Idempotent (NULL WHERE '' is a no-op if clean).

-- crm_contacts date fields
UPDATE crm_contacts SET last_activity_at      = NULL WHERE last_activity_at      = '';
UPDATE crm_contacts SET last_contacted_at     = NULL WHERE last_contacted_at     = '';
UPDATE crm_contacts SET last_replied_at       = NULL WHERE last_replied_at       = '';
UPDATE crm_contacts SET discovery_scheduled_at= NULL WHERE discovery_scheduled_at= '';
UPDATE crm_contacts SET proposal_meeting_at   = NULL WHERE proposal_meeting_at   = '';

-- crm_contacts structural empties
UPDATE crm_contacts SET mobile         = NULL WHERE mobile         = '';
UPDATE crm_contacts SET department     = NULL WHERE department     = '';
UPDATE crm_contacts SET twitter_handle = NULL WHERE twitter_handle = '';
UPDATE crm_contacts SET website        = NULL WHERE website        = '';
UPDATE crm_contacts SET external_ids   = NULL WHERE external_ids   = '';
UPDATE crm_contacts SET lists          = NULL WHERE lists          = '';
UPDATE crm_contacts SET custom_fields  = NULL WHERE custom_fields  = '';
UPDATE crm_contacts SET address_line1  = NULL WHERE address_line1  = '';
UPDATE crm_contacts SET address_line2  = NULL WHERE address_line2  = '';
UPDATE crm_contacts SET city           = NULL WHERE city           = '';
UPDATE crm_contacts SET state          = NULL WHERE state          = '';
UPDATE crm_contacts SET postal_code    = NULL WHERE postal_code    = '';
UPDATE crm_contacts SET country        = NULL WHERE country        = '';

-- crm_deals date + structural empties
UPDATE crm_deals SET expected_close_date = NULL WHERE expected_close_date = '';
UPDATE crm_deals SET actual_close_date   = NULL WHERE actual_close_date   = '';
UPDATE crm_deals SET last_activity_at    = NULL WHERE last_activity_at    = '';
UPDATE crm_deals SET external_ids        = NULL WHERE external_ids        = '';
UPDATE crm_deals SET custom_fields       = NULL WHERE custom_fields       = '';
UPDATE crm_deals SET lost_reason         = NULL WHERE lost_reason         = '';
UPDATE crm_deals SET win_reason          = NULL WHERE win_reason          = '';
UPDATE crm_deals SET zoho_deal_id        = NULL WHERE zoho_deal_id        = '';
UPDATE crm_deals SET owner_user_id       = NULL WHERE owner_user_id       = '';
UPDATE crm_deals SET tags                = NULL WHERE tags                = '';

-- persons
UPDATE persons SET website = NULL WHERE website = '';

-- tasks
UPDATE tasks SET assigned_agent = NULL WHERE assigned_agent = '';

-- projects
UPDATE projects SET setup_script = NULL WHERE setup_script = '';
UPDATE projects SET dev_script   = NULL WHERE dev_script   = '';

-- Back-fill owner_id for any project_knowledge_sources rows missing it
-- (owner_type defaults to 'project' from previous migration)
UPDATE project_knowledge_sources
SET owner_id = LOWER(HEX(project_id))
WHERE owner_id IS NULL;


-- ── Part 2: User knowledge graph ─────────────────────────────
-- Private per-user knowledge: skills, interaction history,
-- personal research, bookmarks, notes.
-- Cross-referenced to companies, persons, and projects.

CREATE TABLE IF NOT EXISTS user_knowledge_sources (
    id           BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    user_id      BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- source_type values:
    -- 'skill' | 'interaction' | 'note' | 'bookmark' |
    -- 'research' | 'conversation' | 'artifact' | 'context_injection'
    source_type  TEXT NOT NULL,
    source_id    TEXT NOT NULL,
    source_title TEXT NOT NULL DEFAULT '',
    source_summary TEXT,

    -- Optional cross-references into the broader knowledge graph
    related_company_id BLOB REFERENCES companies(id),
    related_person_id  BLOB REFERENCES persons(id),
    related_project_id BLOB REFERENCES projects(id),

    coverage_score REAL    NOT NULL DEFAULT 0.0,
    is_active      INTEGER NOT NULL DEFAULT 1,
    is_stale       INTEGER NOT NULL DEFAULT 0,

    last_refreshed_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    created_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(user_id, source_type, source_id)
);

CREATE INDEX IF NOT EXISTS idx_uks_user        ON user_knowledge_sources(user_id);
CREATE INDEX IF NOT EXISTS idx_uks_user_type   ON user_knowledge_sources(user_id, source_type);
CREATE INDEX IF NOT EXISTS idx_uks_company     ON user_knowledge_sources(related_company_id);
CREATE INDEX IF NOT EXISTS idx_uks_person      ON user_knowledge_sources(related_person_id);
CREATE INDEX IF NOT EXISTS idx_uks_project     ON user_knowledge_sources(related_project_id);
