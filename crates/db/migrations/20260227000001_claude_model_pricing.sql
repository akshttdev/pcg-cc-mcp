-- Add pricing rows for Claude model variants used by agents
-- The seed migration included claude-sonnet-4 and claude-opus-4 aliases,
-- but agents now use the full dated model IDs. Add those for VIBE billing.

-- Claude Sonnet 4 (dated): $3/$15 market -> $6/$30 at 2x = 600/3000 cents
INSERT OR IGNORE INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'claude-sonnet-4-20250514', 'anthropic', 600, 3000, 2.0);

-- Claude 3.5 Sonnet (fallback): $3/$15 market -> $6/$30 at 2x = 600/3000 cents
INSERT OR IGNORE INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'claude-3-5-sonnet-20241022', 'anthropic', 600, 3000, 2.0);

-- Claude 3.5 Haiku: $0.80/$4 market -> $1.60/$8 at 2x = 160/800 cents
INSERT OR IGNORE INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'claude-3-5-haiku-20241022', 'anthropic', 160, 800, 2.0);

-- Claude Opus 4 (dated): $15/$75 market -> $30/$150 at 2x = 3000/15000 cents
INSERT OR IGNORE INTO model_pricing (id, model, provider, input_cost_per_million, output_cost_per_million, multiplier) VALUES
    (randomblob(16), 'claude-opus-4-20250514', 'anthropic', 3000, 15000, 2.0);
