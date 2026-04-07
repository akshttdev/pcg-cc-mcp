-- Migration: Task Execution Flow Type
-- Enables AgentFlowExecutor to handle Topsi-created task flows

-- Add project_id to agent_flows for task context
ALTER TABLE agent_flows ADD COLUMN project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_agent_flows_task_id_status
    ON agent_flows(task_id, status) WHERE task_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_agent_flows_project_id
    ON agent_flows(project_id) WHERE project_id IS NOT NULL;
