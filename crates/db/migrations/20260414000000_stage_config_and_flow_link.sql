-- Stage configuration: data-driven pipeline stage actions, gates, and agent assignment
ALTER TABLE crm_pipeline_stages ADD COLUMN stage_config TEXT;

-- Link agent flows to CRM deals (not just tasks)
ALTER TABLE agent_flows ADD COLUMN crm_deal_id TEXT REFERENCES crm_deals(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_agent_flows_crm_deal ON agent_flows(crm_deal_id);

-- Cancel window: agent auto-start can be cancelled before this deadline
ALTER TABLE agent_flows ADD COLUMN cancel_deadline TEXT;

-- Retry tracking for agent flow execution
ALTER TABLE agent_flows ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE agent_flows ADD COLUMN last_error TEXT;
