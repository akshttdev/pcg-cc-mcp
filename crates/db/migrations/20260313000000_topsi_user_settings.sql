-- Topsi per-user confirmation settings
-- Controls which tool categories require user confirmation before execution.
CREATE TABLE IF NOT EXISTS topsi_user_settings (
    user_id TEXT PRIMARY KEY,
    -- 'always_confirm' | 'confirm_destructive' | 'autonomous'
    default_confirmation_mode TEXT NOT NULL DEFAULT 'confirm_destructive',
    -- JSON object: {"tool_name": "always_confirm"|"confirm_destructive"|"autonomous"}
    per_tool_overrides TEXT,
    -- Auto-approve timeout in minutes (NULL = no auto-approve)
    auto_approve_timeout_minutes INTEGER,
    created_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now', 'subsec'))
);
