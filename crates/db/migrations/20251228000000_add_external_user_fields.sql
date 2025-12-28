-- Add external authentication fields to users table
-- Allows users authenticated via external providers (e.g., Jungleverse) to be tracked

ALTER TABLE users ADD COLUMN external_provider TEXT;
ALTER TABLE users ADD COLUMN external_id TEXT;

-- Index for efficient lookup of external users
CREATE UNIQUE INDEX IF NOT EXISTS idx_users_external ON users(external_provider, external_id)
WHERE external_provider IS NOT NULL AND external_id IS NOT NULL;
