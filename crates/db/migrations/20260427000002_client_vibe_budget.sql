-- Migration: Client VIBE Budget Caps
-- Allows orgs to set per-client spending limits

ALTER TABLE clients ADD COLUMN vibe_budget_limit REAL;
ALTER TABLE clients ADD COLUMN vibe_spent_amount REAL NOT NULL DEFAULT 0.0;

CREATE INDEX IF NOT EXISTS idx_clients_vibe_spent
    ON clients(organization_id, vibe_spent_amount DESC);
