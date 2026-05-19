-- PCG Router: TTS (Text-to-Speech) provider registry
CREATE TABLE IF NOT EXISTS pcg_router_tts_providers (
    id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,        -- 'elevenlabs', 'openai', 'azure', 'chatterbox', 'system'
    provider_base_url TEXT,
    api_key_env_var TEXT,
    api_key_value TEXT,
    priority INTEGER NOT NULL DEFAULT 10,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_local INTEGER NOT NULL DEFAULT 0,
    supported_voices TEXT,              -- JSON array of voice IDs
    supported_formats TEXT DEFAULT '["mp3","wav"]',
    supports_streaming INTEGER DEFAULT 0,
    max_chars_per_request INTEGER DEFAULT 5000,
    cost_per_1000_chars INTEGER DEFAULT 0,  -- microdollars
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(provider_type, name)
);

-- PCG Router: STT (Speech-to-Text) provider registry
CREATE TABLE IF NOT EXISTS pcg_router_stt_providers (
    id BLOB PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,        -- 'whisper', 'local_whisper', 'azure', 'google', 'system'
    provider_base_url TEXT,
    api_key_env_var TEXT,
    api_key_value TEXT,
    priority INTEGER NOT NULL DEFAULT 10,
    is_enabled INTEGER NOT NULL DEFAULT 1,
    is_local INTEGER NOT NULL DEFAULT 0,
    supported_languages TEXT DEFAULT '["en"]',
    supports_streaming INTEGER DEFAULT 0,
    supports_word_timestamps INTEGER DEFAULT 0,
    max_audio_duration_sec INTEGER DEFAULT 300,
    cost_per_minute INTEGER DEFAULT 0,  -- microdollars
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(provider_type, name)
);

-- Seed TTS providers (disabled by default — enable via admin UI)
INSERT OR IGNORE INTO pcg_router_tts_providers
    (id, name, provider_type, api_key_env_var, priority, is_enabled, is_local, supports_streaming, max_chars_per_request, cost_per_1000_chars)
VALUES
    (randomblob(16), 'ElevenLabs',      'elevenlabs', 'ELEVENLABS_API_KEY', 1,  0, 0, 1, 5000,  300000),  -- $0.30/1K chars
    (randomblob(16), 'OpenAI TTS',      'openai',     'OPENAI_API_KEY',     2,  0, 0, 0, 4096,  15000),   -- $0.015/1K chars
    (randomblob(16), 'Azure TTS',       'azure',      'AZURE_SPEECH_KEY',   3,  0, 0, 1, 10000, 16000),   -- $0.016/1K chars
    (randomblob(16), 'Chatterbox Local','chatterbox', NULL,                 10, 0, 1, 0, 5000,  0);       -- Free, local

-- Seed STT providers (disabled by default — enable via admin UI)
INSERT OR IGNORE INTO pcg_router_stt_providers
    (id, name, provider_type, api_key_env_var, priority, is_enabled, is_local, supports_streaming, supports_word_timestamps, max_audio_duration_sec, cost_per_minute)
VALUES
    (randomblob(16), 'OpenAI Whisper',  'whisper',       'OPENAI_API_KEY',    1,  0, 0, 0, 1, 300, 6000),   -- $0.006/min
    (randomblob(16), 'LocalWhisper',    'local_whisper', NULL,                2,  0, 1, 0, 1, 600, 0),      -- Free, local
    (randomblob(16), 'Azure STT',       'azure',         'AZURE_SPEECH_KEY',  3,  0, 0, 1, 1, 300, 10000);  -- $0.010/min

-- Indexes for priority-based lookups
CREATE INDEX IF NOT EXISTS idx_tts_providers_enabled_priority ON pcg_router_tts_providers(is_enabled, priority);
CREATE INDEX IF NOT EXISTS idx_stt_providers_enabled_priority ON pcg_router_stt_providers(is_enabled, priority);
