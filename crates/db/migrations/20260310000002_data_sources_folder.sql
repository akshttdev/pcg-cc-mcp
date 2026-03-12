-- Add folder path to data_sources for hierarchical organization
ALTER TABLE data_sources ADD COLUMN folder TEXT NOT NULL DEFAULT 'Unfiled';

-- Auto-populate folders for existing records based on data_type / source metadata
UPDATE data_sources SET folder = 'Meetings'
  WHERE json_extract(metadata, '$.source') = 'nora_meet';

UPDATE data_sources SET folder = 'Meetings/Discord'
  WHERE data_type = 'conversation'
    AND json_extract(metadata, '$.source') IS NULL
    AND title LIKE '%Discord%';

UPDATE data_sources SET folder = 'Documents'
  WHERE source_type = 'file' AND folder = 'Unfiled';

UPDATE data_sources SET folder = 'Conversations'
  WHERE data_type = 'conversation' AND folder = 'Unfiled';
