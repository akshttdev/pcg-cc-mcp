CREATE TABLE review_tokens (
    id BLOB PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    deliverable_id BLOB NOT NULL,
    created_by BLOB,
    expires_at TEXT,
    view_count INTEGER DEFAULT 0,
    is_active INTEGER DEFAULT 1,
    created_at TEXT DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_review_tokens_token ON review_tokens(token);

CREATE TABLE review_comments (
    id BLOB PRIMARY KEY,
    deliverable_id BLOB NOT NULL,
    token_id BLOB,
    timecode_seconds REAL,
    author_name TEXT DEFAULT 'Reviewer',
    author_email TEXT,
    content TEXT NOT NULL,
    is_resolved INTEGER DEFAULT 0,
    resolved_by TEXT, resolved_at TEXT,
    created_at TEXT DEFAULT (datetime('now','subsec')),
    updated_at TEXT DEFAULT (datetime('now','subsec'))
);
CREATE INDEX idx_review_comments_deliverable ON review_comments(deliverable_id);
