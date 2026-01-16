-- Refactor to Treasury Custody Model
-- Users deposit VIBE to treasury, usage tracked in DB, treasury handles all gas

-- Drop agent wallet Aptos columns (not needed - treasury handles all on-chain)
-- SQLite doesn't support DROP COLUMN easily, so we'll just ignore these fields
-- and remove them from the Rust model

-- Create deposits table to track on-chain VIBE deposits to treasury
CREATE TABLE vibe_deposits (
    id BLOB PRIMARY KEY,
    -- Who made the deposit (project or user)
    project_id BLOB NOT NULL REFERENCES projects(id),
    -- Aptos transaction hash of the deposit
    tx_hash TEXT NOT NULL UNIQUE,
    -- Sender address (user's wallet)
    sender_address TEXT NOT NULL,
    -- Amount deposited (in VIBE smallest units)
    amount_vibe INTEGER NOT NULL,
    -- Status: pending (seen on chain), confirmed (enough confirmations), credited (added to balance)
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'confirmed', 'credited', 'failed')),
    -- Block height when deposit was seen
    block_height INTEGER,
    -- When we first saw the deposit
    detected_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    -- When it was credited to project balance
    credited_at TEXT,
    -- Any error message if failed
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_vibe_deposits_project ON vibe_deposits(project_id);
CREATE INDEX idx_vibe_deposits_tx_hash ON vibe_deposits(tx_hash);
CREATE INDEX idx_vibe_deposits_status ON vibe_deposits(status);
CREATE INDEX idx_vibe_deposits_sender ON vibe_deposits(sender_address);

-- Create withdrawals table for when users withdraw VIBE from their balance
CREATE TABLE vibe_withdrawals (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id),
    -- Destination address (user's wallet)
    destination_address TEXT NOT NULL,
    -- Amount requested
    amount_vibe INTEGER NOT NULL,
    -- Status: pending, processing, completed, failed
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
    -- Aptos transaction hash once processed
    tx_hash TEXT,
    -- When withdrawal was requested
    requested_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    -- When it was processed
    processed_at TEXT,
    -- Any error message
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_vibe_withdrawals_project ON vibe_withdrawals(project_id);
CREATE INDEX idx_vibe_withdrawals_status ON vibe_withdrawals(status);

-- Add deposit tracking fields to projects
ALTER TABLE projects ADD COLUMN total_vibe_deposited INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN total_vibe_withdrawn INTEGER NOT NULL DEFAULT 0;

-- Note: Effective balance = total_vibe_deposited - total_vibe_withdrawn - vibe_spent_amount
-- This ensures we can always reconcile on-chain deposits with off-chain usage
