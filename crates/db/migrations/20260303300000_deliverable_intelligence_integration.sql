-- ─────────────────────────────────────────────────────────────────────────────
-- Deliverables × Artifacts × Intelligence
-- 2026-03-03
-- ─────────────────────────────────────────────────────────────────────────────

-- Link deliverables back to execution_artifacts (the workflow artifact system)
-- and to project_knowledge_sources (the knowledge graph).
-- Both are nullable: set when deliverable is marked done and registered.
ALTER TABLE deliverables ADD COLUMN artifact_id BLOB REFERENCES execution_artifacts(id) ON DELETE SET NULL;
ALTER TABLE deliverables ADD COLUMN knowledge_source_id BLOB REFERENCES project_knowledge_sources(id) ON DELETE SET NULL;

-- Allow persons to carry a research status flag so the UI can show in-progress
ALTER TABLE persons ADD COLUMN intelligence_status TEXT NOT NULL DEFAULT 'idle'
    CHECK(intelligence_status IN ('idle','queued','running','done','failed'));
ALTER TABLE persons ADD COLUMN intelligence_agent TEXT; -- 'scout' | 'astra' | etc.

-- Store raw research JSON per person (may be large — kept separate from main SELECT *)
-- Already exists as intelligence_raw TEXT on persons from the first migration.
-- Nothing more needed here.
