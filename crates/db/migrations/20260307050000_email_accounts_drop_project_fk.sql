-- Remove the project_id FK constraint from email_accounts and switch the UNIQUE key
-- to (owner_type, owner_id, provider, email_address) so polymorphic owners work.

PRAGMA foreign_keys = OFF;

-- Temporarily drop triggers that reference ep.task_id (column may not exist yet)
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_artifact;
DROP TRIGGER IF EXISTS trg_knowledge_auto_register_context_injection;

CREATE TABLE email_accounts_new (
    id BLOB PRIMARY KEY,
    project_id BLOB,                          -- nullable, no FK
    provider TEXT NOT NULL,
    account_type TEXT NOT NULL DEFAULT 'primary',
    email_address TEXT NOT NULL,
    display_name TEXT,
    avatar_url TEXT,
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TEXT,
    imap_host TEXT,
    imap_port INTEGER,
    smtp_host TEXT,
    smtp_port INTEGER,
    use_ssl INTEGER NOT NULL DEFAULT 1,
    granted_scopes TEXT,
    storage_used_bytes INTEGER,
    storage_total_bytes INTEGER,
    unread_count INTEGER NOT NULL DEFAULT 0,
    metadata TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    last_sync_at TEXT,
    last_error TEXT,
    sync_enabled INTEGER NOT NULL DEFAULT 1,
    sync_frequency_minutes INTEGER NOT NULL DEFAULT 15,
    auto_reply_enabled INTEGER NOT NULL DEFAULT 0,
    signature TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    owner_type TEXT NOT NULL DEFAULT 'project',
    owner_id TEXT NOT NULL DEFAULT '',
    UNIQUE(owner_type, owner_id, provider, email_address)
);

INSERT INTO email_accounts_new SELECT * FROM email_accounts;

DROP TABLE email_accounts;
ALTER TABLE email_accounts_new RENAME TO email_accounts;

CREATE INDEX IF NOT EXISTS idx_email_accounts_project ON email_accounts(project_id);
CREATE INDEX IF NOT EXISTS idx_email_accounts_owner ON email_accounts(owner_type, owner_id);

PRAGMA foreign_keys = ON;

-- Re-create both triggers
CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_artifact
AFTER INSERT ON execution_artifacts
WHEN NEW.artifact_type IN (
    'research_report', 'strategy_document', 'content_calendar',
    'competitor_analysis', 'content_draft', 'visual_brief',
    'verification_report', 'aggregated_research'
)
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title,
        source_summary, coverage_score, auto_registered
    )
    SELECT
        randomblob(16),
        t.project_id,
        'artifact',
        hex(NEW.id),
        COALESCE(NEW.title, 'Artifact'),
        SUBSTR(COALESCE(NEW.content, ''), 1, 500),
        CASE NEW.artifact_type
            WHEN 'research_report' THEN 0.6
            WHEN 'strategy_document' THEN 0.7
            WHEN 'competitor_analysis' THEN 0.6
            WHEN 'aggregated_research' THEN 0.7
            WHEN 'content_draft' THEN 0.4
            WHEN 'visual_brief' THEN 0.4
            WHEN 'content_calendar' THEN 0.5
            WHEN 'verification_report' THEN 0.5
            ELSE 0.3
        END,
        1
    FROM execution_processes ep
    JOIN tasks t ON t.id = ep.task_id
    WHERE ep.id = NEW.execution_process_id
      AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;

CREATE TRIGGER IF NOT EXISTS trg_knowledge_auto_register_context_injection
AFTER INSERT ON context_injections
WHEN NEW.injection_type IN ('note', 'correction', 'directive', 'answer')
BEGIN
    INSERT INTO project_knowledge_sources (
        id, project_id, source_type, source_id, source_title,
        source_summary, coverage_score, auto_registered
    )
    SELECT
        randomblob(16),
        t.project_id,
        'context_injection',
        hex(NEW.id),
        COALESCE(
            NEW.injector_name || ': ' || NEW.injection_type,
            'Context: ' || NEW.injection_type
        ),
        SUBSTR(NEW.content, 1, 500),
        CASE NEW.injection_type
            WHEN 'correction' THEN 0.8
            WHEN 'directive' THEN 0.7
            WHEN 'answer' THEN 0.6
            WHEN 'note' THEN 0.5
            ELSE 0.4
        END,
        1
    FROM execution_processes ep
    JOIN tasks t ON t.id = ep.task_id
    WHERE ep.id = NEW.execution_process_id
      AND t.project_id IS NOT NULL
    ON CONFLICT(project_id, source_type, source_id)
    DO UPDATE SET
        source_title = COALESCE(excluded.source_title, source_title),
        source_summary = COALESCE(excluded.source_summary, source_summary),
        coverage_score = MAX(coverage_score, excluded.coverage_score),
        updated_at = datetime('now', 'subsec');
END;
