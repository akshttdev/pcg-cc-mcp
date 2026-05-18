-- Fix BUG-2 (backstop): DB-level unique index to prevent duplicate phase1_scout tasks per deal
-- Partial index on non-deleted rows only

CREATE UNIQUE INDEX IF NOT EXISTS uq_tasks_phase1_scout_per_deal
ON tasks (crm_deal_id, workflow_type)
WHERE workflow_type = 'phase1_scout' AND deleted_at IS NULL;
