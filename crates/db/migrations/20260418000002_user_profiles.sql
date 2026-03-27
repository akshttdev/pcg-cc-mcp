-- Contacts Unification Phase 4: user_profiles table + persons field migration
--
-- Creates a thin user_profiles table for platform user identity (separate
-- from CRM contacts). Migrates person_type/financial_role into contacts
-- custom_fields JSON. Does NOT drop persons table yet — callers need
-- updating first (tracked in backlog PC-6).

-- ── 1. user_profiles table ───────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS user_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    crm_contact_id TEXT REFERENCES crm_contacts(id),
    display_name TEXT,
    role TEXT DEFAULT 'member',
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(user_id)
);

-- Backfill from persons where user_id is set
INSERT OR IGNORE INTO user_profiles (id, user_id, crm_contact_id, display_name, created_at, updated_at)
SELECT
    CAST(p.id AS TEXT),
    CAST(p.user_id AS TEXT),
    CAST(p.crm_contact_id AS TEXT),
    p.full_name,
    p.created_at,
    p.updated_at
FROM persons p
WHERE p.user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_user_profiles_user ON user_profiles(user_id);

-- ── 2. Migrate person_type/financial_role to contacts custom_fields ──────────

UPDATE crm_contacts SET
    custom_fields = json_set(
        json_set(COALESCE(custom_fields, '{}'),
            '$.person_type',
            (SELECT p.person_type FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT))
        ),
        '$.financial_role',
        (SELECT p.financial_role FROM persons p WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT))
    )
WHERE EXISTS (
    SELECT 1 FROM persons p
    WHERE CAST(p.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)
    AND (p.person_type IS NOT NULL OR p.financial_role IS NOT NULL)
);
