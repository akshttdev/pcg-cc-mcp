-- Add api_key_value column to store actual API keys (not just env var names)
-- resolve_api_key() checks this first, then falls back to env var lookup
ALTER TABLE pcg_router_models ADD COLUMN api_key_value TEXT;
