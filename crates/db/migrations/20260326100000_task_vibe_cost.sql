-- Add vibe_cost tracking to tasks so we can record LLM/agent spend per workflow step
ALTER TABLE tasks ADD COLUMN vibe_cost REAL DEFAULT 0.0;
ALTER TABLE tasks ADD COLUMN workflow_type TEXT; -- 'person_research' | 'company_research' | 'report_review' | 'lead_qualify' | 'action_item'
ALTER TABLE tasks ADD COLUMN entity_type TEXT;   -- 'person' | 'company'
ALTER TABLE tasks ADD COLUMN entity_id TEXT;     -- UUID of the entity this task relates to
