-- Add vibe and online presence quality fields to persons
ALTER TABLE persons ADD COLUMN vibe TEXT;
ALTER TABLE persons ADD COLUMN online_presence_quality TEXT;
