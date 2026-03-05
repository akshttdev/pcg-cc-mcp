-- Add soft delete support for projects
-- This prevents topos sync from recreating deleted projects

ALTER TABLE projects ADD COLUMN deleted_at TEXT DEFAULT NULL;

-- Create index for efficient filtering of non-deleted projects
CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects(deleted_at);
