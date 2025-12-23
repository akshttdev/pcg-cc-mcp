-- Create workflow_executions table for tracking agent workflow runs
CREATE TABLE IF NOT EXISTS workflow_executions (
    id TEXT PRIMARY KEY NOT NULL,
    agent_id TEXT NOT NULL,
    workflow_id TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    project_id TEXT,
    state TEXT NOT NULL, -- JSON serialized WorkflowState
    context TEXT NOT NULL, -- JSON serialized WorkflowContext
    current_stage INTEGER NOT NULL DEFAULT 0,
    created_tasks TEXT, -- JSON array of task UUIDs
    deliverables TEXT, -- JSON array of deliverables
    started_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

-- Index for querying workflows by agent
CREATE INDEX IF NOT EXISTS idx_workflow_executions_agent_id
ON workflow_executions(agent_id);

-- Index for querying workflows by project
CREATE INDEX IF NOT EXISTS idx_workflow_executions_project_id
ON workflow_executions(project_id) WHERE project_id IS NOT NULL;

-- Index for querying active workflows
CREATE INDEX IF NOT EXISTS idx_workflow_executions_completed
ON workflow_executions(completed_at) WHERE completed_at IS NULL;

-- Index for querying by status (for cleanup and monitoring)
CREATE INDEX IF NOT EXISTS idx_workflow_executions_started
ON workflow_executions(started_at);
