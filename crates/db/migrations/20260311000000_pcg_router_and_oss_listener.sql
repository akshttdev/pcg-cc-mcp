-- PCG Router: unified LLM model registry (OpenRouter-compatible sovereign layer)
CREATE TABLE IF NOT EXISTS pcg_router_models (
    id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    model_id TEXT NOT NULL,
    provider TEXT NOT NULL,             -- 'anthropic' | 'openai' | 'openrouter' | 'local'
    provider_base_url TEXT,             -- optional override base URL
    api_key_env_var TEXT,               -- env var holding the API key
    priority INTEGER NOT NULL DEFAULT 10,
    context_window INTEGER,
    max_output_tokens INTEGER,
    supports_tools INTEGER NOT NULL DEFAULT 1,
    supports_vision INTEGER NOT NULL DEFAULT 0,
    cost_per_million_input INTEGER NOT NULL DEFAULT 0,
    cost_per_million_output INTEGER NOT NULL DEFAULT 0,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(provider, model_id)
);

INSERT OR IGNORE INTO pcg_router_models
    (id, name, model_id, provider, api_key_env_var, priority, context_window, max_output_tokens, supports_tools, supports_vision, cost_per_million_input, cost_per_million_output)
VALUES
    (randomblob(16), 'Claude Sonnet 4.6', 'claude-sonnet-4-6',         'anthropic', 'ANTHROPIC_API_KEY', 1, 200000, 8192, 1, 1, 3000,  15000),
    (randomblob(16), 'Claude Opus 4.6',   'claude-opus-4-6',           'anthropic', 'ANTHROPIC_API_KEY', 2, 200000, 8192, 1, 1, 15000, 75000),
    (randomblob(16), 'Claude Haiku 4.5',  'claude-haiku-4-5-20251001', 'anthropic', 'ANTHROPIC_API_KEY', 3, 200000, 8192, 1, 1, 800,   4000),
    (randomblob(16), 'GPT-4o',            'gpt-4o',                    'openai',    'OPENAI_API_KEY',    5, 128000, 4096, 1, 1, 2500,  10000),
    (randomblob(16), 'GPT-4o Mini',       'gpt-4o-mini',               'openai',    'OPENAI_API_KEY',    6, 128000, 4096, 1, 1, 150,   600);

-- OSS Library Tracker
CREATE TABLE IF NOT EXISTS oss_libraries (
    id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    github_owner TEXT NOT NULL,
    github_repo TEXT NOT NULL,
    tracked_version TEXT,
    latest_version TEXT,
    last_checked_at TEXT,
    check_interval_secs INTEGER NOT NULL DEFAULT 3600,
    is_active INTEGER NOT NULL DEFAULT 1,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(github_owner, github_repo)
);

INSERT OR IGNORE INTO oss_libraries (id, name, github_owner, github_repo, notes)
VALUES
    (randomblob(16), 'axum',    'tokio-rs',    'axum',    'Web framework'),
    (randomblob(16), 'sqlx',    'launchbadge', 'sqlx',    'Async SQL toolkit'),
    (randomblob(16), 'tokio',   'tokio-rs',    'tokio',   'Async runtime'),
    (randomblob(16), 'serde',   'serde-rs',    'serde',   'Serialization framework'),
    (randomblob(16), 'reqwest', 'seanmonstar', 'reqwest', 'HTTP client');

-- OSS Library Updates with agent workflow recommendation
CREATE TABLE IF NOT EXISTS oss_library_updates (
    id BLOB PRIMARY KEY,
    library_id BLOB NOT NULL REFERENCES oss_libraries(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    release_url TEXT,
    release_notes TEXT,
    significance TEXT NOT NULL DEFAULT 'patch',
    agent_recommendation TEXT,
    recommendation_status TEXT NOT NULL DEFAULT 'pending',
    agent_flow_id BLOB,
    published_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(library_id, version)
);
