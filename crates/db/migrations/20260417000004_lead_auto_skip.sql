-- Enable auto_skip on Lead stage by default.
-- Deals entering Lead immediately advance to Intel where Scout triggers.
-- Orgs that want manual qualification can set auto_skip: false via Pipeline Settings.

UPDATE crm_pipeline_stages
SET stage_config = '{"auto_skip":true,"auto_trigger":false,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[],"on_exit_validations":[],"stage_owner":{"label":"Sales Rep","type":"human"}}',
    updated_at = datetime('now', 'subsec')
WHERE stage_type = 'lead' AND stage_config IS NULL;

-- Also update Lead stages that already have config but no auto_skip
UPDATE crm_pipeline_stages
SET stage_config = json_set(stage_config, '$.auto_skip', json('true')),
    updated_at = datetime('now', 'subsec')
WHERE stage_type = 'lead' AND stage_config IS NOT NULL AND json_extract(stage_config, '$.auto_skip') IS NULL;
