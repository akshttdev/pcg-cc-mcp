-- Add stage_type and stage_config to Conferences pipeline stages.
-- Researching stage gets Scout agent with auto-trigger.

UPDATE crm_pipeline_stages
SET stage_type = 'researching',
    stage_config = '{"assigned_agent":"scout","auto_trigger":true,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"trigger_agent","agent":"scout","flow_type":"research"},{"type":"create_review_task","description":"Review conference research findings"}],"on_exit_validations":[],"stage_owner":{"label":"Scout","type":"agent"}}',
    updated_at = datetime('now', 'subsec')
WHERE name = 'Researching'
  AND pipeline_id IN (SELECT id FROM crm_pipelines WHERE pipeline_type = 'conferences')
  AND stage_config IS NULL;
