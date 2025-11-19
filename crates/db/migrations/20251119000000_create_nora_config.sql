-- Create table for storing Nora voice configuration
CREATE TABLE IF NOT EXISTS nora_voice_config (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Singleton table, only one row allowed
    config_json TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP NOT NULL
);

-- Create trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_nora_voice_config_timestamp
AFTER UPDATE ON nora_voice_config
FOR EACH ROW
BEGIN
    UPDATE nora_voice_config SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- Insert default configuration if none exists
INSERT OR IGNORE INTO nora_voice_config (id, config_json)
VALUES (1, '{}');
