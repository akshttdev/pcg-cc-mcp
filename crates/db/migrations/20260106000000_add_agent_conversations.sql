-- Agent Conversations
-- Stores conversation history for each agent, enabling persistent memory and learning.
-- Conversations are scoped to sessions and optionally to projects for context isolation.

CREATE TABLE IF NOT EXISTS agent_conversations (
    -- Primary key
    id BLOB PRIMARY KEY,

    -- Agent that owns this conversation
    agent_id BLOB NOT NULL REFERENCES agents(id) ON DELETE CASCADE,

    -- Session identifier (from frontend/client)
    session_id TEXT NOT NULL,

    -- Optional project scope for context isolation
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,

    -- Optional user identifier for multi-user scenarios
    user_id TEXT,

    -- Conversation state
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived', 'expired')),

    -- Optional title (auto-generated or user-provided)
    title TEXT,

    -- Context snapshot at conversation start (JSON)
    -- Captures project state, relevant data, etc. at the time of conversation start
    context_snapshot TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    last_message_at TEXT,

    -- Message count for quick stats
    message_count INTEGER NOT NULL DEFAULT 0,

    -- Total tokens used in this conversation
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0
);

-- Conversation messages
CREATE TABLE IF NOT EXISTS agent_conversation_messages (
    -- Primary key
    id BLOB PRIMARY KEY,

    -- Parent conversation
    conversation_id BLOB NOT NULL REFERENCES agent_conversations(id) ON DELETE CASCADE,

    -- Message role (matches LLM message roles)
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system', 'tool')),

    -- Message content
    content TEXT NOT NULL,

    -- For tool role messages - the tool call ID this responds to
    tool_call_id TEXT,

    -- For assistant messages with tool calls
    tool_name TEXT,
    tool_arguments TEXT,
    tool_result TEXT,

    -- Token usage for this message
    input_tokens INTEGER,
    output_tokens INTEGER,

    -- Model/provider used for this message (for analytics)
    model TEXT,
    provider TEXT,

    -- Latency in milliseconds
    latency_ms INTEGER,

    -- Timestamp
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient queries

-- Find conversations by agent
CREATE INDEX IF NOT EXISTS idx_agent_conversations_agent
ON agent_conversations(agent_id);

-- Find conversations by project (for project-scoped context)
CREATE INDEX IF NOT EXISTS idx_agent_conversations_project
ON agent_conversations(project_id);

-- Find conversations by session
CREATE INDEX IF NOT EXISTS idx_agent_conversations_session
ON agent_conversations(session_id);

-- Find active conversations (for cleanup/management)
CREATE INDEX IF NOT EXISTS idx_agent_conversations_status
ON agent_conversations(status);

-- Find conversations by agent + session (common lookup)
CREATE INDEX IF NOT EXISTS idx_agent_conversations_agent_session
ON agent_conversations(agent_id, session_id);

-- Find messages by conversation
CREATE INDEX IF NOT EXISTS idx_agent_conversation_messages_conv
ON agent_conversation_messages(conversation_id);

-- Find recent messages (for loading conversation)
CREATE INDEX IF NOT EXISTS idx_agent_conversation_messages_created
ON agent_conversation_messages(conversation_id, created_at);

-- Trigger to update conversation updated_at and last_message_at on new message
CREATE TRIGGER IF NOT EXISTS trg_update_conversation_on_message
AFTER INSERT ON agent_conversation_messages
BEGIN
    UPDATE agent_conversations
    SET updated_at = datetime('now', 'subsec'),
        last_message_at = datetime('now', 'subsec'),
        message_count = message_count + 1,
        total_input_tokens = total_input_tokens + COALESCE(NEW.input_tokens, 0),
        total_output_tokens = total_output_tokens + COALESCE(NEW.output_tokens, 0)
    WHERE id = NEW.conversation_id;
END;
