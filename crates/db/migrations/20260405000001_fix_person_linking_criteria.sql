-- Fix person-to-contact linking criteria.
-- The original migration (20260404000000) used `intelligence_status = 'done'`
-- but some persons have intelligence data (intelligence_summary) without the
-- status being set to 'done'. Use presence of summary as the linking criterion.

UPDATE persons SET
    crm_contact_id = (
        SELECT cc.id FROM crm_contacts cc
        WHERE cc.organization_id = persons.organization_id
          AND cc.full_name = persons.full_name
          AND (cc.company_name = persons.company_name OR (cc.company_name IS NULL AND (persons.company_name IS NULL OR persons.company_name = '')))
        LIMIT 1
    ),
    updated_at = datetime('now','subsec')
WHERE crm_contact_id IS NULL
  AND intelligence_summary IS NOT NULL
  AND EXISTS (
      SELECT 1 FROM crm_contacts cc
      WHERE cc.organization_id = persons.organization_id
        AND cc.full_name = persons.full_name
  );
