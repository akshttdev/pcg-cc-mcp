-- Link meeting sessions to CRM persons (JSON array of person UUID strings)
ALTER TABLE meeting_sessions ADD COLUMN linked_person_ids TEXT NOT NULL DEFAULT '[]';
