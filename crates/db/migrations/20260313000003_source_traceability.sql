-- Add source tracking fields to CRM tables
ALTER TABLE crm_contacts ADD COLUMN source_data_source_id TEXT;
ALTER TABLE crm_contacts ADD COLUMN source_workflow_run_id TEXT;

ALTER TABLE crm_deals ADD COLUMN source_data_source_id TEXT;
ALTER TABLE crm_deals ADD COLUMN source_workflow_run_id TEXT;

ALTER TABLE tasks ADD COLUMN source_data_source_id TEXT;
ALTER TABLE tasks ADD COLUMN source_workflow_run_id TEXT;
