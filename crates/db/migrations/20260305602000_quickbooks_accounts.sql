-- QuickBooks Online account connections (per-organization OAuth)
CREATE TABLE IF NOT EXISTS quickbooks_accounts (
    id              TEXT PRIMARY KEY NOT NULL,
    organization_id TEXT NOT NULL,
    -- QBO realm (company) identifier
    realm_id        TEXT NOT NULL,
    company_name    TEXT,
    -- OAuth 2.0 tokens
    access_token    TEXT,
    refresh_token   TEXT,
    token_expires_at TEXT,
    -- QBO environment
    environment     TEXT NOT NULL DEFAULT 'sandbox', -- 'sandbox' | 'production'
    -- Sync configuration
    sync_enabled    INTEGER NOT NULL DEFAULT 1,
    sync_frequency_minutes INTEGER NOT NULL DEFAULT 60,
    last_sync_at    TEXT,
    -- Sync scope flags
    sync_invoices   INTEGER NOT NULL DEFAULT 1,
    sync_customers  INTEGER NOT NULL DEFAULT 1,
    sync_payments   INTEGER NOT NULL DEFAULT 1,
    sync_expenses   INTEGER NOT NULL DEFAULT 1,
    sync_time_tracking INTEGER NOT NULL DEFAULT 0,
    -- Status
    status          TEXT NOT NULL DEFAULT 'active', -- active | inactive | expired | error | pending_auth
    last_error      TEXT,
    -- Metadata
    metadata        TEXT, -- JSON
    connected_by    TEXT, -- user ID who connected
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    -- One QBO realm per organization
    UNIQUE(organization_id, realm_id)
);

CREATE INDEX IF NOT EXISTS idx_qb_accounts_org ON quickbooks_accounts(organization_id);
CREATE INDEX IF NOT EXISTS idx_qb_accounts_status ON quickbooks_accounts(status);

-- Mapping table: links PCG entities to their QBO counterparts
CREATE TABLE IF NOT EXISTS quickbooks_entity_map (
    id              TEXT PRIMARY KEY NOT NULL,
    quickbooks_account_id TEXT NOT NULL REFERENCES quickbooks_accounts(id) ON DELETE CASCADE,
    -- PCG side
    pcg_entity_type TEXT NOT NULL, -- 'invoice' | 'person' | 'crm_deal' | 'token_usage' | 'operator_rate'
    pcg_entity_id   TEXT NOT NULL,
    -- QBO side
    qbo_entity_type TEXT NOT NULL, -- 'Invoice' | 'Customer' | 'Vendor' | 'Purchase' | 'TimeActivity' | 'Employee'
    qbo_entity_id   TEXT NOT NULL,
    qbo_sync_token  TEXT,         -- QBO optimistic locking token
    -- Sync state
    last_synced_at  TEXT,
    sync_direction  TEXT NOT NULL DEFAULT 'bidirectional', -- 'pcg_to_qbo' | 'qbo_to_pcg' | 'bidirectional'
    sync_status     TEXT NOT NULL DEFAULT 'synced', -- 'synced' | 'pending' | 'conflict' | 'error'
    last_error      TEXT,
    created_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(quickbooks_account_id, pcg_entity_type, pcg_entity_id),
    UNIQUE(quickbooks_account_id, qbo_entity_type, qbo_entity_id)
);

CREATE INDEX IF NOT EXISTS idx_qb_entity_map_account ON quickbooks_entity_map(quickbooks_account_id);
CREATE INDEX IF NOT EXISTS idx_qb_entity_map_pcg ON quickbooks_entity_map(pcg_entity_type, pcg_entity_id);
CREATE INDEX IF NOT EXISTS idx_qb_entity_map_qbo ON quickbooks_entity_map(qbo_entity_type, qbo_entity_id);
