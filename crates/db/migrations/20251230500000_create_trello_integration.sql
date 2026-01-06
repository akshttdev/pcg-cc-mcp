-- Trello board connections: links a PCG project to a Trello board
CREATE TABLE IF NOT EXISTS trello_boards (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    trello_board_id TEXT NOT NULL,
    trello_board_name TEXT,
    sync_enabled INTEGER NOT NULL DEFAULT 1,
    default_list_id TEXT,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, trello_board_id)
);

-- Task-level Trello card mappings
CREATE TABLE IF NOT EXISTS trello_task_links (
    id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL UNIQUE,
    trello_card_id TEXT NOT NULL,
    trello_board_id TEXT NOT NULL,
    trello_list_id TEXT,
    origin TEXT NOT NULL DEFAULT 'trello' CHECK(origin IN ('trello', 'pcg')),
    sync_status TEXT NOT NULL DEFAULT 'synced' CHECK(sync_status IN ('synced', 'pending_push', 'pending_pull', 'error')),
    last_sync_error TEXT,
    trello_card_url TEXT,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- Indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_trello_boards_project ON trello_boards(project_id);
CREATE INDEX IF NOT EXISTS idx_trello_task_links_task ON trello_task_links(task_id);
CREATE INDEX IF NOT EXISTS idx_trello_task_links_card ON trello_task_links(trello_card_id);
CREATE INDEX IF NOT EXISTS idx_trello_task_links_origin ON trello_task_links(origin);
CREATE INDEX IF NOT EXISTS idx_trello_task_links_sync_status ON trello_task_links(sync_status);
