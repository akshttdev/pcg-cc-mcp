-- Add organization_id to existing tables for multi-tenancy
-- This links existing resources to organizations

ALTER TABLE projects ADD COLUMN organization_id UUID;
ALTER TABLE projects ADD CONSTRAINT fk_projects_organization 
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;
CREATE INDEX idx_projects_organization_id ON projects(organization_id);

ALTER TABLE tasks ADD COLUMN organization_id UUID;
ALTER TABLE tasks ADD CONSTRAINT fk_tasks_organization 
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;
CREATE INDEX idx_tasks_organization_id ON tasks(organization_id);

ALTER TABLE task_templates ADD COLUMN organization_id UUID;
ALTER TABLE task_templates ADD CONSTRAINT fk_task_templates_organization 
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE;
CREATE INDEX idx_task_templates_organization_id ON task_templates(organization_id);

-- Add created_by and updated_by tracking
ALTER TABLE projects ADD COLUMN created_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE projects ADD COLUMN updated_by UUID REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE tasks ADD COLUMN created_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN updated_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN assigned_to UUID REFERENCES users(id) ON DELETE SET NULL;

ALTER TABLE project_pods ADD COLUMN created_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE project_boards ADD COLUMN created_by UUID REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE project_assets ADD COLUMN uploaded_by_user_id UUID REFERENCES users(id) ON DELETE SET NULL;

-- Comments should be linked to users
ALTER TABLE task_comments ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE CASCADE;
CREATE INDEX idx_task_comments_user_id ON task_comments(user_id);

-- Activity logs linked to users
ALTER TABLE activity_logs ADD COLUMN user_id UUID REFERENCES users(id) ON DELETE SET NULL;
CREATE INDEX idx_activity_logs_user_id ON activity_logs(user_id);
