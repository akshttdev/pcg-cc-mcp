-- Migration: GitHub repository links
--
-- Maps an external GitHub repository onto a project so commits, PRs, and issue
-- webhook events can flow into deliverables / tasks / knowledge_sources.
--
-- Auth tokens live on integration_connections (provider='github'). This table
-- only carries the *identity* of the linked repo and its sync cursor.

CREATE TABLE IF NOT EXISTS github_repo_links (
    id                        TEXT PRIMARY KEY NOT NULL,
    organization_id           TEXT NOT NULL,
    project_id                TEXT NOT NULL,
    integration_connection_id TEXT,                            -- nullable: link can outlive the OAuth row
    github_repo_id            INTEGER NOT NULL,                -- GitHub's numeric repo id
    owner                     TEXT NOT NULL,
    repo_name                 TEXT NOT NULL,
    full_name                 TEXT NOT NULL,                   -- "owner/repo" denormalised
    default_branch            TEXT NOT NULL DEFAULT 'main',
    clone_url                 TEXT,
    ssh_url                   TEXT,
    private                   INTEGER NOT NULL DEFAULT 0,
    webhook_secret            TEXT,                            -- per-link HMAC secret for incoming hooks
    last_sync_at              TEXT,
    last_synced_commit_sha    TEXT,
    last_error                TEXT,
    metadata                  TEXT NOT NULL DEFAULT '{}',
    created_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at                TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    FOREIGN KEY (integration_connection_id) REFERENCES integration_connections(id) ON DELETE SET NULL,
    UNIQUE(project_id, github_repo_id)
);

CREATE INDEX IF NOT EXISTS idx_github_repo_links_project
    ON github_repo_links(project_id);
CREATE INDEX IF NOT EXISTS idx_github_repo_links_org
    ON github_repo_links(organization_id);
CREATE INDEX IF NOT EXISTS idx_github_repo_links_full_name
    ON github_repo_links(full_name);
CREATE INDEX IF NOT EXISTS idx_github_repo_links_repo_id
    ON github_repo_links(github_repo_id);

-- ============================================================================
-- Deliverable link: surface GitHub PRs / commit ranges on deliverables
-- ============================================================================
-- A nullable pointer that lets `deliverables` reference an upstream PR without
-- requiring a separate table. The PR number alone is ambiguous across repos,
-- so we also store the github_repo_link_id.

ALTER TABLE deliverables ADD COLUMN github_repo_link_id TEXT
    REFERENCES github_repo_links(id) ON DELETE SET NULL;
ALTER TABLE deliverables ADD COLUMN github_pr_number INTEGER;
ALTER TABLE deliverables ADD COLUMN github_pr_url TEXT;
ALTER TABLE deliverables ADD COLUMN github_pr_state TEXT;            -- 'open' | 'closed' | 'merged'

CREATE INDEX IF NOT EXISTS idx_deliverables_github_pr
    ON deliverables(github_repo_link_id, github_pr_number);
