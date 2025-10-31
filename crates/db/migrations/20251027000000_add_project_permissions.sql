-- Add project-level access control
-- Creates project_members table for scoped project access

CREATE TABLE IF NOT EXISTS project_members (
    id BLOB PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    user_id BLOB NOT NULL,
    role TEXT NOT NULL DEFAULT 'viewer',
    -- Roles: owner, admin, editor, viewer
    permissions TEXT DEFAULT '{}',
    -- JSON field for granular permissions
    granted_by BLOB,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users(id) ON DELETE SET NULL,
    UNIQUE(project_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_project_members_project_id ON project_members(project_id);
CREATE INDEX IF NOT EXISTS idx_project_members_user_id ON project_members(user_id);
CREATE INDEX IF NOT EXISTS idx_project_members_role ON project_members(role);

-- Add audit log for permission changes
CREATE TABLE IF NOT EXISTS permission_audit_log (
    id BLOB PRIMARY KEY NOT NULL,
    user_id BLOB NOT NULL,
    action TEXT NOT NULL,
    -- grant, revoke, modify
    resource_type TEXT NOT NULL,
    -- project, setting
    resource_id TEXT,
    details TEXT DEFAULT '{}',
    -- JSON field for additional context
    performed_by BLOB NOT NULL,
    performed_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (performed_by) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_audit_log_user_id ON permission_audit_log(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_resource ON permission_audit_log(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_performed_at ON permission_audit_log(performed_at);
