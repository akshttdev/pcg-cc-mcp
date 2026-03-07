-- Remove the project_id FK constraint from email_accounts and switch the UNIQUE key
-- to (owner_type, owner_id, provider, email_address) so polymorphic owners work.

PRAGMA foreign_keys = OFF;

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
