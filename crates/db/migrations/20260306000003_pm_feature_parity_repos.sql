-- Phase 3: Repo abstraction - separate git config from project metadata

CREATE TABLE IF NOT EXISTS repos (
    id                      BLOB PRIMARY KEY NOT NULL,
    path                    TEXT NOT NULL UNIQUE,
    name                    TEXT NOT NULL,
    display_name            TEXT NOT NULL,
    setup_script            TEXT,
    cleanup_script          TEXT,
    archive_script          TEXT,
    copy_files              TEXT,
    parallel_setup_script   INTEGER NOT NULL DEFAULT 0,
    dev_server_script       TEXT,
    default_target_branch   TEXT,
    default_working_dir     TEXT,
    created_at              TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at              TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_repos_path ON repos(path);

-- Project-repo join table (a project can reference multiple repos)
CREATE TABLE IF NOT EXISTS project_repos (
    id          BLOB PRIMARY KEY NOT NULL,
    project_id  BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    repo_id     BLOB NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    is_default  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(project_id, repo_id)
);

CREATE INDEX IF NOT EXISTS idx_project_repos_project_id ON project_repos(project_id);
CREATE INDEX IF NOT EXISTS idx_project_repos_repo_id ON project_repos(repo_id);

-- Attempt-repo join table for multi-repo workspaces
CREATE TABLE IF NOT EXISTS attempt_repos (
    id              BLOB PRIMARY KEY NOT NULL,
    task_attempt_id BLOB NOT NULL REFERENCES task_attempts(id) ON DELETE CASCADE,
    repo_id         BLOB NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
    target_branch   TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(task_attempt_id, repo_id)
);

CREATE INDEX IF NOT EXISTS idx_attempt_repos_task_attempt_id ON attempt_repos(task_attempt_id);
CREATE INDEX IF NOT EXISTS idx_attempt_repos_repo_id ON attempt_repos(repo_id);

-- Migrate existing projects with git_repo_path into repos table
INSERT OR IGNORE INTO repos (id, path, name, display_name, setup_script, cleanup_script, copy_files, dev_server_script)
SELECT
    randomblob(16),
    p.git_repo_path,
    REPLACE(p.git_repo_path, RTRIM(p.git_repo_path, REPLACE(p.git_repo_path, '/', '')), ''),
    p.name,
    p.setup_script,
    p.cleanup_script,
    p.copy_files,
    p.dev_script
FROM projects p
WHERE p.git_repo_path IS NOT NULL AND p.git_repo_path != '';

-- Link migrated repos back to their projects
INSERT OR IGNORE INTO project_repos (id, project_id, repo_id, is_default)
SELECT
    randomblob(16),
    p.id,
    r.id,
    1
FROM projects p
JOIN repos r ON r.path = p.git_repo_path
WHERE p.git_repo_path IS NOT NULL AND p.git_repo_path != '';
