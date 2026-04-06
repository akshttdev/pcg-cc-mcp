-- Migration: Org VIBE Wallet + Peer Node Ownership
-- 1. Add wallet/vibe fields to organizations
-- 2. Add organization_id FK to peer_nodes

ALTER TABLE organizations ADD COLUMN wallet_address TEXT;
ALTER TABLE organizations ADD COLUMN vibe_balance REAL NOT NULL DEFAULT 0.0;
ALTER TABLE organizations ADD COLUMN vibe_budget_total REAL;

CREATE INDEX IF NOT EXISTS idx_organizations_wallet_address
    ON organizations(wallet_address) WHERE wallet_address IS NOT NULL;

ALTER TABLE peer_nodes ADD COLUMN organization_id TEXT REFERENCES organizations(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_peer_nodes_organization_id
    ON peer_nodes(organization_id) WHERE organization_id IS NOT NULL;
