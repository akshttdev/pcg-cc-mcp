-- Update Proposal stage to use sequential agent queue: Astra Pass 2 (deep_research) then Cash (proposal).
-- The stage_transition processor schedules only the first agent and stores chain_actions for the rest.
-- After Astra Pass 2 completes, the executor chains Cash automatically.

UPDATE crm_pipeline_stages
SET stage_config = '{"assigned_agent":"astra","auto_trigger":true,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"trigger_agent","agent":"astra","flow_type":"deep_research"},{"type":"trigger_agent","agent":"cash","flow_type":"proposal"},{"type":"create_review_task","description":"Review and approve proposal (Cash) before moving to Polish"}],"on_exit_validations":[],"stage_owner":{"label":"Cash","type":"agent"}}',
    updated_at = datetime('now', 'subsec')
WHERE stage_type = 'proposal';
