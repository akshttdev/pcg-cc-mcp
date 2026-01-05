-- Migration: Add Token Usage Tracking
-- Tracks LLM token consumption per task, agent, and project

CREATE TABLE IF NOT EXISTS token_usage (
    id BLOB PRIMARY KEY,
    task_attempt_id BLOB REFERENCES task_attempts(id) ON DELETE SET NULL,
    agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Model info
    model TEXT NOT NULL,  -- e.g., "claude-3-opus", "gpt-4", "gemini-pro"
    provider TEXT NOT NULL DEFAULT 'anthropic',  -- anthropic, openai, google, etc.

    -- Token counts
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,

    -- Cost tracking (optional, in cents)
    cost_cents INTEGER,

    -- Context
    operation_type TEXT,  -- 'chat', 'completion', 'embedding', 'tool_call'
    metadata TEXT,  -- JSON for additional context

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Indexes for efficient aggregation queries
CREATE INDEX IF NOT EXISTS idx_token_usage_project ON token_usage(project_id);
CREATE INDEX IF NOT EXISTS idx_token_usage_agent ON token_usage(agent_id);
CREATE INDEX IF NOT EXISTS idx_token_usage_task ON token_usage(task_attempt_id);
CREATE INDEX IF NOT EXISTS idx_token_usage_created ON token_usage(created_at);
CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage(model);
CREATE INDEX IF NOT EXISTS idx_token_usage_provider ON token_usage(provider);

-- Daily aggregation view for quick dashboard queries
CREATE VIEW IF NOT EXISTS token_usage_daily AS
SELECT
    date(created_at) as usage_date,
    project_id,
    model,
    provider,
    SUM(input_tokens) as total_input_tokens,
    SUM(output_tokens) as total_output_tokens,
    SUM(total_tokens) as total_tokens,
    SUM(cost_cents) as total_cost_cents,
    COUNT(*) as request_count
FROM token_usage
GROUP BY date(created_at), project_id, model, provider;
