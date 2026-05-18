-- Fix BUG-1: Move 31 deals from phantom 'qualification' stage to Intel
-- Matches each deal to the Intel stage within its own pipeline.
-- Safe to run multiple times (WHERE stage = 'qualification' is idempotent)

UPDATE crm_deals
SET
  crm_stage_id = (
    SELECT s.id
    FROM crm_pipeline_stages s
    WHERE s.pipeline_id = crm_deals.crm_pipeline_id
      AND lower(s.name) = 'intel'
    LIMIT 1
  ),
  stage = 'Intel',
  updated_at = datetime('now', 'subsec')
WHERE stage = 'qualification'
  AND crm_pipeline_id IN (
    SELECT id FROM crm_pipelines WHERE pipeline_type = 'sales'
  );
