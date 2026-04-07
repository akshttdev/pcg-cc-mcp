-- Fix agent_conversations.agent_id: was BLOB FK to agents.id (TEXT), causing FK constraint failures.
-- Switch agent_id to TEXT so UUID strings bind correctly.

PRAGMA foreign_keys = OFF;

-- Drop the trigger that references agent_conversations so we can recreate the table
DROP TRIGGER IF EXISTS trg_update_conversation_on_message;

CREATE TABLE agent_conversations_new (
    id BLOB PRIMARY KEY,
    agent_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    project_id BLOB REFERENCES projects(id) ON DELETE SET NULL,
    user_id TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'archived', 'expired')),
    title TEXT,
    context_snapshot TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    last_message_at TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    total_input_tokens INTEGER DEFAULT 0,
    total_output_tokens INTEGER DEFAULT 0
);

-- Migrate existing rows: convert BLOB agent_id (16-byte UUID) to TEXT UUID string where applicable
INSERT INTO agent_conversations_new
SELECT id,
       CASE
           WHEN typeof(agent_id) = 'blob' AND length(agent_id) = 16
           THEN lower(hex(substr(agent_id,1,4))) || '-' ||
                lower(hex(substr(agent_id,5,2))) || '-' ||
                lower(hex(substr(agent_id,7,2))) || '-' ||
                lower(hex(substr(agent_id,9,2))) || '-' ||
                lower(hex(substr(agent_id,11,6)))
           ELSE CAST(agent_id AS TEXT)
       END,
       session_id, project_id, user_id, status, title,
       context_snapshot, created_at, updated_at, last_message_at,
       message_count, total_input_tokens, total_output_tokens
FROM agent_conversations;

DROP TABLE agent_conversations;
ALTER TABLE agent_conversations_new RENAME TO agent_conversations;

CREATE INDEX IF NOT EXISTS idx_agent_conversations_agent_session
    ON agent_conversations(agent_id, session_id);

-- Recreate the trigger
CREATE TRIGGER trg_update_conversation_on_message
AFTER INSERT ON agent_conversation_messages
BEGIN
    UPDATE agent_conversations
    SET message_count = message_count + 1,
        last_message_at = datetime('now', 'subsec'),
        updated_at = datetime('now', 'subsec')
    WHERE id = NEW.conversation_id;
END;

PRAGMA foreign_keys = ON;
