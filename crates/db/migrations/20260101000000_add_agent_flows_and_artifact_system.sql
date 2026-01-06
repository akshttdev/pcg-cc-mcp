-- Migration: Add Agent Flows and Enhanced Artifact System
-- Implements Planner/Executor/Verifier pattern, Wide Research, and artifact collaboration
-- Inspired by Google Antigravity + Manus AI architecture

-- ============================================================================
-- PART 1: Extended Artifact Types
-- ============================================================================

-- Drop and recreate the check constraint to add new artifact types
-- SQLite doesn't support ALTER CONSTRAINT, so we use a trigger approach
CREATE TABLE IF NOT EXISTS execution_artifacts_new (
    id BLOB PRIMARY KEY,
    execution_process_id BLOB NOT NULL REFERENCES execution_processes(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL CHECK (artifact_type IN (
        -- Existing types
        'plan', 'screenshot', 'walkthrough', 'diff_summary', 'test_result', 'checkpoint', 'error_report',
        -- Planning phase artifacts (new)
        'research_report', 'strategy_document', 'content_calendar', 'competitor_analysis',
        -- Execution phase artifacts (new)
        'content_draft', 'visual_brief', 'schedule_manifest', 'engagement_log',
        -- Verification phase artifacts (new)
        'verification_report', 'browser_recording', 'compliance_score', 'platform_screenshot',
        -- Wide Research artifacts (new)
        'subagent_result', 'aggregated_research'
    )),
    title TEXT NOT NULL,
    content TEXT,
    file_path TEXT,
    metadata TEXT, -- JSON for flexible additional data

    -- New columns for enhanced artifact tracking
    phase TEXT CHECK (phase IN ('planning', 'execution', 'verification')),
    created_by_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    review_status TEXT DEFAULT 'none' CHECK (review_status IN ('none', 'pending', 'approved', 'rejected', 'revision_requested')),
    parent_artifact_id BLOB REFERENCES execution_artifacts_new(id) ON DELETE SET NULL, -- For revisions

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

-- Migrate existing data
INSERT INTO execution_artifacts_new (id, execution_process_id, artifact_type, title, content, file_path, metadata, created_at)
SELECT id, execution_process_id, artifact_type, title, content, file_path, metadata, created_at
FROM execution_artifacts;

-- Drop old table and rename new one
DROP TABLE IF EXISTS execution_artifacts;
ALTER TABLE execution_artifacts_new RENAME TO execution_artifacts;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_process
    ON execution_artifacts(execution_process_id);
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_type
    ON execution_artifacts(artifact_type);
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_phase
    ON execution_artifacts(phase);
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_review
    ON execution_artifacts(review_status) WHERE review_status != 'none';
CREATE INDEX IF NOT EXISTS idx_execution_artifacts_agent
    ON execution_artifacts(created_by_agent_id);

-- ============================================================================
-- PART 2: Agent Flows (Planner/Executor/Verifier Pattern)
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_flows (
    id BLOB PRIMARY KEY,
    task_id BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    flow_type TEXT NOT NULL CHECK (flow_type IN (
        'content_creation', 'research', 'engagement', 'scheduling',
        'campaign', 'analysis', 'monitoring', 'custom'
    )),
    status TEXT NOT NULL DEFAULT 'planning' CHECK (status IN (
        'planning', 'executing', 'verifying', 'completed', 'failed', 'paused', 'awaiting_approval'
    )),

    -- Phase agents (Planner → Executor → Verifier)
    planner_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    executor_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    verifier_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,

    current_phase TEXT NOT NULL DEFAULT 'planning' CHECK (current_phase IN (
        'planning', 'execution', 'verification'
    )),

    -- Phase timestamps
    planning_started_at TEXT,
    planning_completed_at TEXT,
    execution_started_at TEXT,
    execution_completed_at TEXT,
    verification_started_at TEXT,
    verification_completed_at TEXT,

    -- Configuration
    flow_config TEXT, -- JSON: autonomy settings, approval gates per phase
    handoff_instructions TEXT, -- Instructions passed between phases

    -- Quality tracking
    verification_score REAL, -- 0.0 to 1.0 from Sentinel verification
    human_approval_required INTEGER NOT NULL DEFAULT 0,
    approved_by TEXT,
    approved_at TEXT,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_agent_flows_task ON agent_flows(task_id);
CREATE INDEX IF NOT EXISTS idx_agent_flows_status ON agent_flows(status);
CREATE INDEX IF NOT EXISTS idx_agent_flows_phase ON agent_flows(current_phase);
CREATE INDEX IF NOT EXISTS idx_agent_flows_awaiting
    ON agent_flows(status) WHERE status = 'awaiting_approval';

-- ============================================================================
-- PART 3: Wide Research Sessions (Parallel Subagent Spawning)
-- ============================================================================

CREATE TABLE IF NOT EXISTS wide_research_sessions (
    id BLOB PRIMARY KEY,
    agent_flow_id BLOB REFERENCES agent_flows(id) ON DELETE CASCADE,
    parent_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,

    task_description TEXT NOT NULL,
    total_subagents INTEGER NOT NULL,
    completed_subagents INTEGER DEFAULT 0,
    failed_subagents INTEGER DEFAULT 0,

    status TEXT NOT NULL DEFAULT 'spawning' CHECK (status IN (
        'spawning', 'in_progress', 'aggregating', 'completed', 'failed', 'cancelled'
    )),

    -- Configuration
    parallelism_limit INTEGER DEFAULT 10, -- Max concurrent subagents
    timeout_per_subagent INTEGER DEFAULT 300000, -- ms

    -- Results
    aggregated_result_artifact_id BLOB REFERENCES execution_artifacts(id) ON DELETE SET NULL,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_wide_research_flow ON wide_research_sessions(agent_flow_id);
CREATE INDEX IF NOT EXISTS idx_wide_research_status ON wide_research_sessions(status);

-- Subagent instances for Wide Research
CREATE TABLE IF NOT EXISTS wide_research_subagents (
    id BLOB PRIMARY KEY,
    session_id BLOB NOT NULL REFERENCES wide_research_sessions(id) ON DELETE CASCADE,

    subagent_index INTEGER NOT NULL,
    target_item TEXT NOT NULL, -- e.g., "@gucci" for competitor research
    target_metadata TEXT, -- JSON for additional context

    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'running', 'completed', 'failed', 'timeout', 'cancelled'
    )),

    -- Execution tracking
    execution_process_id BLOB REFERENCES execution_processes(id) ON DELETE SET NULL,
    result_artifact_id BLOB REFERENCES execution_artifacts(id) ON DELETE SET NULL,
    error_message TEXT,

    started_at TEXT,
    completed_at TEXT,

    UNIQUE(session_id, subagent_index)
);

CREATE INDEX IF NOT EXISTS idx_wide_research_subagents_session
    ON wide_research_subagents(session_id);
CREATE INDEX IF NOT EXISTS idx_wide_research_subagents_status
    ON wide_research_subagents(status);

-- ============================================================================
-- PART 4: Artifact Reviews (Human Collaboration)
-- ============================================================================

CREATE TABLE IF NOT EXISTS artifact_reviews (
    id BLOB PRIMARY KEY,
    artifact_id BLOB NOT NULL REFERENCES execution_artifacts(id) ON DELETE CASCADE,

    -- Reviewer (human or agent)
    reviewer_id TEXT, -- user_id for humans
    reviewer_agent_id BLOB REFERENCES agents(id) ON DELETE SET NULL,
    reviewer_name TEXT,

    review_type TEXT NOT NULL CHECK (review_type IN (
        'approval', 'feedback', 'revision_request', 'quality_check', 'compliance_check'
    )),
    status TEXT NOT NULL CHECK (status IN (
        'pending', 'approved', 'rejected', 'revision_requested', 'acknowledged'
    )),

    -- Review content
    feedback_text TEXT,
    rating INTEGER CHECK (rating IS NULL OR (rating >= 1 AND rating <= 5)),

    -- For revision requests
    revision_notes TEXT, -- JSON: structured feedback per section
    revision_deadline TEXT,

    -- Tracking
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    resolved_at TEXT,
    resolved_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_artifact_reviews_artifact ON artifact_reviews(artifact_id);
CREATE INDEX IF NOT EXISTS idx_artifact_reviews_status
    ON artifact_reviews(status) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_artifact_reviews_reviewer
    ON artifact_reviews(reviewer_id);

-- ============================================================================
-- PART 5: Direct Task-Artifact Linking
-- ============================================================================

CREATE TABLE IF NOT EXISTS task_artifacts (
    task_id BLOB NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    artifact_id BLOB NOT NULL REFERENCES execution_artifacts(id) ON DELETE CASCADE,

    artifact_role TEXT NOT NULL DEFAULT 'supporting' CHECK (artifact_role IN (
        'primary', 'supporting', 'verification', 'reference'
    )),
    display_order INTEGER DEFAULT 0,
    pinned INTEGER NOT NULL DEFAULT 0,

    added_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    added_by TEXT,

    PRIMARY KEY (task_id, artifact_id)
);

CREATE INDEX IF NOT EXISTS idx_task_artifacts_task ON task_artifacts(task_id);
CREATE INDEX IF NOT EXISTS idx_task_artifacts_artifact ON task_artifacts(artifact_id);
CREATE INDEX IF NOT EXISTS idx_task_artifacts_pinned
    ON task_artifacts(task_id, pinned) WHERE pinned = 1;

-- ============================================================================
-- PART 6: Extended Board Types
-- ============================================================================

-- Add new board types to project_boards
-- Note: SQLite CHECK constraints can't be easily modified, so we add a trigger

CREATE TRIGGER IF NOT EXISTS validate_board_type_extended
BEFORE INSERT ON project_boards
FOR EACH ROW
WHEN NEW.board_type NOT IN (
    'executive_assets', 'brand_assets', 'dev_assets', 'social_assets', 'custom',
    -- New board types
    'agent_flows', 'artifact_gallery', 'approval_queue', 'research_hub'
)
BEGIN
    SELECT RAISE(ABORT, 'Invalid board type');
END;

CREATE TRIGGER IF NOT EXISTS validate_board_type_update_extended
BEFORE UPDATE OF board_type ON project_boards
FOR EACH ROW
WHEN NEW.board_type NOT IN (
    'executive_assets', 'brand_assets', 'dev_assets', 'social_assets', 'custom',
    -- New board types
    'agent_flows', 'artifact_gallery', 'approval_queue', 'research_hub'
)
BEGIN
    SELECT RAISE(ABORT, 'Invalid board type');
END;

-- ============================================================================
-- PART 7: Agent Flow Events (for SSE/WebSocket streaming)
-- ============================================================================

CREATE TABLE IF NOT EXISTS agent_flow_events (
    id BLOB PRIMARY KEY,
    agent_flow_id BLOB NOT NULL REFERENCES agent_flows(id) ON DELETE CASCADE,

    event_type TEXT NOT NULL CHECK (event_type IN (
        'phase_started', 'phase_completed', 'artifact_created', 'artifact_updated',
        'approval_requested', 'approval_decision', 'wide_research_started',
        'subagent_progress', 'wide_research_completed', 'agent_handoff',
        'flow_paused', 'flow_resumed', 'flow_failed', 'flow_completed'
    )),

    event_data TEXT NOT NULL, -- JSON payload

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_agent_flow_events_flow
    ON agent_flow_events(agent_flow_id);
CREATE INDEX IF NOT EXISTS idx_agent_flow_events_type
    ON agent_flow_events(event_type);
CREATE INDEX IF NOT EXISTS idx_agent_flow_events_created
    ON agent_flow_events(created_at);

-- ============================================================================
-- PART 8: Extend Browser Sessions (add agent_flow_id for platform interaction recording)
-- ============================================================================

-- Add new columns to existing browser_sessions table (created in 20251230200000)
ALTER TABLE browser_sessions ADD COLUMN agent_flow_id BLOB REFERENCES agent_flows(id) ON DELETE SET NULL;
ALTER TABLE browser_sessions ADD COLUMN session_type TEXT CHECK (session_type IN (
    'verification', 'research', 'engagement', 'posting', 'testing', 'browser'
));
ALTER TABLE browser_sessions ADD COLUMN platform TEXT;
ALTER TABLE browser_sessions ADD COLUMN target_url TEXT;
ALTER TABLE browser_sessions ADD COLUMN recording_path TEXT;
ALTER TABLE browser_sessions ADD COLUMN recording_duration_ms INTEGER;
ALTER TABLE browser_sessions ADD COLUMN screenshots_count INTEGER DEFAULT 0;
ALTER TABLE browser_sessions ADD COLUMN success INTEGER DEFAULT 0;
ALTER TABLE browser_sessions ADD COLUMN action_log TEXT;
ALTER TABLE browser_sessions ADD COLUMN completed_at TEXT;

CREATE INDEX IF NOT EXISTS idx_browser_sessions_flow ON browser_sessions(agent_flow_id);
CREATE INDEX IF NOT EXISTS idx_browser_sessions_platform ON browser_sessions(platform);
