-- Many-to-many: person ↔ company (with role/title at that company)
CREATE TABLE IF NOT EXISTS person_company_roles (
    id           BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    person_id    BLOB NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    company_id   BLOB NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    role         TEXT NOT NULL DEFAULT 'contact',
    title        TEXT,
    is_primary   INTEGER NOT NULL DEFAULT 0,
    start_date   TEXT,
    end_date     TEXT,
    notes        TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(person_id, company_id)
);

-- Many-to-many: person ↔ organization (person is a contact/prospect of a platform workspace)
CREATE TABLE IF NOT EXISTS person_organization_contacts (
    id              BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    person_id       BLOB NOT NULL REFERENCES persons(id) ON DELETE CASCADE,
    organization_id BLOB NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
    context         TEXT NOT NULL DEFAULT 'contact',
    notes           TEXT,
    added_at        TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(person_id, organization_id)
);

-- Company-level contact methods (not person-specific)
CREATE TABLE IF NOT EXISTS company_contact_methods (
    id          BLOB PRIMARY KEY DEFAULT (randomblob(16)),
    company_id  BLOB NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
    method_type TEXT NOT NULL,
    label       TEXT,
    value       TEXT NOT NULL,
    is_primary  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

CREATE INDEX IF NOT EXISTS idx_pcr_person   ON person_company_roles(person_id);
CREATE INDEX IF NOT EXISTS idx_pcr_company  ON person_company_roles(company_id);
CREATE INDEX IF NOT EXISTS idx_poc_person   ON person_organization_contacts(person_id);
CREATE INDEX IF NOT EXISTS idx_poc_org      ON person_organization_contacts(organization_id);
CREATE INDEX IF NOT EXISTS idx_ccm_company  ON company_contact_methods(company_id);

-- Migrate existing single-FK data into junction tables
INSERT OR IGNORE INTO person_company_roles (person_id, company_id, role, title, is_primary)
SELECT id, company_id, 'contact', job_title, 1
FROM persons WHERE company_id IS NOT NULL;

INSERT OR IGNORE INTO person_organization_contacts (person_id, organization_id, context)
SELECT id, organization_id, 'contact'
FROM persons WHERE organization_id IS NOT NULL;
