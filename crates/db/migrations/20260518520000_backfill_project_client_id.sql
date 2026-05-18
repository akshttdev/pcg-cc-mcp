-- Fix BUG-6 (partial): backfill client_id on projects created by Won provisioning
-- that are missing it. Matches on org + company name similarity.

UPDATE projects
SET
  client_id = (
    SELECT cl.id
    FROM clients cl
    JOIN crm_deals d ON d.project_id = projects.id
    JOIN crm_contacts cc ON cc.id = d.crm_contact_id
    WHERE cl.organization_id = projects.organization_id
      AND (
        lower(cl.name) = lower(COALESCE(cc.company_name, ''))
        OR lower(cl.name) = lower(d.name)
      )
    LIMIT 1
  ),
  updated_at = datetime('now', 'subsec')
WHERE client_id IS NULL
  AND id IN (
    SELECT project_id
    FROM crm_deals
    WHERE won_at IS NOT NULL
      AND project_id IS NOT NULL
  );
