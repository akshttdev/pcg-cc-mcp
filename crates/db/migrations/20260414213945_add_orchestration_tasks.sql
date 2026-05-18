-- Orchestration tasks: child entities of agent_flows for parallel task execution
-- Part of Task Orchestration Framework (see planning/2026-04-14--plan--task-orchestration-framework.md)

CREATE TABLE orchestration_tasks (
    id TEXT PRIMARY KEY NOT NULL,
    agent_flow_id TEXT NOT NULL,
    task_type TEXT NOT NULL CHECK (task_type IN (
        'local_bash', 'local_agent', 'remote_agent',
        'in_process_teammate', 'local_workflow', 'monitor_mcp', 'dream'
    )),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'running', 'completed', 'failed', 'killed'
    )),
    description TEXT NOT NULL,
    tool_use_id TEXT,
    start_time_ms INTEGER,
    end_time_ms INTEGER,
    output TEXT,
    error TEXT,
    position INTEGER NOT NULL DEFAULT 0,
    is_concurrency_safe INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    FOREIGN KEY (agent_flow_id) REFERENCES agent_flows(id) ON DELETE CASCADE
);

CREATE INDEX idx_orchestration_tasks_flow ON orchestration_tasks(agent_flow_id);
CREATE INDEX idx_orchestration_tasks_status ON orchestration_tasks(status);
CREATE INDEX idx_orchestration_tasks_flow_status ON orchestration_tasks(agent_flow_id, status);
