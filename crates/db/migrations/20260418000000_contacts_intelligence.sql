-- Contacts Unification Phase 1: Add intelligence fields to crm_contacts
--
-- The Intel tab reads from persons.intelligence_summary, but CRM-created
-- contacts have no person record. This adds intelligence fields directly
-- to crm_contacts so Scout can write there without a person bridge.

-- ── Add intelligence columns ─────────────────────────────────────────────
ALTER TABLE crm_contacts ADD COLUMN intelligence_summary TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_status TEXT NOT NULL DEFAULT 'idle';
ALTER TABLE crm_contacts ADD COLUMN intelligence_raw TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_confidence REAL NOT NULL DEFAULT 0.0;
ALTER TABLE crm_contacts ADD COLUMN intelligence_last_run_at TEXT;
ALTER TABLE crm_contacts ADD COLUMN intelligence_agent TEXT;
ALTER TABLE crm_contacts ADD COLUMN research_pass_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE crm_contacts ADD COLUMN research_depth TEXT NOT NULL DEFAULT 'shallow';

-- Bridge columns for transition period
ALTER TABLE crm_contacts ADD COLUMN company_id TEXT;
ALTER TABLE crm_contacts ADD COLUMN person_id TEXT;

-- ── Backfill from linked persons ─────────────────────────────────────────
-- Only update contacts that have a linked person with data
UPDATE crm_contacts SET
    intelligence_summary = (SELECT p.intelligence_summary FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    intelligence_status = COALESCE(
        (SELECT p.intelligence_status FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
        'idle'
    ),
    intelligence_raw = (SELECT p.intelligence_raw FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    intelligence_confidence = COALESCE(
        (SELECT p.intelligence_confidence FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
        0.0
    ),
    intelligence_last_run_at = (SELECT p.intelligence_last_run_at FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    intelligence_agent = (SELECT p.intelligence_agent FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    research_pass_count = COALESCE(
        (SELECT p.research_pass_count FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
        0
    ),
    research_depth = COALESCE(
        (SELECT p.research_depth FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
        'shallow'
    ),
    company_id = (SELECT p.company_id FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    person_id = (SELECT CAST(p.id AS TEXT) FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT))
WHERE EXISTS (
    SELECT 1 FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)
);

-- Index for intelligence status queries
CREATE INDEX IF NOT EXISTS idx_crm_contacts_intel_status ON crm_contacts(intelligence_status);
