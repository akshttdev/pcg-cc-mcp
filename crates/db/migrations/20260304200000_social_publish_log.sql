CREATE TABLE social_publish_log (
    id BLOB PRIMARY KEY,
    post_id BLOB NOT NULL,
    account_id BLOB NOT NULL,
    attempt_number INTEGER DEFAULT 1,
    status TEXT NOT NULL,
    platform_post_id TEXT,
    platform_url TEXT,
    error_message TEXT,
    response_body TEXT,
    duration_ms INTEGER,
    attempted_at TEXT DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_spl_post   ON social_publish_log(post_id);
CREATE INDEX idx_spl_status ON social_publish_log(status);
