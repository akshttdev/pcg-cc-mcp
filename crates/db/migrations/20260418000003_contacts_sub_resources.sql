-- PC-6: Migrate person sub-resource tables to contacts
-- Adds crm_contact_id, backfills from persons bridge, then renames tables.

-- ── 1. person_notes → contact_notes ──────────────────────────────────────────
ALTER TABLE person_notes ADD COLUMN crm_contact_id TEXT REFERENCES crm_contacts(id);
UPDATE person_notes SET crm_contact_id = (
    SELECT crm_contact_id FROM persons WHERE CAST(persons.id AS TEXT) = CAST(person_notes.person_id AS TEXT)
) WHERE crm_contact_id IS NULL;
ALTER TABLE person_notes RENAME TO contact_notes;

-- ── 2. person_company_roles → contact_company_roles ──────────────────────────
ALTER TABLE person_company_roles ADD COLUMN crm_contact_id TEXT REFERENCES crm_contacts(id);
UPDATE person_company_roles SET crm_contact_id = (
    SELECT crm_contact_id FROM persons WHERE CAST(persons.id AS TEXT) = CAST(person_company_roles.person_id AS TEXT)
) WHERE crm_contact_id IS NULL;
ALTER TABLE person_company_roles RENAME TO contact_company_roles;
-- Add unique index on (crm_contact_id, company_id) for upsert support
CREATE UNIQUE INDEX IF NOT EXISTS idx_ccr_contact_company
    ON contact_company_roles(crm_contact_id, company_id)
    WHERE crm_contact_id IS NOT NULL;

-- ── 3. person_organization_contacts → contact_organization_links ─────────────
ALTER TABLE person_organization_contacts ADD COLUMN crm_contact_id TEXT REFERENCES crm_contacts(id);
UPDATE person_organization_contacts SET crm_contact_id = (
    SELECT crm_contact_id FROM persons WHERE CAST(persons.id AS TEXT) = CAST(person_organization_contacts.person_id AS TEXT)
) WHERE crm_contact_id IS NULL;
ALTER TABLE person_organization_contacts RENAME TO contact_organization_links;
-- Add unique index on (crm_contact_id, organization_id) for upsert support
CREATE UNIQUE INDEX IF NOT EXISTS idx_col_contact_org
    ON contact_organization_links(crm_contact_id, organization_id)
    WHERE crm_contact_id IS NOT NULL;

-- ── 4. invoices: add crm_contact_id (keep table name) ───────────────────────
ALTER TABLE invoices ADD COLUMN crm_contact_id TEXT REFERENCES crm_contacts(id);
UPDATE invoices SET crm_contact_id = (
    SELECT crm_contact_id FROM persons WHERE CAST(persons.id AS TEXT) = CAST(invoices.person_id AS TEXT)
) WHERE crm_contact_id IS NULL AND person_id IS NOT NULL;

-- ── 5. crm_contacts: add fields migrated from persons ───────────────────────
ALTER TABLE crm_contacts ADD COLUMN onboarding_channel TEXT;
ALTER TABLE crm_contacts ADD COLUMN preferred_contact TEXT;
ALTER TABLE crm_contacts ADD COLUMN person_type TEXT;
ALTER TABLE crm_contacts ADD COLUMN financial_role TEXT;

-- Backfill from persons
UPDATE crm_contacts SET
    onboarding_channel = (SELECT onboarding_channel FROM persons WHERE CAST(persons.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    preferred_contact = (SELECT preferred_contact FROM persons WHERE CAST(persons.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    person_type = (SELECT person_type FROM persons WHERE CAST(persons.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT)),
    financial_role = (SELECT financial_role FROM persons WHERE CAST(persons.crm_contact_id AS TEXT) = CAST(crm_contacts.id AS TEXT))
WHERE id IN (SELECT crm_contact_id FROM persons WHERE crm_contact_id IS NOT NULL);
