-- Add parent_project_id and sort_order to projects for nestable project hierarchy
-- Replaces the separate project_folders concept with parent-child project relationships
-- Max nesting: 3 levels deep

ALTER TABLE projects ADD COLUMN parent_project_id BLOB REFERENCES projects(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_projects_parent ON projects(parent_project_id);

-- Migrate existing folders that contain projects into container projects.
-- Only convert folders that actually have children (projects with folder_id pointing to them).
-- Reuse the folder ID as the new project ID so existing references stay valid.
INSERT INTO projects (id, name, git_repo_path, organization_id, client_id, parent_project_id, sort_order, created_at, updated_at)
SELECT
  pf.id,
  pf.name,
  '',
  pf.organization_id,
  pf.client_id,
  NULL,
  pf.sort_order,
  pf.created_at,
  pf.updated_at
FROM project_folders pf
WHERE pf.is_active = 1
  AND NOT EXISTS (SELECT 1 FROM projects WHERE id = pf.id)
  AND EXISTS (SELECT 1 FROM projects WHERE folder_id = pf.id AND deleted_at IS NULL);

-- Reparent projects that were in folders → set parent_project_id to the folder-turned-project
UPDATE projects
SET parent_project_id = folder_id,
    sort_order = 0
WHERE folder_id IS NOT NULL
  AND deleted_at IS NULL
  AND EXISTS (SELECT 1 FROM projects AS p2 WHERE p2.id = projects.folder_id);
