-- Link meeting sessions to CRM persons (JSON array of person UUID strings)
-- NOTE: linked_person_ids column already exists in live DB; ALTER skipped.
-- ALTER TABLE meeting_sessions ADD COLUMN linked_person_ids TEXT NOT NULL DEFAULT '[]';
SELECT 1; -- no-op placeholder
