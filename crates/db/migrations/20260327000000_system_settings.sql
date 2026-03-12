-- System-wide settings for admin configuration
CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_by TEXT,
    updated_at DATETIME DEFAULT (datetime('now','subsec'))
);

-- Default: VIBE balance check bypass disabled
INSERT OR IGNORE INTO system_settings (key, value) VALUES ('vibe_check_bypass', 'false');
