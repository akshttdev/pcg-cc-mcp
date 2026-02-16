-- ORCHA Agent Tier System, User Onboarding & Virtual World Access Control
-- Adds wallet_address and home_project_id to users, owner_id and agent_tier to agents

-- Aptos wallet on users (matches agent wallet_address pattern)
ALTER TABLE users ADD COLUMN wallet_address TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_wallet_address ON users(wallet_address) WHERE wallet_address IS NOT NULL;

-- Per-user agent ownership
ALTER TABLE agents ADD COLUMN owner_id BLOB;

-- Agent tier: 'admin' (Nora/Pythia), 'user' (per-user Orcha), 'system' (Topsi - global but user-facing)
ALTER TABLE agents ADD COLUMN agent_tier TEXT NOT NULL DEFAULT 'user';

-- Home project auto-created on signup
ALTER TABLE users ADD COLUMN home_project_id BLOB;

-- Tag existing agents as admin tier
UPDATE agents SET agent_tier = 'admin' WHERE short_name IN ('Nora','Pythia','Maci','Editron','Genesis','Astra','Scout','Auri');

-- Tag Topsi as system tier (global but user-facing)
UPDATE agents SET agent_tier = 'system' WHERE short_name = 'Topsi';
