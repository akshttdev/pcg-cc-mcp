-- Migration A: Client ↔ Knowledge Graph links
ALTER TABLE clients ADD COLUMN company_id TEXT REFERENCES companies(id);
ALTER TABLE clients ADD COLUMN primary_person_id TEXT REFERENCES persons(id);
ALTER TABLE clients ADD COLUMN prospect_at TEXT;
ALTER TABLE clients ADD COLUMN client_since TEXT;

-- Migration B: Business Report ↔ Client link
ALTER TABLE business_reports ADD COLUMN client_id TEXT REFERENCES clients(id);

-- Migration C: Deliverable ↔ Business Report link (for PDF artifact)
ALTER TABLE deliverables ADD COLUMN business_report_id TEXT REFERENCES business_reports(id);

-- Backfill: link existing clients to companies by name (exact case-insensitive match)
-- companies.id is BLOB; store as lowercase UUID string with dashes
UPDATE clients
SET company_id = (
    SELECT lower(
        hex(substr(c.id,1,4)) || '-' ||
        hex(substr(c.id,5,2)) || '-' ||
        hex(substr(c.id,7,2)) || '-' ||
        hex(substr(c.id,9,2)) || '-' ||
        hex(substr(c.id,11,6))
    )
    FROM companies c
    WHERE lower(c.name) = lower(clients.name)
    LIMIT 1
)
WHERE (
    SELECT 1 FROM companies c WHERE lower(c.name) = lower(clients.name) LIMIT 1
) IS NOT NULL;
