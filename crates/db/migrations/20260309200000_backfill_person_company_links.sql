-- Backfill: link persons to companies where company_name matches companies.name
-- Only inserts if no junction entry exists yet (handles both exact and case-insensitive matches)

INSERT OR IGNORE INTO person_company_roles (person_id, company_id, role, title, is_primary)
SELECT p.id, c.id, 'contact', p.job_title, 1
FROM persons p
JOIN companies c ON lower(c.name) = lower(p.company_name)
WHERE p.company_name IS NOT NULL AND length(p.company_name) > 0
  AND p.id NOT IN (SELECT person_id FROM person_company_roles WHERE company_id = c.id);
