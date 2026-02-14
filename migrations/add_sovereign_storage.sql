-- Migration: Add Sovereign Storage Architecture
-- Date: 2026-02-09
-- Description: Adds tables for storage providers, device registry, and collaboration

-- ============================================================================
-- DEVICE REGISTRY
-- ============================================================================
CREATE TABLE IF NOT EXISTS device_registry (
    id TEXT PRIMARY KEY,
    owner_id BLOB REFERENCES users(id) ON DELETE CASCADE,
    device_name TEXT NOT NULL,
    device_type TEXT NOT NULL CHECK (device_type IN ('always_on', 'mobile', 'storage_provider')),
    apn_node_id TEXT NOT NULL,
    public_key TEXT NOT NULL,

    -- Status
    is_online BOOLEAN NOT NULL DEFAULT FALSE,
    last_seen TIMESTAMP,
    last_heartbeat TIMESTAMP,

    -- Capabilities
    storage_capacity_gb INTEGER DEFAULT 0,
    serves_data BOOLEAN DEFAULT FALSE,
    accepts_storage_contracts BOOLEAN DEFAULT FALSE,

    -- Metadata
    ip_address TEXT,
    location TEXT,
    hardware_info TEXT, -- JSON

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_device_registry_owner ON device_registry(owner_id);
CREATE INDEX idx_device_registry_online ON device_registry(is_online);
CREATE INDEX idx_device_registry_type ON device_registry(device_type);
CREATE INDEX idx_device_registry_apn ON device_registry(apn_node_id);

-- ============================================================================
-- STORAGE CONTRACTS
-- ============================================================================
CREATE TABLE IF NOT EXISTS storage_contracts (
    id TEXT PRIMARY KEY,

    -- Parties
    client_id BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider_device_id TEXT NOT NULL REFERENCES device_registry(id) ON DELETE CASCADE,

    -- Terms
    storage_size_gb INTEGER NOT NULL,
    monthly_rate_vibe INTEGER NOT NULL,
    transfer_rate_vibe_per_gb REAL NOT NULL DEFAULT 0.1,
    uptime_requirement_percent INTEGER NOT NULL DEFAULT 99,

    -- Contract period
    start_date TIMESTAMP NOT NULL,
    end_date TIMESTAMP,
    auto_renew BOOLEAN DEFAULT TRUE,

    -- Escrow
    escrow_amount INTEGER NOT NULL DEFAULT 0,
    escrow_address TEXT, -- Aptos wallet address for escrow

    -- Status
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('pending', 'active', 'paused', 'cancelled', 'expired')),

    -- Metrics
    actual_storage_used_gb REAL DEFAULT 0,
    total_data_transferred_gb REAL DEFAULT 0,
    uptime_percent REAL DEFAULT 100,

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_storage_contracts_client ON storage_contracts(client_id);
CREATE INDEX idx_storage_contracts_provider ON storage_contracts(provider_device_id);
CREATE INDEX idx_storage_contracts_status ON storage_contracts(status);

-- ============================================================================
-- STORAGE METRICS (for billing)
-- ============================================================================
CREATE TABLE IF NOT EXISTS storage_metrics (
    id TEXT PRIMARY KEY,
    contract_id TEXT NOT NULL REFERENCES storage_contracts(id) ON DELETE CASCADE,

    -- Metrics period
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,

    -- Storage
    avg_storage_gb REAL NOT NULL,
    max_storage_gb REAL NOT NULL,

    -- Transfers
    data_uploaded_gb REAL DEFAULT 0,
    data_downloaded_gb REAL DEFAULT 0,

    -- Uptime
    total_checks INTEGER NOT NULL,
    successful_checks INTEGER NOT NULL,
    uptime_percent REAL,

    -- Billing
    storage_charge_vibe INTEGER DEFAULT 0,
    transfer_charge_vibe INTEGER DEFAULT 0,
    uptime_multiplier REAL DEFAULT 1.0,
    total_charge_vibe INTEGER DEFAULT 0,
    paid BOOLEAN DEFAULT FALSE,

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_storage_metrics_contract ON storage_metrics(contract_id);
CREATE INDEX idx_storage_metrics_period ON storage_metrics(period_start, period_end);

-- ============================================================================
-- PROJECT COLLABORATORS (Enhanced)
-- ============================================================================
-- Update existing project_members table or create new one
CREATE TABLE IF NOT EXISTS project_collaborators (
    id TEXT PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    user_id BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Role & Permissions
    role TEXT NOT NULL DEFAULT 'viewer'
        CHECK (role IN ('owner', 'admin', 'editor', 'viewer')),

    -- Granular permissions
    can_read BOOLEAN DEFAULT TRUE,
    can_write BOOLEAN DEFAULT FALSE,
    can_delete BOOLEAN DEFAULT FALSE,
    can_invite BOOLEAN DEFAULT FALSE,
    can_manage BOOLEAN DEFAULT FALSE,

    -- Collaboration metadata
    invited_by BLOB REFERENCES users(id),
    invited_at TIMESTAMP,
    accepted_at TIMESTAMP,
    status TEXT DEFAULT 'active' CHECK (status IN ('pending', 'active', 'revoked')),

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),

    UNIQUE(project_id, user_id)
);

CREATE INDEX idx_project_collaborators_project ON project_collaborators(project_id);
CREATE INDEX idx_project_collaborators_user ON project_collaborators(user_id);
CREATE INDEX idx_project_collaborators_role ON project_collaborators(role);

-- ============================================================================
-- DATA REPLICATION STATE
-- ============================================================================
CREATE TABLE IF NOT EXISTS data_replication_state (
    id TEXT PRIMARY KEY,

    -- Source and destination
    source_device_id TEXT NOT NULL REFERENCES device_registry(id) ON DELETE CASCADE,
    destination_device_id TEXT NOT NULL REFERENCES device_registry(id) ON DELETE CASCADE,

    -- What data
    data_owner_id BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    data_type TEXT NOT NULL, -- 'full_db', 'projects', 'tasks', etc.

    -- Replication status
    last_sync_timestamp TIMESTAMP,
    last_sync_version INTEGER DEFAULT 0,
    current_version INTEGER DEFAULT 0,
    sync_status TEXT DEFAULT 'in_sync'
        CHECK (sync_status IN ('in_sync', 'syncing', 'out_of_sync', 'error')),

    -- Checksums for verification
    source_checksum TEXT,
    destination_checksum TEXT,

    -- Sync configuration
    sync_enabled BOOLEAN DEFAULT TRUE,
    sync_interval_minutes INTEGER DEFAULT 5,
    auto_sync BOOLEAN DEFAULT TRUE,

    -- Error tracking
    last_error TEXT,
    error_count INTEGER DEFAULT 0,

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),

    UNIQUE(source_device_id, destination_device_id, data_owner_id)
);

