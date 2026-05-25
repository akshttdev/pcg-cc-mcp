-- Model Pickleball — Phase 1: form-submission tables
--
-- The model-pickleball-web Astro site already POSTs to 8 endpoints under
-- /api/public/model-pickleball/* but none of them exist in this repo yet
-- (forms 404 in prod today). This migration adds the persistence layer for
-- those 8 endpoints. The HTTP handlers land in a follow-up commit on this
-- same branch.
--
-- The 8 endpoints collapse into 4 tables:
--   newsletter_subscribers   POST /newsletter
--   mp_contact_submissions   POST /contact
--   mp_registrations         POST /registrations/{model|agency}
--   mp_inquiries             POST /inquiries/{brand|sponsor|media|venue}
--
-- Uses BLOB ids + organization_id FK to match recent practice
-- (20260426000000_video_studio.sql et al.) since organizations.id is BLOB.
-- The TEXT-UUID rule in .claude/rules/database.md is forward-looking; once
-- organizations is migrated, MP tables can follow.

PRAGMA foreign_keys = ON;

-- -----------------------------------------------------------------------------
-- newsletter_subscribers
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id BLOB NOT NULL REFERENCES organizations(id),
    email           TEXT NOT NULL,
    source          TEXT NOT NULL DEFAULT 'model_pickleball',
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at      TEXT,
    UNIQUE (organization_id, email)
);
CREATE INDEX IF NOT EXISTS idx_newsletter_org   ON newsletter_subscribers(organization_id);
CREATE INDEX IF NOT EXISTS idx_newsletter_email ON newsletter_subscribers(email);

-- -----------------------------------------------------------------------------
-- mp_contact_submissions
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mp_contact_submissions (
    id                 BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id    BLOB NOT NULL REFERENCES organizations(id),
    first_name         TEXT NOT NULL,
    last_name          TEXT NOT NULL,
    company            TEXT,
    email              TEXT NOT NULL,
    phone              TEXT NOT NULL,
    instagram          TEXT,
    website            TEXT,
    inquiry            TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'new'
                         CHECK (status IN ('new','contacted','qualified','closed','spam')),
    handled_by         BLOB REFERENCES users(id),
    handled_at         TEXT,
    cf_turnstile_token TEXT,
    ip_address         TEXT,
    user_agent         TEXT,
    created_at         TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at         TEXT
);
CREATE INDEX IF NOT EXISTS idx_mp_contacts_org    ON mp_contact_submissions(organization_id);
CREATE INDEX IF NOT EXISTS idx_mp_contacts_email  ON mp_contact_submissions(email);
CREATE INDEX IF NOT EXISTS idx_mp_contacts_status ON mp_contact_submissions(status);

-- -----------------------------------------------------------------------------
-- mp_registrations (kind=model|agency, kind-specific fields in payload JSON)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mp_registrations (
    id                 BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id    BLOB NOT NULL REFERENCES organizations(id),
    kind               TEXT NOT NULL CHECK (kind IN ('model','agency')),
    email              TEXT NOT NULL,
    phone              TEXT NOT NULL,
    payload            TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'new'
                         CHECK (status IN ('new','contacted','approved','rejected','spam')),
    handled_by         BLOB REFERENCES users(id),
    handled_at         TEXT,
    cf_turnstile_token TEXT,
    ip_address         TEXT,
    user_agent         TEXT,
    created_at         TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at         TEXT
);
CREATE INDEX IF NOT EXISTS idx_mp_regs_org     ON mp_registrations(organization_id, kind, status);
CREATE INDEX IF NOT EXISTS idx_mp_regs_email   ON mp_registrations(email);
CREATE INDEX IF NOT EXISTS idx_mp_regs_created ON mp_registrations(created_at);

-- -----------------------------------------------------------------------------
-- mp_inquiries (kind=brand|sponsor|media|venue, kind-specific fields in payload JSON)
-- -----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS mp_inquiries (
    id                 BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    organization_id    BLOB NOT NULL REFERENCES organizations(id),
    kind               TEXT NOT NULL CHECK (kind IN ('brand','sponsor','media','venue')),
    email              TEXT NOT NULL,
    phone              TEXT,
    payload            TEXT NOT NULL,
    status             TEXT NOT NULL DEFAULT 'new'
                         CHECK (status IN ('new','contacted','qualified','won','lost','spam')),
    handled_by         BLOB REFERENCES users(id),
    handled_at         TEXT,
    cf_turnstile_token TEXT,
    ip_address         TEXT,
    user_agent         TEXT,
    created_at         TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    deleted_at         TEXT
);
CREATE INDEX IF NOT EXISTS idx_mp_inq_org     ON mp_inquiries(organization_id, kind, status);
CREATE INDEX IF NOT EXISTS idx_mp_inq_email   ON mp_inquiries(email);
CREATE INDEX IF NOT EXISTS idx_mp_inq_created ON mp_inquiries(created_at);
