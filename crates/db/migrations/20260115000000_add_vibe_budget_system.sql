-- Add VIBE budget fields to projects
ALTER TABLE projects ADD COLUMN vibe_budget_limit INTEGER;
ALTER TABLE projects ADD COLUMN vibe_spent_amount INTEGER NOT NULL DEFAULT 0;

-- Add VIBE budget fields to agent_wallets
ALTER TABLE agent_wallets ADD COLUMN vibe_budget_limit INTEGER;
ALTER TABLE agent_wallets ADD COLUMN vibe_spent_amount INTEGER NOT NULL DEFAULT 0;

-- VIBE transaction ledger for tracking all cost deductions
CREATE TABLE vibe_transactions (
    id BLOB PRIMARY KEY,
    -- Source of deduction (agent wallet or project)
    source_type TEXT NOT NULL CHECK(source_type IN ('agent', 'project')),
    source_id BLOB NOT NULL,
    -- Amount in VIBE units (1 VIBE = $0.001 USD)
    amount_vibe INTEGER NOT NULL,
    -- Token usage details
    input_tokens INTEGER,
    output_tokens INTEGER,
    model TEXT,
    provider TEXT,
    -- Cost calculation in cents (for audit)
    calculated_cost_cents INTEGER,
    -- Blockchain reference
    aptos_tx_hash TEXT,
    aptos_tx_status TEXT CHECK(aptos_tx_status IN ('pending', 'confirmed', 'failed')),
    -- Task context
    task_id BLOB,
    task_attempt_id BLOB,
    process_id BLOB,
    -- Metadata
    description TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX idx_vibe_tx_source ON vibe_transactions(source_type, source_id);
CREATE INDEX idx_vibe_tx_created ON vibe_transactions(created_at);
CREATE INDEX idx_vibe_tx_aptos_hash ON vibe_transactions(aptos_tx_hash);
CREATE INDEX idx_vibe_tx_task ON vibe_transactions(task_id);

-- Model pricing lookup table (2x market rate for profit margin)
CREATE TABLE model_pricing (
    id BLOB PRIMARY KEY,
    model TEXT NOT NULL,
    provider TEXT NOT NULL,
    -- Costs in cents per 1 million tokens
    input_cost_per_million INTEGER NOT NULL,
    output_cost_per_million INTEGER NOT NULL,
    -- Multiplier applied to market rate (default 2.0 = 2x)
    multiplier REAL NOT NULL DEFAULT 2.0,
    -- Effective date for versioning
    effective_from TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    UNIQUE(model, provider)
);

-- Seed initial pricing at 2x market rate (costs in cents per 1M tokens)
-- GPT-4o: $2.50/$10 market -> $5/$20 at 2x = 500/2000 cents
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'gpt-4o', 'openai', 500, 2000, 2.0);

-- GPT-4: $30/$60 market -> $60/$120 at 2x = 6000/12000 cents
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'gpt-4', 'openai', 6000, 12000, 2.0);

-- GPT-5/Codex: Same as GPT-4
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'gpt-5', 'openai', 6000, 12000, 2.0);

INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'gpt-5-codex', 'openai', 6000, 12000, 2.0);

-- Claude Sonnet 4: $3/$15 market -> $6/$30 at 2x = 600/3000 cents
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'claude-sonnet-4', 'anthropic', 600, 3000, 2.0);

-- Claude Opus 4: $15/$75 market -> $30/$150 at 2x = 3000/15000 cents
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'claude-opus-4', 'anthropic', 3000, 15000, 2.0);

-- Gemini Pro: $1.25/$5 market -> $2.50/$10 at 2x = 250/1000 cents
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'gemini-pro', 'google', 250, 1000, 2.0);

-- GPT-OSS (Local/Ollama): Very low cost = 50/200 cents (effectively $0.50/$2 per 1M)
INSERT INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'gpt-oss', 'ollama', 50, 200, 2.0);
