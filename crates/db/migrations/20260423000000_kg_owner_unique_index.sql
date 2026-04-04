-- Deduplicate non-project KG entries by (owner_type, owner_id, source_type, source_id).
-- Existing project-scoped rows are covered by the existing UNIQUE(project_id, source_type, source_id).
-- This partial index covers org, company, and user-scoped rows where project_id IS NULL.
CREATE UNIQUE INDEX IF NOT EXISTS idx_pks_owner_source_dedup
    ON project_knowledge_sources(owner_type, owner_id, source_type, source_id)
    WHERE project_id IS NULL;
