-- Migration: Entity System and Conference Workflow Engine
-- Implements reusable entities (speakers, brands, venues) with knowledge graph
-- Adds conference workflow orchestration and QA tracking

-- ============================================================================
-- PART 1: REUSABLE ENTITIES
-- Entities that can be shared across multiple conferences (speakers, brands, etc.)
-- ============================================================================

CREATE TABLE IF NOT EXISTS entities (
    id BLOB PRIMARY KEY,

    -- Entity classification
    entity_type TEXT NOT NULL CHECK (entity_type IN (
        'speaker', 'brand', 'sponsor', 'venue', 'production_company', 'organizer'
    )),

    -- Identity
    canonical_name TEXT NOT NULL,
    slug TEXT NOT NULL UNIQUE,

    -- External identifiers (JSON: { linkedin, twitter, website, etc. })
    external_ids TEXT,

    -- Profile information
    bio TEXT,
    title TEXT,
    company TEXT,
    photo_url TEXT,

    -- Social media profiles (JSON: [{ platform, handle, followers, url }])
    social_profiles TEXT,

    -- Cached social analysis (JSON: engagement metrics, posting patterns, etc.)
    social_analysis TEXT,

    -- Data quality indicator (0.0 to 1.0)
    data_completeness REAL DEFAULT 0.0,

    -- Last research timestamp
    last_researched_at TEXT,

    -- Metadata
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type);
CREATE INDEX IF NOT EXISTS idx_entities_slug ON entities(slug);
CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(canonical_name);
CREATE INDEX IF NOT EXISTS idx_entities_company ON entities(company);
CREATE INDEX IF NOT EXISTS idx_entities_completeness ON entities(data_completeness);

-- ============================================================================
-- PART 2: ENTITY APPEARANCES
-- Track when entities appear at specific conferences
-- ============================================================================

CREATE TABLE IF NOT EXISTS entity_appearances (
    id BLOB PRIMARY KEY,
    entity_id BLOB NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    conference_board_id BLOB NOT NULL REFERENCES project_boards(id) ON DELETE CASCADE,

    -- Appearance details
    appearance_type TEXT NOT NULL CHECK (appearance_type IN (
        'speaker', 'sponsor', 'exhibitor', 'organizer', 'panelist', 'keynote', 'workshop_leader'
    )),

    -- Speaker-specific
    talk_title TEXT,
    talk_description TEXT,
    talk_slot TEXT,  -- e.g., "Day 1, 2:00 PM - Main Stage"

    -- Associated research artifact (from artifact system)
    research_artifact_id BLOB,

    -- Status tracking
    status TEXT DEFAULT 'discovered' CHECK (status IN (
        'discovered', 'researching', 'researched', 'verified', 'content_created'
    )),

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(entity_id, conference_board_id, appearance_type)
);

