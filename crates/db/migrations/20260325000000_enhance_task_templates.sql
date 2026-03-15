-- Sprint 3C: Enhance task templates with agent-ready fields
-- These fields make templates "agent-ready task presets" — one click to create a well-specified agent task

ALTER TABLE task_templates ADD COLUMN priority TEXT;  -- critical, high, medium, low
ALTER TABLE task_templates ADD COLUMN completion_criteria TEXT;
ALTER TABLE task_templates ADD COLUMN output_format TEXT;
ALTER TABLE task_templates ADD COLUMN assigned_agent TEXT;  -- agent codename
ALTER TABLE task_templates ADD COLUMN tags TEXT;  -- JSON array
ALTER TABLE task_templates ADD COLUMN organization_id BLOB;  -- for org-scoped templates (NULL = global or project)

CREATE INDEX IF NOT EXISTS idx_task_templates_org ON task_templates(organization_id);
