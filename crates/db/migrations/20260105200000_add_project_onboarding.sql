-- Project Onboarding System
-- Tracks the Airo-style onboarding workflow for new projects

-- Main onboarding tracking table
CREATE TABLE IF NOT EXISTS project_onboarding (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'paused', 'completed', 'cancelled')),
    current_phase TEXT NOT NULL DEFAULT 'context_gathering',
    -- JSON: Answers to NORA's initial questions
    context_data TEXT,
    -- JSON: Overall recommendations from research phase
    recommendations TEXT,
    started_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    UNIQUE(project_id)
);

-- Individual workflow segments (carousel items)
CREATE TABLE IF NOT EXISTS onboarding_segments (
    id BLOB PRIMARY KEY,
    project_id BLOB NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    onboarding_id BLOB NOT NULL REFERENCES project_onboarding(id) ON DELETE CASCADE,
    -- Segment type: research, brand, website, email, legal, social
    segment_type TEXT NOT NULL,
    -- Display name for the segment
    name TEXT NOT NULL,
    -- Agent assigned to this segment (e.g., Genesis for brand, Auri for website)
    assigned_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    assigned_agent_name TEXT,
    -- Status tracking
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'in_progress', 'needs_review', 'completed', 'skipped')),
    -- JSON: Agent-generated recommendations for this segment
    recommendations TEXT,
    -- JSON: Human selections/refinements/decisions
    user_decisions TEXT,
    -- Order in carousel
    order_index INTEGER NOT NULL,
    -- Timing
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now','subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now','subsec'))
);

-- Link tasks to onboarding workflow phases
ALTER TABLE tasks ADD COLUMN onboarding_segment_id BLOB REFERENCES onboarding_segments(id) ON DELETE SET NULL;
ALTER TABLE tasks ADD COLUMN workflow_phase TEXT;

-- Additional approval tracking fields on tasks (requires_approval, approval_status already exist)
ALTER TABLE tasks ADD COLUMN approved_by TEXT;
ALTER TABLE tasks ADD COLUMN approved_at TEXT;
ALTER TABLE tasks ADD COLUMN approval_options TEXT; -- JSON: available choices
ALTER TABLE tasks ADD COLUMN approval_selection TEXT; -- JSON: what was selected

-- Link assets to creating agent and source task
ALTER TABLE project_assets ADD COLUMN created_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL;
ALTER TABLE project_assets ADD COLUMN source_task_id BLOB REFERENCES tasks(id) ON DELETE SET NULL;
ALTER TABLE project_assets ADD COLUMN workflow_phase TEXT;
ALTER TABLE project_assets ADD COLUMN version INTEGER DEFAULT 1;
ALTER TABLE project_assets ADD COLUMN parent_asset_id BLOB REFERENCES project_assets(id) ON DELETE SET NULL;

-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_project_onboarding_project ON project_onboarding(project_id);
CREATE INDEX IF NOT EXISTS idx_project_onboarding_status ON project_onboarding(status);
CREATE INDEX IF NOT EXISTS idx_onboarding_segments_project ON onboarding_segments(project_id);
CREATE INDEX IF NOT EXISTS idx_onboarding_segments_onboarding ON onboarding_segments(onboarding_id);
CREATE INDEX IF NOT EXISTS idx_onboarding_segments_status ON onboarding_segments(status);
CREATE INDEX IF NOT EXISTS idx_tasks_onboarding_segment ON tasks(onboarding_segment_id);
CREATE INDEX IF NOT EXISTS idx_tasks_workflow_phase ON tasks(workflow_phase);
CREATE INDEX IF NOT EXISTS idx_tasks_approval_status ON tasks(approval_status);
CREATE INDEX IF NOT EXISTS idx_project_assets_agent ON project_assets(created_by_agent_id);
CREATE INDEX IF NOT EXISTS idx_project_assets_source_task ON project_assets(source_task_id);
