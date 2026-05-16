-- Migration: OAuth Token Manager (unified credential store)
--
-- Adds the canonical OAuth credential store (`integration_connections`) used by
-- every external provider (LinkedIn, OneDrive, Gmail, etc.). Existing per-provider
-- tables (social_accounts, email_accounts, quickbooks_accounts, dropbox_sources)
-- keep their provider-specific sync state but new connect flows write tokens here.
--
-- Tokens are stored as AES-256-GCM ciphertext (base64) when OAUTH_ENCRYPTION_KEY
-- is set. Plaintext storage is rejected at the service layer.

CREATE TABLE IF NOT EXISTS integration_connections (
    id                       TEXT PRIMARY KEY NOT NULL,
    organization_id          TEXT NOT NULL,
    project_id               TEXT,                          -- NULL = org-level connection
    provider                 TEXT NOT NULL,                  -- 'linkedin' | 'instagram' | 'gmail' | etc.
    provider_account_id      TEXT NOT NULL,                  -- platform's user/account ID
    display_name             TEXT,
    avatar_url               TEXT,
    -- Encrypted tokens (base64 of AES-GCM nonce||ciphertext||tag)
    access_token_ciphertext  TEXT,
    refresh_token_ciphertext TEXT,
    token_expires_at         TEXT,
    scopes                   TEXT,                           -- JSON array
    status                   TEXT NOT NULL DEFAULT 'active'  -- active | expired | error | disconnected
        CHECK (status IN ('active','expired','error','disconnected','pending_auth')),
    last_sync_at             TEXT,
    last_error               TEXT,
    metadata                 TEXT NOT NULL DEFAULT '{}',     -- JSON, provider-specific extras
    created_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at               TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(organization_id, provider, provider_account_id)
);

CREATE INDEX IF NOT EXISTS idx_integration_connections_org
    ON integration_connections(organization_id);
CREATE INDEX IF NOT EXISTS idx_integration_connections_provider
    ON integration_connections(provider);
CREATE INDEX IF NOT EXISTS idx_integration_connections_status
    ON integration_connections(status);
-- Partial index drives the 15-min refresh worker scan.
CREATE INDEX IF NOT EXISTS idx_integration_connections_expiring
    ON integration_connections(token_expires_at)
    WHERE status = 'active' AND token_expires_at IS NOT NULL;

-- ============================================================================
-- Pending OAuth state (CSRF protection for in-flight authorization flows)
-- ============================================================================
-- A short-lived row is created when the user hits /social/connect/:platform
-- and consumed exactly once at /social/callback/:platform. TTL is small (10 min)
-- so the table stays tiny.

CREATE TABLE IF NOT EXISTS oauth_pending_states (
    state_token     TEXT PRIMARY KEY NOT NULL,
    provider        TEXT NOT NULL,
    organization_id TEXT NOT NULL,
    project_id      TEXT,
    redirect_after  TEXT,                                    -- where to send the user once done
    metadata        TEXT NOT NULL DEFAULT '{}',              -- JSON: extra context (authored_as, etc.)
    created_at      TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    expires_at      TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_oauth_pending_states_expires
    ON oauth_pending_states(expires_at);

-- ============================================================================
-- Soft link: social_accounts.integration_connection_id
-- ============================================================================
-- New connect flows populate this so the row in social_accounts can defer to
-- integration_connections for live tokens. Existing rows keep NULL until
-- a future backfill migrates their plaintext tokens onto the unified store.

ALTER TABLE social_accounts ADD COLUMN integration_connection_id TEXT
    REFERENCES integration_connections(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_social_accounts_integration_connection
    ON social_accounts(integration_connection_id);
