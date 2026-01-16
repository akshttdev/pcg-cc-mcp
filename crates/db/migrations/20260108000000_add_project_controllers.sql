-- Project Controller Configuration
-- Each project can have its own AI controller with customizable personality

CREATE TABLE project_controller_config (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL DEFAULT 'Controller',
    personality TEXT NOT NULL DEFAULT 'professional',
    system_prompt TEXT,
    voice_id TEXT,
    avatar_url TEXT,
    model TEXT DEFAULT 'gpt-4o-mini',
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 2048,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Index for fast lookup by project
CREATE INDEX idx_project_controller_config_project_id ON project_controller_config(project_id);

-- Project Controller Conversations
-- Tracks chat sessions between users and project controllers

CREATE TABLE project_controller_conversations (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    title TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_project_controller_conversations_project_id ON project_controller_conversations(project_id);
CREATE INDEX idx_project_controller_conversations_user_id ON project_controller_conversations(user_id);

-- Project Controller Messages
-- Individual messages within a conversation

CREATE TABLE project_controller_messages (
    id TEXT PRIMARY KEY NOT NULL,
    conversation_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    tokens_used INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (conversation_id) REFERENCES project_controller_conversations(id) ON DELETE CASCADE
);

CREATE INDEX idx_project_controller_messages_conversation_id ON project_controller_messages(conversation_id);
