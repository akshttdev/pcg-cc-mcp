-- Upgrade agent default models from OpenAI/Ollama to Claude Sonnet 4
-- This updates existing DB records that were seeded with INSERT OR IGNORE

-- Nora: Upgrade from gpt-4o to Claude Sonnet 4 with Anthropic provider
UPDATE agents SET default_model = 'claude-sonnet-4-20250514',
  model_config = json_set(COALESCE(model_config, '{}'), '$.provider', 'anthropic', '$.max_tokens', 8192)
WHERE short_name = 'Nora';

-- Maci, Editron, Scout, Astra, Genesis: Upgrade to Claude Sonnet 4
UPDATE agents SET default_model = 'claude-sonnet-4-20250514',
  model_config = json_set(COALESCE(model_config, '{}'), '$.provider', 'anthropic')
WHERE short_name IN ('Maci', 'Editron', 'Scout', 'Astra', 'Genesis');

-- Auri already uses claude-sonnet-4, just ensure provider is set
UPDATE agents SET
  model_config = json_set(COALESCE(model_config, '{}'), '$.provider', 'anthropic')
WHERE short_name = 'Auri';
