-- Add auto_watch_agent_ids column to agent_execution_config.
-- JSON array of agent IDs that should automatically watch tasks assigned to this agent.
-- Example: '["a0000000-0000-0000-0000-000000000002"]' on the Dev agent means
-- the QA agent auto-watches every task the Dev agent is assigned to.

ALTER TABLE agent_execution_config ADD COLUMN auto_watch_agent_ids TEXT;

-- Wire ORCHA Dev → ORCHA QA as default watcher
UPDATE agent_execution_config
SET auto_watch_agent_ids = '["a0000000-0000-0000-0000-000000000002"]',
    updated_at = datetime('now')
WHERE agent_id = 'a0000000-0000-0000-0000-000000000001';
