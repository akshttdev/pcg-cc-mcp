-- Seed default stage_config for existing Dealflow sales pipeline stages.
-- These convert the hardcoded automations in stage_transition.rs into data.
-- The processor checks stage_config first; NULL = hardcoded fallback still works.

-- Intel stage → Scout research
UPDATE crm_pipeline_stages SET stage_config = '{"assigned_agent":"scout","auto_trigger":true,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"trigger_agent","agent":"scout","flow_type":"research"},{"type":"create_review_task","description":"Review Phase I intelligence (Scout): person profile & company overview"}],"on_exit_validations":[{"type":"require_field","field":"description","message":"Operator context required before advancing"},{"type":"require_intel","entity":"person","status":"done"},{"type":"require_intel","entity":"company","status":"done"}],"stage_owner":{"label":"Scout","type":"agent"}}'
WHERE stage_type = 'intel' AND stage_config IS NULL;

-- Business Analysis stage → Astra
UPDATE crm_pipeline_stages SET stage_config = '{"assigned_agent":"astra","auto_trigger":true,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"trigger_agent","agent":"astra","flow_type":"business_analysis"},{"type":"create_review_task","description":"Review business report (Astra): pain points, opportunities, recommended services"}],"on_exit_validations":[],"stage_owner":{"label":"Astra","type":"agent"}}'
WHERE stage_type = 'business_analysis' AND stage_config IS NULL;

-- Discovery stage → human-driven
UPDATE crm_pipeline_stages SET stage_config = '{"auto_trigger":false,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"create_review_task","description":"Review discovery transcript and confirm proposal readiness"}],"on_exit_validations":[],"stage_owner":{"label":"Account Manager + Nora","type":"team"}}'
WHERE stage_type = 'discovery' AND stage_config IS NULL;

-- Proposal stage → Cash
UPDATE crm_pipeline_stages SET stage_config = '{"assigned_agent":"cash","auto_trigger":true,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"trigger_agent","agent":"cash","flow_type":"proposal"},{"type":"create_review_task","description":"Review and approve proposal (Cash) before moving to Polish"}],"on_exit_validations":[],"stage_owner":{"label":"Cash","type":"agent"}}'
WHERE stage_type = 'proposal' AND stage_config IS NULL;

-- Polish stage → Lux
UPDATE crm_pipeline_stages SET stage_config = '{"assigned_agent":"lux","auto_trigger":true,"cancel_window_secs":30,"required_fields":["proposal_text"],"approval_gate":false,"on_enter_actions":[{"type":"trigger_agent","agent":"lux","flow_type":"deck"},{"type":"create_review_task","description":"Review and approve deck (Lux) before presenting to client"}],"on_exit_validations":[],"stage_owner":{"label":"Lux","type":"agent"}}'
WHERE stage_type = 'polish' AND stage_config IS NULL;

-- Present stage → human
UPDATE crm_pipeline_stages SET stage_config = '{"auto_trigger":false,"cancel_window_secs":30,"required_fields":["deck_url"],"approval_gate":false,"on_enter_actions":[{"type":"create_review_task","description":"Confirm invoice sent and await payment confirmation"}],"on_exit_validations":[],"stage_owner":{"label":"Account Manager","type":"human"}}'
WHERE stage_type = 'present' AND stage_config IS NULL;

-- Follow Up stage → human + Nora
UPDATE crm_pipeline_stages SET stage_config = '{"auto_trigger":false,"cancel_window_secs":30,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"create_review_task","description":"Update follow-up status — won, lost, or still in discussion"}],"on_exit_validations":[],"stage_owner":{"label":"Nora + AM","type":"team"}}'
WHERE stage_type = 'follow_up' AND stage_config IS NULL;

-- Won stage
UPDATE crm_pipeline_stages SET stage_config = '{"auto_trigger":false,"cancel_window_secs":0,"required_fields":[],"approval_gate":false,"on_enter_actions":[{"type":"create_delivery_deal"}],"on_exit_validations":[],"stage_owner":{"label":"Team","type":"team"}}'
WHERE stage_type = 'won' AND stage_config IS NULL;

-- Lost stage
UPDATE crm_pipeline_stages SET stage_config = '{"auto_trigger":false,"cancel_window_secs":0,"required_fields":[],"approval_gate":false,"on_enter_actions":[],"on_exit_validations":[],"stage_owner":{"label":"Account Manager","type":"human"}}'
WHERE stage_type = 'lost' AND stage_config IS NULL;

-- Delivery pipeline stages (basic config, no agent triggers)
UPDATE crm_pipeline_stages SET stage_config = '{"auto_trigger":false,"required_fields":[],"approval_gate":false,"on_enter_actions":[],"on_exit_validations":[],"stage_owner":{"label":"PM","type":"human"}}'
WHERE stage_type IN ('onboarding', 'brand_guide', 'online_presence', 'social_stack', 'monthly_retainer', 'active_retainer', 'completed') AND stage_config IS NULL;
