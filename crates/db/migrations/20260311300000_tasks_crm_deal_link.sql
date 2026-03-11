ALTER TABLE tasks ADD COLUMN crm_deal_id BLOB REFERENCES crm_deals(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_crm_deal ON tasks(crm_deal_id);
