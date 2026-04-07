-- Migration: Add project status tracking fields
-- Required by command center: waiting_on_client flag and project_status enum

ALTER TABLE projects ADD COLUMN waiting_on_client INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN project_status TEXT NOT NULL DEFAULT 'active';

CREATE INDEX IF NOT EXISTS idx_projects_status
    ON projects(project_status) WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_projects_waiting_on_client
    ON projects(waiting_on_client) WHERE waiting_on_client = 1 AND deleted_at IS NULL;
