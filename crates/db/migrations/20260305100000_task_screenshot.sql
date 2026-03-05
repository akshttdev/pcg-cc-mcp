-- Add screenshot field to tasks for bug reports
-- Stores base64 encoded image data

ALTER TABLE tasks ADD COLUMN screenshot TEXT DEFAULT NULL;
