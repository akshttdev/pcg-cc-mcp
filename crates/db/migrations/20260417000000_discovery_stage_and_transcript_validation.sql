-- Add Discovery stage to existing Acquisition/Sales pipelines that don't have one.
-- The hardcoded create_sales_pipeline() already creates Discovery at position 2,
-- but pipelines created before that code (e.g., Sirak Studios Acquisition) may be missing it.

-- Step 1: Shift positions up by 1 for stages at position >= 3.
-- Process from highest position down to avoid unique constraint violations on (pipeline_id, position).
-- Use a large offset first, then shift back to avoid intermediate conflicts.
UPDATE crm_pipeline_stages
SET position = position + 100,
    updated_at = datetime('now', 'subsec')
WHERE pipeline_id IN (
    SELECT p.id FROM crm_pipelines p
    WHERE p.pipeline_type = 'sales'
      AND p.id NOT IN (
          SELECT pipeline_id FROM crm_pipeline_stages WHERE stage_type = 'discovery'
      )
)
AND position >= 3;

-- Now shift from +100 offset to +1 offset (target positions)
UPDATE crm_pipeline_stages
SET position = position - 99,
    updated_at = datetime('now', 'subsec')
WHERE position >= 103;

-- Step 2: Insert Discovery stage at position 3 for pipelines that don't have it
INSERT INTO crm_pipeline_stages (id, pipeline_id, name, description, color, position, is_closed, is_won, probability, stage_type, stage_config, created_at, updated_at)
SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))),
    p.id,
    'Discovery',
    'Discovery call with the prospect — gather requirements, assess fit, log transcript',
    '#8B5CF6',
    3,
    0,
    0,
    40,
    'discovery',
    '{"auto_trigger":false,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"create_review_task","description":"Conduct discovery call and link transcript"}],"on_exit_validations":[{"type":"require_transcript_or_source","message":"No transcript or data source linked — proposals will lack client-specific insights"}],"stage_owner":{"label":"Account Manager + Nora","type":"team"}}',
    datetime('now', 'subsec'),
    datetime('now', 'subsec')
FROM crm_pipelines p
WHERE p.pipeline_type = 'sales'
  AND p.id NOT IN (
      SELECT pipeline_id FROM crm_pipeline_stages WHERE stage_type = 'discovery'
  );

-- Step 3: Update existing Discovery stages (from create_sales_pipeline) to use the new validation
UPDATE crm_pipeline_stages
SET stage_config = '{"auto_trigger":false,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"create_review_task","description":"Conduct discovery call and link transcript"}],"on_exit_validations":[{"type":"require_transcript_or_source","message":"No transcript or data source linked — proposals will lack client-specific insights"}],"stage_owner":{"label":"Account Manager + Nora","type":"team"}}',
    updated_at = datetime('now', 'subsec')
WHERE stage_type = 'discovery';
