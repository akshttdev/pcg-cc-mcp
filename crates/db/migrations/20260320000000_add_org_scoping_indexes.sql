-- ============================================================================
-- Migration: Organization / Client / Member / CRM scoping indexes
-- ============================================================================
-- Adds missing indexes on organization_id, client_id, and join columns
-- used for org-scoped queries. These support the artifact org-scoping fix
-- and ensure performant JOINs across the multi-tenant data model.
--
-- FUTURE IMPROVEMENTS (execution_artifacts org scoping):
--   Currently, artifacts are scoped to organizations at query time by JOINing
--   through data_sources.organization_id via json_extract(metadata, '$.data_source_id').
--   This works but is slow for large datasets because SQLite cannot index
--   json_extract expressions used in JOINs.
--
--   Phase 2 should:
--   1. Add organization_id BLOB column to execution_artifacts
--   2. Backfill from data_sources: UPDATE execution_artifacts SET organization_id =
--      (SELECT ds.organization_id FROM data_sources ds
--       WHERE ds.id = CAST(json_extract(execution_artifacts.metadata, '$.data_source_id') AS TEXT))
--   3. Create index: CREATE INDEX idx_execution_artifacts_org ON execution_artifacts(organization_id)
--   4. Update list_recent_artifacts query to use direct WHERE instead of JOIN
--   5. Set organization_id on INSERT in workflow execution code
--
--   Same pattern should be applied to execution_processes if direct org
--   queries become common (currently scoped via task_attempt → task → project chain).
-- ============================================================================

-- ── Organization-level primary key indexes ──────────────────────────────────

-- projects.organization_id — core join for org-scoped project queries
CREATE INDEX IF NOT EXISTS idx_projects_organization_id
    ON projects(organization_id) WHERE organization_id IS NOT NULL;

-- projects.client_id — client-scoped project lookups
CREATE INDEX IF NOT EXISTS idx_projects_client_id
    ON projects(client_id) WHERE client_id IS NOT NULL;

-- ── CRM table org/client scoping indexes ────────────────────────────────────
-- (Some may already exist from 20260307000000 — IF NOT EXISTS guards duplicates)

-- crm_pipelines: org + client scoping
CREATE INDEX IF NOT EXISTS idx_crm_pipelines_organization
    ON crm_pipelines(organization_id) WHERE organization_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_pipelines_client
    ON crm_pipelines(client_id) WHERE client_id IS NOT NULL;

-- crm_pipeline_stages: pipeline_id FK (used in deal form stage lookups)
CREATE INDEX IF NOT EXISTS idx_crm_pipeline_stages_pipeline
    ON crm_pipeline_stages(pipeline_id);

-- crm_contacts: org + client (may already exist)
CREATE INDEX IF NOT EXISTS idx_crm_contacts_organization
    ON crm_contacts(organization_id) WHERE organization_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_contacts_client
    ON crm_contacts(client_id) WHERE client_id IS NOT NULL;

-- crm_deals: org + client + pipeline + stage (may partially exist)
CREATE INDEX IF NOT EXISTS idx_crm_deals_organization
    ON crm_deals(organization_id) WHERE organization_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_deals_client
    ON crm_deals(client_id) WHERE client_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_crm_deals_pipeline_id
    ON crm_deals(crm_pipeline_id);

CREATE INDEX IF NOT EXISTS idx_crm_deals_stage_id
    ON crm_deals(crm_stage_id);

-- crm_activities: org scoping
CREATE INDEX IF NOT EXISTS idx_crm_activities_organization
    ON crm_activities(organization_id) WHERE organization_id IS NOT NULL;

-- ── Member-level indexes ────────────────────────────────────────────────────

-- organization_members composite for role-based lookups
CREATE INDEX IF NOT EXISTS idx_org_members_org_role
    ON organization_members(organization_id, role);

-- ── Artifact / execution join-chain indexes ─────────────────────────────────

-- data_sources.organization_id — used by artifact org-scoping JOIN
-- (already exists from 20260310000000 but re-declare for safety)
CREATE INDEX IF NOT EXISTS idx_data_sources_org
    ON data_sources(organization_id) WHERE organization_id IS NOT NULL;

-- execution_artifacts.created_at — ORDER BY in recent queries
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_created_at
    ON execution_artifacts(created_at DESC);

-- ── Workflow staging org scoping ────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_staging_org
    ON workflow_output_staging(organization_id) WHERE organization_id IS NOT NULL;

-- ── Workflow definitions owner lookup ───────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_workflow_definitions_owner
    ON workflow_definitions(owner_type, owner_id);

-- ── Companies org link ──────────────────────────────────────────────────────

CREATE INDEX IF NOT EXISTS idx_companies_org_id
    ON companies(organization_id) WHERE organization_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_companies_created_by_org
    ON companies(created_by_org_id) WHERE created_by_org_id IS NOT NULL;
