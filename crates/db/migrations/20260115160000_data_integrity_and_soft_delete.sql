-- Data Integrity and Soft Delete Migration
-- Fixes critical data integrity issues and adds soft delete infrastructure

-- ============================================================
-- PART 1: Fix project_members.project_id type (TEXT → BLOB)
-- ============================================================
-- SQLite doesn't support ALTER COLUMN, so we need to recreate the table

-- Step 1: Create new table with correct types
CREATE TABLE IF NOT EXISTS project_members_new (
    id BLOB PRIMARY KEY NOT NULL,
    project_id BLOB NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer' CHECK(role IN ('owner', 'admin', 'editor', 'viewer')),
    permissions TEXT NOT NULL DEFAULT '{}',
    granted_by BLOB,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(project_id, user_id)
);

-- Step 2: Migrate existing data (handle TEXT to BLOB conversion)
INSERT OR IGNORE INTO project_members_new (id, project_id, user_id, role, permissions, granted_by, granted_at)
SELECT
    id,
    -- Convert TEXT project_id to BLOB (assuming it's a hex UUID string)
    CASE
        WHEN typeof(project_id) = 'text' AND length(project_id) = 36
        THEN (SELECT id FROM projects WHERE hex(id) = replace(upper(project_id), '-', '') LIMIT 1)
        WHEN typeof(project_id) = 'blob' THEN project_id
        ELSE NULL
    END,
    user_id,
    role,
    permissions,
    granted_by,
    granted_at
FROM project_members
WHERE project_id IS NOT NULL;

-- Step 3: Drop old table and rename new one
DROP TABLE IF EXISTS project_members;
ALTER TABLE project_members_new RENAME TO project_members;

-- Step 4: Recreate indexes
CREATE INDEX IF NOT EXISTS idx_project_members_project_id ON project_members(project_id);
CREATE INDEX IF NOT EXISTS idx_project_members_user_id ON project_members(user_id);

-- ============================================================
-- PART 2: Add owner_id to projects table
-- ============================================================
ALTER TABLE projects ADD COLUMN owner_id BLOB REFERENCES users(id) ON DELETE SET NULL;

-- Create index for owner lookups
CREATE INDEX IF NOT EXISTS idx_projects_owner_id ON projects(owner_id);

-- ============================================================
-- PART 3: Add soft delete infrastructure
-- ============================================================

-- Add soft delete columns to projects
ALTER TABLE projects ADD COLUMN deleted_at TEXT;
ALTER TABLE projects ADD COLUMN deleted_by BLOB REFERENCES users(id) ON DELETE SET NULL;

-- Add soft delete columns to tasks
ALTER TABLE tasks ADD COLUMN deleted_at TEXT;
ALTER TABLE tasks ADD COLUMN deleted_by BLOB REFERENCES users(id) ON DELETE SET NULL;

-- Add soft delete columns to task_attempts
ALTER TABLE task_attempts ADD COLUMN deleted_at TEXT;
ALTER TABLE task_attempts ADD COLUMN deleted_by BLOB REFERENCES users(id) ON DELETE SET NULL;

-- Add soft delete to users (for GDPR compliance)
ALTER TABLE users ADD COLUMN deleted_at TEXT;
ALTER TABLE users ADD COLUMN deleted_by BLOB REFERENCES users(id) ON DELETE SET NULL;

-- Add soft delete to organizations
ALTER TABLE organizations ADD COLUMN deleted_at TEXT;
ALTER TABLE organizations ADD COLUMN deleted_by BLOB REFERENCES users(id) ON DELETE SET NULL;

-- Create indexes for soft delete queries (filter out deleted records efficiently)
CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects(deleted_at);
CREATE INDEX IF NOT EXISTS idx_tasks_deleted_at ON tasks(deleted_at);
CREATE INDEX IF NOT EXISTS idx_task_attempts_deleted_at ON task_attempts(deleted_at);
CREATE INDEX IF NOT EXISTS idx_users_deleted_at ON users(deleted_at);
CREATE INDEX IF NOT EXISTS idx_organizations_deleted_at ON organizations(deleted_at);

-- ============================================================
-- PART 4: Add missing foreign key indexes
-- ============================================================
CREATE INDEX IF NOT EXISTS idx_tasks_board_id ON tasks(board_id);
CREATE INDEX IF NOT EXISTS idx_tasks_pod_id ON tasks(pod_id);
CREATE INDEX IF NOT EXISTS idx_tasks_project_created ON tasks(project_id, created_at DESC);

-- ============================================================
-- PART 5: Fix tasks.created_by to be a proper reference
-- ============================================================
-- Note: We keep created_by as TEXT for backward compatibility with existing
-- 'system' and 'airtable_import' values, but add a created_by_user_id for proper FK
ALTER TABLE tasks ADD COLUMN created_by_user_id BLOB REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_tasks_created_by_user_id ON tasks(created_by_user_id);

-- ============================================================
-- PART 6: Add audit trail improvements
-- ============================================================
-- Add deletion audit log table
CREATE TABLE IF NOT EXISTS deletion_audit_log (
    id BLOB PRIMARY KEY NOT NULL,
    table_name TEXT NOT NULL,
    record_id BLOB NOT NULL,
    deleted_by BLOB REFERENCES users(id) ON DELETE SET NULL,
    deleted_at TEXT NOT NULL DEFAULT (datetime('now')),
    record_data TEXT, -- JSON snapshot of deleted record
    reason TEXT,
    FOREIGN KEY (deleted_by) REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_deletion_audit_log_table ON deletion_audit_log(table_name, deleted_at);
CREATE INDEX IF NOT EXISTS idx_deletion_audit_log_deleted_by ON deletion_audit_log(deleted_by);
