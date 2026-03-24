-- Rename "Invoice" stage to "Present & Invoice" to capture both
-- the deck presentation step and invoice/payment confirmation.
-- Presentation tracking is stored in custom_fields JSON (no new columns needed).

UPDATE crm_pipeline_stages
SET name = 'Present & Invoice',
    description = 'Present deck to client on live call, then send invoice and confirm payment',
    updated_at = datetime('now', 'subsec')
WHERE stage_type = 'present' AND name IN ('Invoice', 'Proposal Meeting');

-- Update stage_config with presentation + invoice exit validations
UPDATE crm_pipeline_stages
SET stage_config = '{"auto_trigger":false,"cancel_window_secs":30,"required_fields":["deck_url"],"approval_gate":false,"on_enter_actions":[{"type":"create_review_task","description":"Present deck to client, send invoice, and confirm payment"}],"on_exit_validations":[{"type":"require_field","field":"deck_url","message":"Deck URL required before advancing"}],"stage_owner":{"label":"Account Manager","type":"human"}}',
    updated_at = datetime('now', 'subsec')
WHERE stage_type = 'present';
