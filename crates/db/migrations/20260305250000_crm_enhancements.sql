-- CRM Enhancements (Atomic CRM patterns):
-- 1. Structured multi-email/phone on persons — emails/phones JSON arrays
-- 2. Rep assignment on persons — assigned_to FK to users
-- 3. person_notes table — structured note threading with status + attachments
-- Note: contact_ids already included in create_business_entities CREATE TABLE.

-- ── 1. Structured multi-email / multi-phone on persons ───────────────────────
ALTER TABLE persons ADD COLUMN emails TEXT NOT NULL DEFAULT '[]';
ALTER TABLE persons ADD COLUMN phones TEXT NOT NULL DEFAULT '[]';

-- Migrate existing single email/phone into the new arrays for existing rows
UPDATE persons
SET emails = json_array(json_object('value', email, 'label', 'primary'))
WHERE email IS NOT NULL AND email != '';

UPDATE persons
SET phones = json_array(json_object('value', phone, 'label', 'primary'))
WHERE phone IS NOT NULL AND phone != '';

-- ── 2. Rep assignment ─────────────────────────────────────────────────────────
ALTER TABLE persons ADD COLUMN assigned_to BLOB REFERENCES users(id) ON DELETE SET NULL;

-- ── 3. Person notes ───────────────────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS person_notes (
    id          BLOB PRIMARY KEY,
    person_id   BLOB NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    author_id   BLOB REFERENCES users(id) ON DELETE SET NULL,

    text        TEXT NOT NULL,

    -- 'open' | 'follow_up' | 'resolved' | 'pinned'
    status      TEXT NOT NULL DEFAULT 'open'
                CHECK (status IN ('open', 'follow_up', 'resolved', 'pinned')),

    -- JSON array of {name, url, mime_type, size_bytes}
    attachments TEXT NOT NULL DEFAULT '[]',

    -- Optional link to a specific proposal
    proposal_id BLOB REFERENCES proposals(id) ON DELETE SET NULL,

    created_at  TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at  TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_person_notes_person    ON person_notes(person_id);
CREATE INDEX IF NOT EXISTS idx_person_notes_author    ON person_notes(author_id);
CREATE INDEX IF NOT EXISTS idx_person_notes_status    ON person_notes(status);
CREATE INDEX IF NOT EXISTS idx_person_notes_proposal  ON person_notes(proposal_id);