CREATE INDEX idx_replication_source ON data_replication_state(source_device_id);
CREATE INDEX idx_replication_destination ON data_replication_state(destination_device_id);
CREATE INDEX idx_replication_owner ON data_replication_state(data_owner_id);
CREATE INDEX idx_replication_status ON data_replication_state(sync_status);

-- ============================================================================
-- CONTENT INDEX (Metadata for federated queries)
-- ============================================================================
CREATE TABLE IF NOT EXISTS content_index (
    id TEXT PRIMARY KEY,

    -- Content identification
    content_type TEXT NOT NULL, -- 'project', 'task', 'file', etc.
    content_id TEXT NOT NULL,
    owner_id BLOB NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Location
    primary_device_id TEXT NOT NULL REFERENCES device_registry(id),
    replica_device_ids TEXT, -- JSON array of device IDs that have replicas

    -- Availability
    is_online BOOLEAN DEFAULT FALSE,
    last_available TIMESTAMP,

    -- Metadata (searchable)
    title TEXT,
    description TEXT,
    tags TEXT, -- JSON array
    metadata JSON,

    -- Access control
    visibility TEXT DEFAULT 'private' CHECK (visibility IN ('private', 'shared', 'public')),

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),
    updated_at TIMESTAMP NOT NULL DEFAULT (datetime('now')),

    UNIQUE(content_type, content_id)
);

CREATE INDEX idx_content_index_owner ON content_index(owner_id);
CREATE INDEX idx_content_index_device ON content_index(primary_device_id);
CREATE INDEX idx_content_index_online ON content_index(is_online);
CREATE INDEX idx_content_index_type ON content_index(content_type);

-- ============================================================================
-- STORAGE PROVIDER EARNINGS
-- ============================================================================
CREATE TABLE IF NOT EXISTS storage_provider_earnings (
    id TEXT PRIMARY KEY,

    -- Provider
    provider_device_id TEXT NOT NULL REFERENCES device_registry(id) ON DELETE CASCADE,
    contract_id TEXT NOT NULL REFERENCES storage_contracts(id) ON DELETE CASCADE,

    -- Earnings period
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,

    -- Breakdown
    storage_earnings_vibe INTEGER DEFAULT 0,
    transfer_earnings_vibe INTEGER DEFAULT 0,
    uptime_bonus_vibe INTEGER DEFAULT 0,
    total_earnings_vibe INTEGER DEFAULT 0,

    -- Payment status
    paid BOOLEAN DEFAULT FALSE,
    payment_tx_hash TEXT, -- Aptos transaction hash
    paid_at TIMESTAMP,

    created_at TIMESTAMP NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_provider_earnings_device ON storage_provider_earnings(provider_device_id);
CREATE INDEX idx_provider_earnings_contract ON storage_provider_earnings(contract_id);
CREATE INDEX idx_provider_earnings_period ON storage_provider_earnings(period_start, period_end);

-- ============================================================================
-- Add columns to existing projects table
-- ============================================================================
-- Track which device serves this project
ALTER TABLE projects ADD COLUMN primary_device_id TEXT REFERENCES device_registry(id);
ALTER TABLE projects ADD COLUMN is_shared BOOLEAN DEFAULT FALSE;
ALTER TABLE projects ADD COLUMN collaboration_enabled BOOLEAN DEFAULT FALSE;

-- ============================================================================
-- TRIGGERS for updated_at
-- ============================================================================
CREATE TRIGGER IF NOT EXISTS update_device_registry_timestamp
AFTER UPDATE ON device_registry
BEGIN
    UPDATE device_registry SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_storage_contracts_timestamp
AFTER UPDATE ON storage_contracts
BEGIN
    UPDATE storage_contracts SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_project_collaborators_timestamp
AFTER UPDATE ON project_collaborators
BEGIN
    UPDATE project_collaborators SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_replication_state_timestamp
AFTER UPDATE ON data_replication_state
BEGIN
    UPDATE data_replication_state SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_content_index_timestamp
AFTER UPDATE ON content_index
BEGIN
    UPDATE content_index SET updated_at = datetime('now') WHERE id = NEW.id;
END;
