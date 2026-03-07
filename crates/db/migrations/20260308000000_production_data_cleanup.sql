-- Production data cleanup: remove junk/orphan projects and empty folders.
-- This migration is idempotent — safe to run on both dev and production.

-- ============================================================================
-- 1. DELETE ORPHAN JUNK PROJECTS
--    These have organization_id IS NULL and were created by bots/testing.
-- ============================================================================

-- First, delete any tasks belonging to orphan junk projects
DELETE FROM tasks WHERE project_id IN (
    SELECT id FROM projects
    WHERE organization_id IS NULL
      AND deleted_at IS NULL
      AND (
          name LIKE 'Sirak Studios CRM Analysis%'
          OR name LIKE 'Analyze Sirak Studios CRM%'
          OR name LIKE 'PCG CC MCP Boards%'
          OR name LIKE 'Junglevere%'
          OR name LIKE 'HelloWorld%'
          OR name LIKE 'Hello World%'
          OR name LIKE 'Write Hello World%'
          OR name LIKE 'Unknown Caller%'
          OR name LIKE 'jungle''s Workspace%'
      )
);

-- Delete the orphan projects themselves
DELETE FROM projects
WHERE organization_id IS NULL
  AND deleted_at IS NULL
  AND (
      name LIKE 'Sirak Studios CRM Analysis%'
      OR name LIKE 'Analyze Sirak Studios CRM%'
      OR name LIKE 'PCG CC MCP Boards%'
      OR name LIKE 'Junglevere%'
      OR name LIKE 'HelloWorld%'
      OR name LIKE 'Hello World%'
      OR name LIKE 'Write Hello World%'
      OR name LIKE 'Unknown Caller%'
      OR name LIKE 'jungle''s Workspace%'
  );

-- Also delete duplicate Bulgari Perfume projects (org_id IS NULL, the real one has org_id set)
DELETE FROM projects
WHERE organization_id IS NULL
  AND deleted_at IS NULL
  AND name = 'Bulgari Perfume';

-- ============================================================================
-- 2. ASSIGN REMAINING UNLINKED PROJECTS TO POWERCLUB GLOBAL
--    Projects like "Operations", "Travers's Workspace" that have no org.
-- ============================================================================

UPDATE projects
SET organization_id = (SELECT id FROM organizations WHERE slug = 'powerclub-global')
WHERE organization_id IS NULL
  AND deleted_at IS NULL
  AND EXISTS (SELECT 1 FROM organizations WHERE slug = 'powerclub-global');

-- ============================================================================
-- 3. DELETE EMPTY FOLDERS
--    All folders on production are empty (no projects have folder_id set).
--    Prevents the parent_project_id migration from creating ghost containers.
-- ============================================================================

DELETE FROM project_folders
WHERE id NOT IN (SELECT DISTINCT folder_id FROM projects WHERE folder_id IS NOT NULL);
