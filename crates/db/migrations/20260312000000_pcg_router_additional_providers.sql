-- Additional provider models for the PCG Router.
-- All seeded as is_enabled = 0; enable each once the corresponding API key
-- is added to .env.
--
-- Env vars to add when ready:
--   GEMINI_API_KEY    — https://aistudio.google.com/
--   XAI_API_KEY       — https://console.x.ai/
--   MISTRAL_API_KEY   — https://console.mistral.ai/
--   DEEPSEEK_API_KEY  — https://platform.deepseek.com/
--   GROQ_API_KEY      — https://console.groq.com/

INSERT OR IGNORE INTO pcg_router_models
    (id, name, model_id, provider, api_key_env_var,
     priority, context_window, max_output_tokens,
     supports_tools, supports_vision,
     cost_per_million_input, cost_per_million_output, is_enabled)
VALUES
    -- ── Google Gemini ──────────────────────────────────────────────────────
    (randomblob(16), 'Gemini 2.0 Flash',   'gemini-2.0-flash',          'gemini', 'GEMINI_API_KEY',   4,  1000000, 8192,  1, 1, 100,  400,  0),
    (randomblob(16), 'Gemini 2.5 Pro',     'gemini-2.5-pro-preview-03-25','gemini','GEMINI_API_KEY',  7,  1000000, 65536, 1, 1, 1250, 10000,0),

    -- ── xAI Grok ──────────────────────────────────────────────────────────
    (randomblob(16), 'Grok 3',             'grok-3',                    'xai',    'XAI_API_KEY',      8,  131072,  8192,  1, 1, 3000, 15000, 0),
    (randomblob(16), 'Grok 3 Mini',        'grok-3-mini',               'xai',    'XAI_API_KEY',      9,  131072,  8192,  1, 0, 300,  500,   0),

    -- ── Mistral ───────────────────────────────────────────────────────────
    (randomblob(16), 'Mistral Large',      'mistral-large-latest',      'mistral','MISTRAL_API_KEY',  10, 128000,  4096,  1, 1, 2000, 6000,  0),
    (randomblob(16), 'Mistral Small',      'mistral-small-latest',      'mistral','MISTRAL_API_KEY',  11, 128000,  4096,  1, 0, 100,  300,   0),

    -- ── DeepSeek ──────────────────────────────────────────────────────────
    (randomblob(16), 'DeepSeek Chat',      'deepseek-chat',             'deepseek','DEEPSEEK_API_KEY',12, 64000,   4096,  1, 0, 270,  1100,  0),
    (randomblob(16), 'DeepSeek R1',        'deepseek-reasoner',         'deepseek','DEEPSEEK_API_KEY',13, 64000,   8192,  0, 0, 550,  2190,  0),

    -- ── Meta Llama via Groq ───────────────────────────────────────────────
    (randomblob(16), 'Llama 3.3 70B',      'llama-3.3-70b-versatile',   'groq',   'GROQ_API_KEY',     14, 128000, 8192,  1, 0, 590,  790,   0),
    (randomblob(16), 'Llama 3.1 8B',       'llama-3.1-8b-instant',      'groq',   'GROQ_API_KEY',     15, 128000, 8192,  0, 0, 50,   80,    0);
