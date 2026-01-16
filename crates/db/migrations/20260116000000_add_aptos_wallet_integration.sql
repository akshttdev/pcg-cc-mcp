-- Add Aptos blockchain integration fields to agent_wallets
-- These enable on-chain VIBE token transfers

-- Aptos address for the agent's on-chain wallet
ALTER TABLE agent_wallets ADD COLUMN aptos_address TEXT;

-- Encrypted private key for signing transactions
-- Uses AES-256-GCM encryption with a server-side key
ALTER TABLE agent_wallets ADD COLUMN aptos_private_key_encrypted TEXT;

-- Track if wallet has been funded with APT for gas
ALTER TABLE agent_wallets ADD COLUMN aptos_funded INTEGER NOT NULL DEFAULT 0;

-- Add Aptos fields to projects for project-level on-chain wallets
ALTER TABLE projects ADD COLUMN aptos_address TEXT;
ALTER TABLE projects ADD COLUMN aptos_private_key_encrypted TEXT;
ALTER TABLE projects ADD COLUMN aptos_funded INTEGER NOT NULL DEFAULT 0;

-- Create indexes for looking up wallets by Aptos address
CREATE INDEX idx_agent_wallets_aptos_address ON agent_wallets(aptos_address);
CREATE INDEX idx_projects_aptos_address ON projects(aptos_address);

-- Update vibe_transactions to track on-chain sync status
ALTER TABLE vibe_transactions ADD COLUMN on_chain_synced INTEGER NOT NULL DEFAULT 0;
