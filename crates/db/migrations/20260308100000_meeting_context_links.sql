-- Link meeting sessions to CRM entities (company, proposal, persons)
-- Enables meeting artifacts to be published to the knowledge graph

ALTER TABLE meeting_sessions ADD COLUMN company_id BLOB REFERENCES companies(id) ON DELETE SET NULL;
ALTER TABLE meeting_sessions ADD COLUMN proposal_id BLOB REFERENCES proposals(id) ON DELETE SET NULL;
ALTER TABLE meeting_sessions ADD COLUMN attendee_person_ids TEXT NOT NULL DEFAULT '[]';  -- JSON array of person UUIDs
ALTER TABLE meeting_sessions ADD COLUMN knowledge_source_id TEXT;  -- set once published to KG

CREATE INDEX IF NOT EXISTS idx_meeting_sessions_company ON meeting_sessions(company_id);
CREATE INDEX IF NOT EXISTS idx_meeting_sessions_proposal ON meeting_sessions(proposal_id);
