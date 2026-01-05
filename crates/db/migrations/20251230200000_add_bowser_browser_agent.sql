-- Migration: Add Bowser Browser Agent Support
-- Enables browser automation with Playwright, screenshot capture, visual diffs, and security allowlisting

-- Browser sessions for tracking Playwright instances
CREATE TABLE IF NOT EXISTS browser_sessions (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    browser_type TEXT NOT NULL DEFAULT 'chromium' CHECK (browser_type IN ('chromium', 'firefox', 'webkit')),
    viewport_width INTEGER NOT NULL DEFAULT 1280,
    viewport_height INTEGER NOT NULL DEFAULT 720,
    headless INTEGER NOT NULL DEFAULT 1,
    status TEXT NOT NULL DEFAULT 'starting' CHECK (status IN ('starting', 'active', 'idle', 'closed', 'error')),
    current_url TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    closed_at TEXT
);

-- Screenshots captured during browser sessions
CREATE TABLE IF NOT EXISTS browser_screenshots (
    id BLOB PRIMARY KEY,
    browser_session_id BLOB NOT NULL REFERENCES browser_sessions(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    page_title TEXT,
    screenshot_path TEXT NOT NULL,
    thumbnail_path TEXT,
    -- Visual diff comparison
    baseline_screenshot_id BLOB REFERENCES browser_screenshots(id),
    diff_path TEXT,
    diff_percentage REAL,
    -- Metadata
    viewport_width INTEGER NOT NULL,
    viewport_height INTEGER NOT NULL,
    full_page INTEGER NOT NULL DEFAULT 0,
    metadata TEXT, -- JSON for additional data (element selectors, annotations, etc.)
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Security allowlist for URL access control (Bowser only visits approved URLs)
CREATE TABLE IF NOT EXISTS browser_allowlist (
    id BLOB PRIMARY KEY,
    project_id BLOB REFERENCES projects(id) ON DELETE CASCADE,
    pattern TEXT NOT NULL,
    pattern_type TEXT NOT NULL DEFAULT 'glob' CHECK (pattern_type IN ('glob', 'regex', 'exact')),
    description TEXT,
    is_global INTEGER NOT NULL DEFAULT 0,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Browser actions log for audit trail
CREATE TABLE IF NOT EXISTS browser_actions (
    id BLOB PRIMARY KEY,
    browser_session_id BLOB NOT NULL REFERENCES browser_sessions(id) ON DELETE CASCADE,
    action_type TEXT NOT NULL CHECK (action_type IN (
        'navigate', 'click', 'type', 'scroll', 'screenshot', 'wait',
        'select', 'hover', 'press_key', 'evaluate', 'upload', 'download'
    )),
    target_selector TEXT,
    action_data TEXT, -- JSON with action-specific parameters
    result TEXT CHECK (result IN ('success', 'failed', 'blocked', 'timeout')),
    error_message TEXT,
    duration_ms INTEGER,
    screenshot_id BLOB REFERENCES browser_screenshots(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_browser_sessions_execution
    ON browser_sessions(execution_process_id);

CREATE INDEX IF NOT EXISTS idx_browser_sessions_status
    ON browser_sessions(status);

CREATE INDEX IF NOT EXISTS idx_browser_screenshots_session
    ON browser_screenshots(browser_session_id);

CREATE INDEX IF NOT EXISTS idx_browser_screenshots_url
    ON browser_screenshots(url);

CREATE INDEX IF NOT EXISTS idx_browser_allowlist_project
    ON browser_allowlist(project_id);

CREATE INDEX IF NOT EXISTS idx_browser_allowlist_global
    ON browser_allowlist(is_global) WHERE is_global = 1;

CREATE INDEX IF NOT EXISTS idx_browser_actions_session
    ON browser_actions(browser_session_id);

CREATE INDEX IF NOT EXISTS idx_browser_actions_type
    ON browser_actions(action_type);

-- Default safe patterns (global allowlist for localhost development)
INSERT INTO browser_allowlist (id, pattern, pattern_type, description, is_global) VALUES
    (randomblob(16), 'localhost:*', 'glob', 'Local development server', 1),
    (randomblob(16), '127.0.0.1:*', 'glob', 'Local loopback address', 1),
    (randomblob(16), '0.0.0.0:*', 'glob', 'Local any address', 1),
    (randomblob(16), '*.local:*', 'glob', 'Local network domains', 1);