CREATE INDEX IF NOT EXISTS idx_entity_appearances_entity ON entity_appearances(entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_appearances_board ON entity_appearances(conference_board_id);
CREATE INDEX IF NOT EXISTS idx_entity_appearances_status ON entity_appearances(status);
CREATE INDEX IF NOT EXISTS idx_entity_appearances_type ON entity_appearances(appearance_type);

-- ============================================================================
-- PART 3: CONFERENCE WORKFLOW ORCHESTRATION
-- Main workflow state for automated conference pipelines
-- ============================================================================

CREATE TABLE IF NOT EXISTS conference_workflows (
    id BLOB PRIMARY KEY,
    conference_board_id BLOB NOT NULL REFERENCES project_boards(id) ON DELETE CASCADE,

    -- Conference details
    conference_name TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    location TEXT,
    timezone TEXT,
    website TEXT,

    -- Workflow state
    status TEXT DEFAULT 'intake' CHECK (status IN (
        'intake',           -- Initial intake received
        'researching',      -- Research flow in progress
        'research_complete', -- Research done, awaiting content
        'content_creation', -- Content being created
        'graphics_creation', -- Graphics being generated
        'review',           -- Awaiting QA/review
        'scheduling',       -- Scheduling social posts
        'active',           -- Live coverage period
        'post_event',       -- Post-event content phase
        'completed',        -- All deliverables done
        'paused',           -- Manually paused
        'failed'            -- Error state
    )),

    -- Current stage tracking
    current_stage TEXT,
    current_stage_started_at TEXT,

    -- Sub-workflow IDs (execution process IDs)
    research_flow_id BLOB,
    content_flow_id BLOB,
    graphics_flow_id BLOB,

    -- Quality metrics
    last_qa_score REAL,
    last_qa_run_id BLOB,

    -- Statistics
    speakers_count INTEGER DEFAULT 0,
    sponsors_count INTEGER DEFAULT 0,
    side_events_count INTEGER DEFAULT 0,
    social_posts_scheduled INTEGER DEFAULT 0,
    social_posts_published INTEGER DEFAULT 0,

    -- Target platforms for social posts (JSON array of platform IDs)
    target_platform_ids TEXT,

    -- Configuration overrides (JSON)
    config_overrides TEXT,

    -- Error tracking
    last_error TEXT,
    retry_count INTEGER DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    completed_at TEXT,

    UNIQUE(conference_board_id)
);

CREATE INDEX IF NOT EXISTS idx_conference_workflows_status ON conference_workflows(status);
CREATE INDEX IF NOT EXISTS idx_conference_workflows_board ON conference_workflows(conference_board_id);
CREATE INDEX IF NOT EXISTS idx_conference_workflows_dates ON conference_workflows(start_date, end_date);

-- ============================================================================
-- PART 4: WORKFLOW QA RUNS
-- Audit trail for quality assurance checks
-- ============================================================================

CREATE TABLE IF NOT EXISTS workflow_qa_runs (
    id BLOB PRIMARY KEY,
    workflow_id BLOB NOT NULL REFERENCES conference_workflows(id) ON DELETE CASCADE,

    -- What was evaluated
    stage_name TEXT NOT NULL,
    artifact_id BLOB,  -- Optional reference to specific artifact

    -- Evaluation results (JSON: [{ item, passed, notes }])
    checklist_items TEXT NOT NULL,

    -- Confidence assessment
    confidence_level TEXT NOT NULL CHECK (confidence_level IN ('high', 'medium', 'low')),
    overall_score REAL NOT NULL,  -- 0.0 to 1.0

    -- Decision
    decision TEXT NOT NULL CHECK (decision IN ('approve', 'retry', 'escalate')),
    retry_guidance TEXT,  -- If retry, what to fix
    escalation_reason TEXT,  -- If escalate, why

    -- Agent that performed QA
    agent_id BLOB REFERENCES agents(id),

    -- Execution tracking
    execution_time_ms INTEGER,
    tokens_used INTEGER,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_qa_runs_workflow ON workflow_qa_runs(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_qa_runs_stage ON workflow_qa_runs(stage_name);
CREATE INDEX IF NOT EXISTS idx_workflow_qa_runs_decision ON workflow_qa_runs(decision);
CREATE INDEX IF NOT EXISTS idx_workflow_qa_runs_created ON workflow_qa_runs(created_at);

-- ============================================================================
-- PART 5: SIDE EVENTS
-- Discovered side events from Lu.ma, Eventbrite, Partiful, etc.
-- ============================================================================

CREATE TABLE IF NOT EXISTS side_events (
    id BLOB PRIMARY KEY,
    conference_workflow_id BLOB NOT NULL REFERENCES conference_workflows(id) ON DELETE CASCADE,

    -- Source platform
    platform TEXT CHECK (platform IN ('luma', 'eventbrite', 'partiful', 'meetup', 'manual', 'other')),
    platform_event_id TEXT,  -- ID on the source platform

    -- Event details
    name TEXT NOT NULL,
    description TEXT,
    event_date TEXT,
    start_time TEXT,
    end_time TEXT,

    -- Venue
    venue_name TEXT,
    venue_address TEXT,
    latitude REAL,
    longitude REAL,

    -- Links
    event_url TEXT,
    registration_url TEXT,

    -- Organizer
    organizer_name TEXT,
    organizer_url TEXT,

    -- Relevance scoring (0.0 to 1.0)
    relevance_score REAL,
    relevance_reason TEXT,

    -- Capacity/attendance
    capacity INTEGER,
    registered_count INTEGER,

    -- Content flags
    is_featured INTEGER DEFAULT 0,  -- Should this be highlighted?
    requires_registration INTEGER DEFAULT 1,
    is_free INTEGER DEFAULT 0,
    price_info TEXT,

    -- Status
    status TEXT DEFAULT 'discovered' CHECK (status IN (
        'discovered', 'validated', 'included', 'excluded', 'attended'
    )),

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(conference_workflow_id, platform, platform_event_id)
);

CREATE INDEX IF NOT EXISTS idx_side_events_workflow ON side_events(conference_workflow_id);
CREATE INDEX IF NOT EXISTS idx_side_events_platform ON side_events(platform);
CREATE INDEX IF NOT EXISTS idx_side_events_date ON side_events(event_date);
CREATE INDEX IF NOT EXISTS idx_side_events_relevance ON side_events(relevance_score);
CREATE INDEX IF NOT EXISTS idx_side_events_status ON side_events(status);

-- ============================================================================
-- PART 6: RESEARCH STAGE RESULTS
-- Track results from each research stage
-- ============================================================================

CREATE TABLE IF NOT EXISTS research_stage_results (
    id BLOB PRIMARY KEY,
    workflow_id BLOB NOT NULL REFERENCES conference_workflows(id) ON DELETE CASCADE,

    -- Stage identification
    stage_name TEXT NOT NULL CHECK (stage_name IN (
        'conference_intel', 'speaker_research', 'brand_research',
        'production_team', 'competitive_intel', 'side_events'
    )),
    stage_order INTEGER NOT NULL,

    -- Execution details
    started_at TEXT NOT NULL,
    completed_at TEXT,
    execution_time_ms INTEGER,

    -- Status
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending', 'running', 'completed', 'failed', 'skipped'
    )),

    -- Results summary (JSON: stage-specific output)
    results_summary TEXT,

    -- Artifacts created (JSON array of artifact IDs)
    artifact_ids TEXT,

    -- Items processed
    items_discovered INTEGER DEFAULT 0,
    items_processed INTEGER DEFAULT 0,
    items_failed INTEGER DEFAULT 0,

    -- QA result
    qa_run_id BLOB REFERENCES workflow_qa_runs(id),
    qa_passed INTEGER,

    -- Error tracking
    error_message TEXT,
    retry_count INTEGER DEFAULT 0,

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(workflow_id, stage_name)
);

CREATE INDEX IF NOT EXISTS idx_research_stage_results_workflow ON research_stage_results(workflow_id);
CREATE INDEX IF NOT EXISTS idx_research_stage_results_stage ON research_stage_results(stage_name);
CREATE INDEX IF NOT EXISTS idx_research_stage_results_status ON research_stage_results(status);

-- ============================================================================
-- PART 7: WORKFLOW EVENT LOG
-- Detailed event log for workflow debugging and monitoring
-- ============================================================================

CREATE TABLE IF NOT EXISTS workflow_events (
    id BLOB PRIMARY KEY,
    workflow_id BLOB NOT NULL REFERENCES conference_workflows(id) ON DELETE CASCADE,

    -- Event details
    event_type TEXT NOT NULL CHECK (event_type IN (
        'workflow_started', 'workflow_completed', 'workflow_failed', 'workflow_paused',
        'stage_started', 'stage_completed', 'stage_failed', 'stage_retried',
        'entity_discovered', 'entity_researched', 'entity_linked',
        'qa_started', 'qa_passed', 'qa_failed', 'qa_escalated',
        'content_created', 'graphics_created',
        'social_scheduled', 'social_published',
        'error', 'warning', 'info'
    )),

    -- Event context
    stage_name TEXT,
    entity_id BLOB,
    artifact_id BLOB,

    -- Event data (JSON: event-specific payload)
    event_data TEXT,

    -- Message for logging
    message TEXT NOT NULL,

    -- Severity for filtering
    severity TEXT DEFAULT 'info' CHECK (severity IN ('debug', 'info', 'warning', 'error')),

    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec'))
);

CREATE INDEX IF NOT EXISTS idx_workflow_events_workflow ON workflow_events(workflow_id);
CREATE INDEX IF NOT EXISTS idx_workflow_events_type ON workflow_events(event_type);
CREATE INDEX IF NOT EXISTS idx_workflow_events_created ON workflow_events(created_at);
CREATE INDEX IF NOT EXISTS idx_workflow_events_severity ON workflow_events(severity);

-- ============================================================================
-- PART 8: ENTITY ALIASES
-- Alternative names for entities (for deduplication)
-- ============================================================================

CREATE TABLE IF NOT EXISTS entity_aliases (
    id BLOB PRIMARY KEY,
    entity_id BLOB NOT NULL REFERENCES entities(id) ON DELETE CASCADE,
    alias TEXT NOT NULL,
    alias_type TEXT DEFAULT 'nickname' CHECK (alias_type IN (
        'nickname', 'maiden_name', 'stage_name', 'former_name', 'abbreviation', 'typo'
    )),
    created_at TEXT NOT NULL DEFAULT (datetime('now', 'subsec')),

    UNIQUE(alias)
);

CREATE INDEX IF NOT EXISTS idx_entity_aliases_entity ON entity_aliases(entity_id);
CREATE INDEX IF NOT EXISTS idx_entity_aliases_alias ON entity_aliases(alias);

-- ============================================================================
-- PART 9: ADD WORKFLOW REFERENCE TO SOCIAL POSTS
-- ============================================================================

ALTER TABLE social_posts ADD COLUMN conference_workflow_id BLOB REFERENCES conference_workflows(id) ON DELETE SET NULL;
ALTER TABLE social_posts ADD COLUMN entity_id BLOB REFERENCES entities(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_social_posts_workflow ON social_posts(conference_workflow_id);
CREATE INDEX IF NOT EXISTS idx_social_posts_entity ON social_posts(entity_id);
