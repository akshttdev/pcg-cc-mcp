-- Link intake-generated sales deals to their persons/contacts.
-- The intake pipeline created persons with intelligence data and created deals,
-- but never created crm_contacts or linked them together.
-- Deal names follow "Person Name — Company Name" format matching persons.full_name + company_name.
--
-- Steps:
-- 1. Create crm_contacts for each matched person (scoped to the deal's organization)
-- 2. Link the deal to the new contact (UPDATE crm_deals.crm_contact_id)
-- 3. Set persons.crm_contact_id = new contact id

-- Step 1: Create contacts for persons that match intake deals
INSERT INTO crm_contacts (
    id, organization_id,
    full_name, email, company_name, job_title, avatar_url,
    lifecycle_stage, lead_score,
    created_at, updated_at
)
SELECT
    randomblob(16),
    d.organization_id,
    p.full_name,
    p.email,
    p.company_name,
    p.job_title,
    p.avatar_url,
    'lead',
    0,
    datetime('now','subsec'),
    datetime('now','subsec')
FROM crm_deals d
JOIN crm_pipeline_stages s ON s.id = d.crm_stage_id
JOIN crm_pipelines pip ON pip.id = d.crm_pipeline_id
JOIN persons p ON (
    -- Match: deal name starts with person's full_name
    SUBSTR(d.name, 1, LENGTH(p.full_name)) = p.full_name
    AND (
        p.company_name IS NULL
        OR p.company_name = ''
        OR d.name LIKE '%' || p.company_name || '%'
    )
)
WHERE pip.pipeline_type = 'sales'
  AND d.crm_contact_id IS NULL
  AND p.crm_contact_id IS NULL
  AND p.organization_id = d.organization_id
  AND NOT EXISTS (
      SELECT 1 FROM crm_contacts cc2
      WHERE cc2.organization_id = d.organization_id
        AND cc2.full_name = p.full_name
        AND (cc2.company_name = p.company_name OR (cc2.company_name IS NULL AND (p.company_name IS NULL OR p.company_name = '')))
  );

-- Step 2: Update persons to point to their new crm_contact
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
  AND intelligence_status = 'done'
  AND EXISTS (
      SELECT 1 FROM crm_contacts cc
      WHERE cc.organization_id = persons.organization_id
        AND cc.full_name = persons.full_name
  );

-- Step 3: Link deals to their matching contacts
UPDATE crm_deals SET
    crm_contact_id = (
        SELECT cc.id
        FROM crm_contacts cc
        JOIN persons p ON p.crm_contact_id = cc.id
        WHERE cc.organization_id = crm_deals.organization_id
          AND SUBSTR(crm_deals.name, 1, LENGTH(p.full_name)) = p.full_name
          AND (
              p.company_name IS NULL OR p.company_name = ''
              OR crm_deals.name LIKE '%' || p.company_name || '%'
          )
        LIMIT 1
    ),
    updated_at = datetime('now','subsec')
WHERE crm_contact_id IS NULL
  AND crm_pipeline_id IN (
      SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales'
  );

-- Step 4: Move Le Chateau deals to Closed Won
UPDATE crm_deals SET
    crm_stage_id = (
        SELECT s.id FROM crm_pipeline_stages s
        JOIN crm_pipelines p ON p.id = s.pipeline_id
        WHERE p.id = crm_deals.crm_pipeline_id
          AND s.name = 'Closed Won'
        LIMIT 1
    ),
    probability = 100,
    actual_close_date = datetime('now','subsec'),
    updated_at = datetime('now','subsec')
WHERE name LIKE '%Le Chateau%'
  AND crm_pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales');
