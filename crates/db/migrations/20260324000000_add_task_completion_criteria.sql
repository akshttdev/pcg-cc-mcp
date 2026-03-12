-- Add structured completion criteria and output format fields to tasks.
-- These enable AI agents to self-evaluate whether a task is complete
-- by checking measurable success criteria and expected deliverable format.

ALTER TABLE tasks ADD COLUMN completion_criteria TEXT;
ALTER TABLE tasks ADD COLUMN output_format TEXT;
