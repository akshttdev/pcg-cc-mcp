-- Full model catalog expansion for PCG Router.
-- Adds top models from all 8 major providers.
-- All new rows seeded as is_enabled = 0.
--
-- New env vars needed:
--   DASHSCOPE_API_KEY  — https://dashscope.console.aliyun.com/  (Qwen / Alibaba)

INSERT OR IGNORE INTO pcg_router_models
    (id, name, model_id, provider, api_key_env_var,
     priority, context_window, max_output_tokens,
     supports_tools, supports_vision,
     cost_per_million_input, cost_per_million_output, is_enabled)
VALUES
    -- ── OpenAI (additional) ───────────────────────────────────────────────
    (randomblob(16), 'o4 Mini',              'o4-mini',                    'openai',   'OPENAI_API_KEY',    16, 200000,  100000, 1, 1, 1100,  4400,  0),
    (randomblob(16), 'o3',                   'o3',                         'openai',   'OPENAI_API_KEY',    17, 200000,  100000, 1, 1, 10000, 40000, 0),
    (randomblob(16), 'GPT-4 Turbo',          'gpt-4-turbo',                'openai',   'OPENAI_API_KEY',    18, 128000,  4096,   1, 1, 10000, 30000, 0),

    -- ── Google Gemini (additional) ────────────────────────────────────────
    (randomblob(16), 'Gemini 1.5 Pro',       'gemini-1.5-pro-002',         'gemini',   'GEMINI_API_KEY',    19, 2000000, 8192,   1, 1, 1250,  5000,  0),
    (randomblob(16), 'Gemini 1.5 Flash',     'gemini-1.5-flash-002',       'gemini',   'GEMINI_API_KEY',    20, 1000000, 8192,   1, 1, 75,    300,   0),
    (randomblob(16), 'Gemini 2.0 Flash Lite','gemini-2.0-flash-lite',      'gemini',   'GEMINI_API_KEY',    21, 1000000, 8192,   1, 1, 75,    300,   0),

    -- ── Mistral (additional) ──────────────────────────────────────────────
    (randomblob(16), 'Codestral',            'codestral-latest',            'mistral',  'MISTRAL_API_KEY',   22, 256000,  4096,   1, 0, 1000,  3000,  0),
    (randomblob(16), 'Mistral Nemo',         'open-mistral-nemo',           'mistral',  'MISTRAL_API_KEY',   23, 128000,  4096,   1, 0, 150,   150,   0),

    -- ── Meta Llama via Groq (additional) ─────────────────────────────────
    (randomblob(16), 'Llama 3.1 70B',        'llama-3.1-70b-versatile',    'groq',     'GROQ_API_KEY',      24, 128000,  8192,   1, 0, 590,   790,   0),
    (randomblob(16), 'Mixtral 8x7B',         'mixtral-8x7b-32768',         'groq',     'GROQ_API_KEY',      25, 32768,   4096,   1, 0, 240,   240,   0),

    -- ── xAI (additional) ─────────────────────────────────────────────────
    (randomblob(16), 'Grok 2',               'grok-2-1212',                'xai',      'XAI_API_KEY',       26, 131072,  4096,   1, 1, 2000,  10000, 0),

    -- ── Qwen / Alibaba DashScope ──────────────────────────────────────────
    -- Uses OpenAI-compatible API at dashscope.aliyuncs.com/compatible-mode
    (randomblob(16), 'Qwen Max',             'qwen-max',                   'qwen',     'DASHSCOPE_API_KEY', 27, 32768,   8192,   1, 1, 2400,  9600,  0),
    (randomblob(16), 'QwQ 32B',              'qwq-32b',                    'qwen',     'DASHSCOPE_API_KEY', 28, 32768,   8192,   0, 0, 600,   1800,  0),
    (randomblob(16), 'Qwen 2.5 72B',         'qwen2.5-72b-instruct',       'qwen',     'DASHSCOPE_API_KEY', 29, 131072,  8192,   1, 0, 400,   1200,  0),
    (randomblob(16), 'Qwen Plus',            'qwen-plus',                  'qwen',     'DASHSCOPE_API_KEY', 30, 131072,  8192,   1, 0, 800,   2400,  0),
    (randomblob(16), 'Qwen Turbo',           'qwen-turbo',                 'qwen',     'DASHSCOPE_API_KEY', 31, 1000000, 8192,   1, 0, 50,    150,   0);
