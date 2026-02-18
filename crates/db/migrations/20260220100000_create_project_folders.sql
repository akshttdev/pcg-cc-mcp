-- Project folders for grouping related projects in the sidebar

CREATE TABLE IF NOT EXISTS project_folders (
    id                BLOB PRIMARY KEY NOT NULL,
    organization_id   BLOB NOT NULL,
    client_id         BLOB,
    name              TEXT NOT NULL,
    sort_order        INTEGER NOT NULL DEFAULT 0,
    is_active         INTEGER NOT NULL DEFAULT 1,
    created_at        TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at        TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
    FOREIGN KEY (client_id) REFERENCES clients(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_project_folders_org ON project_folders(organization_id);
CREATE INDEX IF NOT EXISTS idx_project_folders_client ON project_folders(client_id);

-- Add folder_id to projects table
ALTER TABLE projects ADD COLUMN folder_id BLOB REFERENCES project_folders(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_projects_folder_id ON projects(folder_id);
