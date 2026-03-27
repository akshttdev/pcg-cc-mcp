-- Contacts Unification Phase 2 W3: Re-FK child tables to crm_contacts
--
-- person_research_passes and person_social_profiles currently FK to persons(id).
-- Re-FK them to crm_contacts(id) via the persons.crm_contact_id bridge.
-- business_reports gets a crm_contact_id column backfilled from persons.

-- ── 1. person_research_passes → contact_research_passes ──────────────────────

CREATE TABLE contact_research_passes (
    id                  TEXT NOT NULL PRIMARY KEY,
    crm_contact_id      TEXT NOT NULL REFERENCES crm_contacts(id) ON DELETE CASCADE,
    pass_number         INTEGER NOT NULL DEFAULT 1,
    research_focus      TEXT NOT NULL,
    focus_prompt        TEXT,
    status              TEXT NOT NULL DEFAULT 'pending',
    summary             TEXT,
    raw_results         TEXT,
    key_findings        TEXT DEFAULT '[]',
    search_queries      TEXT DEFAULT '[]',
    confidence_delta    REAL DEFAULT 0.0,
    agent_used          TEXT,
    tokens_used         INTEGER,
    error               TEXT,
    created_at          TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    completed_at        TEXT
);

-- Migrate data: map person_id → crm_contact_id via persons bridge
-- Only migrate rows with valid contact links (skip orphaned rows)
INSERT INTO contact_research_passes
SELECT
    prp.id,
    p.crm_contact_id,
    prp.pass_number, prp.research_focus, prp.focus_prompt, prp.status,
    prp.summary, prp.raw_results, prp.key_findings, prp.search_queries,
    prp.confidence_delta, prp.agent_used, prp.tokens_used, prp.error,
    prp.created_at, prp.completed_at
FROM person_research_passes prp
JOIN persons p ON CAST(p.id AS TEXT) = CAST(prp.person_id AS TEXT)
WHERE p.crm_contact_id IS NOT NULL;

DROP TABLE IF EXISTS person_research_passes;

CREATE INDEX idx_contact_research_passes_contact ON contact_research_passes(crm_contact_id, pass_number);

-- ── 2. person_social_profiles → contact_social_profiles ──────────────────────

CREATE TABLE contact_social_profiles (
    id                TEXT PRIMARY KEY NOT NULL,
    crm_contact_id    TEXT NOT NULL REFERENCES crm_contacts(id) ON DELETE CASCADE,
    platform          TEXT NOT NULL,
    handle            TEXT,
    profile_url       TEXT,
    follower_count    INTEGER,
    following_count   INTEGER,
    bio               TEXT,
    verified          INTEGER NOT NULL DEFAULT 0,
    raw_data          TEXT,
    last_synced_at    TEXT,
    created_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE (crm_contact_id, platform)
);

-- Only migrate rows with valid contact links
INSERT INTO contact_social_profiles
SELECT
    sp.id,
    p.crm_contact_id,
    sp.platform, sp.handle, sp.profile_url, sp.follower_count, sp.following_count,
    sp.bio, sp.verified, sp.raw_data, sp.last_synced_at, sp.created_at, sp.updated_at
FROM person_social_profiles sp
JOIN persons p ON CAST(p.id AS TEXT) = CAST(sp.person_id AS TEXT)
WHERE p.crm_contact_id IS NOT NULL;

DROP TABLE IF EXISTS person_social_profiles;

CREATE INDEX idx_csp_contact ON contact_social_profiles(crm_contact_id);
CREATE INDEX idx_csp_platform ON contact_social_profiles(platform);

-- ── 3. business_reports: add crm_contact_id ──────────────────────────────────

-- business_reports uses BLOB ids (not yet migrated to TEXT for this column)
-- Add crm_contact_id TEXT column, backfill from persons bridge
ALTER TABLE business_reports ADD COLUMN crm_contact_id TEXT;

UPDATE business_reports SET crm_contact_id = (
    SELECT CAST(p.crm_contact_id AS TEXT)
    FROM persons p
    WHERE CAST(p.id AS TEXT) = CAST(business_reports.person_id AS TEXT)
)
WHERE person_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_business_reports_contact ON business_reports(crm_contact_id);
