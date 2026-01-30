-- Migration: Add CRM Pipelines for Kanban Board Support
-- Creates pipeline and stage tables, updates deals table with pipeline references

-- ============================================================================
-- PART 1: CRM Pipelines
-- ============================================================================

CREATE TABLE IF NOT EXISTS crm_pipelines (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,

    -- Pipeline info
    name TEXT NOT NULL,
    description TEXT,
    pipeline_type TEXT NOT NULL CHECK (pipeline_type IN (
        'conferences', 'clients', 'custom'
    )),

    -- Status
    is_active INTEGER DEFAULT 1,
    is_default INTEGER DEFAULT 0,

    -- Metadata
    icon TEXT,
    color TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(project_id, name)
);

CREATE INDEX IF NOT EXISTS idx_crm_pipelines_project ON crm_pipelines(project_id);
CREATE INDEX IF NOT EXISTS idx_crm_pipelines_type ON crm_pipelines(pipeline_type);

-- ============================================================================
-- PART 2: CRM Pipeline Stages
-- ============================================================================

CREATE TABLE IF NOT EXISTS crm_pipeline_stages (
    id BLOB PRIMARY KEY,
    pipeline_id BLOB NOT NULL REFERENCES crm_pipelines(id) ON DELETE CASCADE,

    -- Stage info
    name TEXT NOT NULL,
    description TEXT,
    color TEXT NOT NULL DEFAULT '#6B7280',

    -- Position and state
    position INTEGER NOT NULL DEFAULT 0,
    is_closed INTEGER DEFAULT 0,
    is_won INTEGER DEFAULT 0,
    probability INTEGER DEFAULT 0 CHECK (probability >= 0 AND probability <= 100),

    -- Automation hints
    auto_move_after_days INTEGER,
    notify_on_enter INTEGER DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(pipeline_id, name),
    UNIQUE(pipeline_id, position)
);

CREATE INDEX IF NOT EXISTS idx_crm_pipeline_stages_pipeline ON crm_pipeline_stages(pipeline_id);
CREATE INDEX IF NOT EXISTS idx_crm_pipeline_stages_position ON crm_pipeline_stages(pipeline_id, position);

-- ============================================================================
-- PART 3: Update CRM Deals with Pipeline References
-- ============================================================================

-- Add new columns to crm_deals for pipeline support
ALTER TABLE crm_deals ADD COLUMN crm_pipeline_id BLOB REFERENCES crm_pipelines(id) ON DELETE SET NULL;
ALTER TABLE crm_deals ADD COLUMN crm_stage_id BLOB REFERENCES crm_pipeline_stages(id) ON DELETE SET NULL;
ALTER TABLE crm_deals ADD COLUMN position INTEGER DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_crm_deals_pipeline ON crm_deals(crm_pipeline_id);
CREATE INDEX IF NOT EXISTS idx_crm_deals_stage ON crm_deals(crm_stage_id);
CREATE INDEX IF NOT EXISTS idx_crm_deals_position ON crm_deals(crm_stage_id, position);

-- ============================================================================
-- PART 4: Seed Default Pipelines for Existing Projects
-- ============================================================================

-- Note: We'll seed pipelines programmatically when projects are accessed
-- to avoid complex SQLite INSERT with subqueries
