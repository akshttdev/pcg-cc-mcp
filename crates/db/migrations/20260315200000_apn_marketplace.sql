-- APN Marketplace: Service listings, consumer subscriptions, gateway request log
-- Supports the decentralized marketplace where providers register services
-- and consumers (like Jungleverse) purchase access via VIBE tokens.

-- Service listings: providers (APN nodes or orgs with infrastructure) register services
CREATE TABLE IF NOT EXISTS marketplace_listings (
    id BLOB NOT NULL PRIMARY KEY,

    -- Provider identity: can be an APN node OR an org with its own infra
    provider_type TEXT NOT NULL DEFAULT 'apn_node',  -- 'apn_node' | 'org_infra'
    provider_node_id TEXT,                           -- APN node_id (for apn_node type)
    provider_org_id BLOB REFERENCES organizations(id), -- PCG org (for org_infra type)
    provider_wallet TEXT NOT NULL,                   -- wallet address for VIBE rewards

    -- Service definition
    service_type TEXT NOT NULL,      -- 'web-hosting' | 'compute' | 'data-api' | 'proxy' | 'storage' | 'custom'
    service_name TEXT NOT NULL,
    service_description TEXT,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON array of searchable tags

    -- Pricing
    pricing_model TEXT NOT NULL DEFAULT 'per-request', -- 'per-request' | 'per-mb' | 'per-hour' | 'flat'
    price_vibe REAL NOT NULL DEFAULT 1.0,              -- VIBE per unit
    min_vibe REAL NOT NULL DEFAULT 0.0,                -- minimum charge per request

    -- Capacity & connectivity
    max_concurrent INTEGER DEFAULT 10,
    endpoint_url TEXT,               -- direct URL for org_infra providers; null for apn_node (routed via NATS)
    region TEXT,                     -- geographic region hint e.g. 'us-east', 'eu-west'

    -- Availability
    status TEXT NOT NULL DEFAULT 'active', -- 'active' | 'paused' | 'inactive'
    uptime_pct REAL DEFAULT NULL,          -- rolling 7d uptime %
    avg_response_ms INTEGER DEFAULT NULL,  -- rolling avg latency

    -- Service-specific config (JSON)
    metadata TEXT NOT NULL DEFAULT '{}',

    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- Consumer subscriptions: external clients or PCG projects get API keys to call the gateway
CREATE TABLE IF NOT EXISTS marketplace_subscriptions (
    id BLOB NOT NULL PRIMARY KEY,

    -- Consumer identity
    consumer_name TEXT NOT NULL,         -- display name (e.g. "Jungleverse")
    consumer_type TEXT NOT NULL DEFAULT 'external', -- 'project' | 'org' | 'external'
    consumer_project_id BLOB REFERENCES projects(id),
    consumer_org_id BLOB REFERENCES organizations(id),
    contact_email TEXT,

    -- What they're subscribing to
    service_type TEXT NOT NULL,          -- subscribes to all active listings of this type
    listing_id BLOB REFERENCES marketplace_listings(id), -- OR a specific listing

    -- Authentication
    api_key TEXT NOT NULL UNIQUE,        -- bearer token for gateway calls
    api_key_prefix TEXT NOT NULL,        -- first 8 chars for display (e.g. "apn_live")

    -- VIBE economics
    vibe_budget REAL NOT NULL DEFAULT 0.0,   -- allocated balance
    vibe_spent REAL NOT NULL DEFAULT 0.0,    -- lifetime spend
    auto_refill_threshold REAL DEFAULT NULL, -- top-up when balance drops below this
    auto_refill_amount REAL DEFAULT NULL,

    -- Status
    status TEXT NOT NULL DEFAULT 'active', -- 'active' | 'suspended' | 'cancelled'
    suspend_reason TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    expires_at TEXT DEFAULT NULL
);

-- Gateway request log: every request routed through the marketplace
CREATE TABLE IF NOT EXISTS gateway_requests (
    id BLOB NOT NULL PRIMARY KEY,
    subscription_id BLOB NOT NULL REFERENCES marketplace_subscriptions(id),
    listing_id BLOB REFERENCES marketplace_listings(id),
    service_type TEXT NOT NULL,

    -- Provider that fulfilled this request
    provider_node_id TEXT,
    provider_wallet TEXT,

    -- Request metadata
    request_method TEXT DEFAULT 'POST',
    request_path TEXT,
    request_size_bytes INTEGER NOT NULL DEFAULT 0,
    response_size_bytes INTEGER NOT NULL DEFAULT 0,

    -- Lifecycle
    status TEXT NOT NULL DEFAULT 'pending', -- 'pending' | 'routing' | 'fulfilled' | 'failed' | 'timeout'
    error_message TEXT,

    -- Economics
    vibe_charged REAL NOT NULL DEFAULT 0.0,  -- debited from subscription
    vibe_rewarded REAL NOT NULL DEFAULT 0.0, -- credited to provider node

    -- Timing
    response_ms INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    fulfilled_at TEXT DEFAULT NULL
);

CREATE INDEX IF NOT EXISTS idx_marketplace_listings_service_type ON marketplace_listings(service_type);
CREATE INDEX IF NOT EXISTS idx_marketplace_listings_status ON marketplace_listings(status);
CREATE INDEX IF NOT EXISTS idx_marketplace_listings_provider_node ON marketplace_listings(provider_node_id);
CREATE INDEX IF NOT EXISTS idx_marketplace_subs_api_key ON marketplace_subscriptions(api_key);
CREATE INDEX IF NOT EXISTS idx_marketplace_subs_service_type ON marketplace_subscriptions(service_type);
CREATE INDEX IF NOT EXISTS idx_marketplace_subs_consumer ON marketplace_subscriptions(consumer_project_id);
CREATE INDEX IF NOT EXISTS idx_gateway_requests_sub ON gateway_requests(subscription_id);
CREATE INDEX IF NOT EXISTS idx_gateway_requests_listing ON gateway_requests(listing_id);
CREATE INDEX IF NOT EXISTS idx_gateway_requests_created ON gateway_requests(created_at);
CREATE INDEX IF NOT EXISTS idx_gateway_requests_status ON gateway_requests(status);
